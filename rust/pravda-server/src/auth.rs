//! Mot de passe local : empreinte PBKDF2 salée, sessions en mémoire, limitation des tentatives.
//! Ne protège pas contre un processus local malveillant ou un administrateur de la machine.
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Form, Json, Router,
};
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    fmt::Write as _,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;

pub(crate) const MIN_PASSWORD_CHARS: usize = 15;
const MAX_PASSWORD_BYTES: usize = 1024;
pub(crate) const PBKDF2_ITERATIONS: u32 = 600_000;
const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);
const COOKIE_NAME: &str = "pravda_session";
const FREE_ATTEMPTS: u32 = 5;
const BASE_LOCKOUT: Duration = Duration::from_secs(30);
const MAX_LOCKOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AuthError {
    WeakPassword,
    Mismatch,
    Invalid,
    Locked(Duration),
    NotConfigured,
    AlreadyConfigured,
    Storage,
}

#[derive(Serialize, Deserialize, Clone)]
struct Record {
    version: u32,
    algorithm: String,
    iterations: u32,
    salt: String,
    hash: String,
}

#[derive(Default)]
struct Failures {
    count: u32,
    locked_until: Option<Instant>,
}

#[derive(Clone)]
pub(crate) struct SessionToken(String);

pub(crate) struct Auth {
    path: PathBuf,
    iterations: u32,
    record: Mutex<Option<Record>>,
    sessions: Mutex<HashMap<String, Instant>>,
    failures: Mutex<Failures>,
    closed_tx: broadcast::Sender<String>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

fn derive(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut out = [0_u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut out);
    out
}

fn random_bytes<const N: usize>() -> Result<[u8; N], AuthError> {
    let mut buffer = [0_u8; N];
    getrandom::getrandom(&mut buffer).map_err(|_| AuthError::Storage)?;
    Ok(buffer)
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.chars().count() < MIN_PASSWORD_CHARS || password.len() > MAX_PASSWORD_BYTES {
        return Err(AuthError::WeakPassword);
    }
    Ok(())
}

impl Auth {
    /// Un fichier existant mais illisible ou invalide fait échouer le démarrage (fermeture par défaut).
    pub(crate) fn load(path: impl Into<PathBuf>, iterations: u32) -> Result<Self, String> {
        let path = path.into();
        let record = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let record: Record = serde_json::from_str(&text)
                    .map_err(|_| "invalid authentication file".to_string())?;
                let valid = record.version == 1
                    && record.algorithm == "pbkdf2-sha256"
                    && record.iterations > 0
                    && from_hex(&record.salt).is_some_and(|salt| salt.len() >= 16)
                    && from_hex(&record.hash).is_some_and(|hash| hash.len() == 32);
                if !valid {
                    return Err("invalid authentication file".into());
                }
                Some(record)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err("authentication file unreadable".into()),
        };
        Ok(Self {
            path,
            iterations,
            record: Mutex::new(record),
            sessions: Mutex::new(HashMap::new()),
            failures: Mutex::new(Failures::default()),
            closed_tx: broadcast::channel(16).0,
        })
    }

    pub(crate) fn is_configured(&self) -> bool {
        lock(&self.record).is_some()
    }

    /// Calcul PBKDF2 coûteux : à appeler depuis `spawn_blocking`.
    pub(crate) fn set_password(&self, password: &str) -> Result<(), AuthError> {
        validate_password(password)?;
        let mut record = lock(&self.record);
        if record.is_some() {
            return Err(AuthError::AlreadyConfigured);
        }
        let salt = random_bytes::<16>()?;
        let new_record = Record {
            version: 1,
            algorithm: "pbkdf2-sha256".into(),
            iterations: self.iterations,
            salt: to_hex(&salt),
            hash: to_hex(&derive(password, &salt, self.iterations)),
        };
        let text = serde_json::to_string_pretty(&new_record).map_err(|_| AuthError::Storage)?;
        // A random sibling temporary file avoids collisions between processes.
        // hard_link creates the final path only if it does not already exist.
        // Unlike rename, it cannot replace another server's password record.
        let mut temporary = self.path.clone().into_os_string();
        temporary.push(format!(".{}.tmp", to_hex(&random_bytes::<16>()?)));
        let temporary = PathBuf::from(temporary);
        let result = (|| -> Result<(), AuthError> {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| AuthError::Storage)?;
            file.write_all(text.as_bytes())
                .map_err(|_| AuthError::Storage)?;
            file.sync_all().map_err(|_| AuthError::Storage)?;
            std::fs::hard_link(&temporary, &self.path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    AuthError::AlreadyConfigured
                } else {
                    AuthError::Storage
                }
            })
        })();
        let _ = std::fs::remove_file(&temporary);
        result?;
        *record = Some(new_record);
        Ok(())
    }

    /// Calcul PBKDF2 coûteux : à appeler depuis `spawn_blocking`. Les essais sont sérialisés.
    pub(crate) fn login(&self, password: &str) -> Result<String, AuthError> {
        let mut failures = lock(&self.failures);
        let now = Instant::now();
        if let Some(until) = failures.locked_until {
            if until > now {
                return Err(AuthError::Locked(until - now));
            }
        }
        let record = lock(&self.record).clone().ok_or(AuthError::NotConfigured)?;
        let salt = from_hex(&record.salt).ok_or(AuthError::Storage)?;
        let expected = from_hex(&record.hash).ok_or(AuthError::Storage)?;
        let candidate = if password.len() <= MAX_PASSWORD_BYTES {
            derive(password, &salt, record.iterations)
        } else {
            [0_u8; 32]
        };
        let matches =
            password.len() <= MAX_PASSWORD_BYTES && bool::from(candidate.ct_eq(&expected[..]));
        if !matches {
            failures.count = failures.count.saturating_add(1);
            if failures.count >= FREE_ATTEMPTS {
                let exponent = (failures.count - FREE_ATTEMPTS).min(10);
                let delay = (BASE_LOCKOUT * 2_u32.pow(exponent)).min(MAX_LOCKOUT);
                failures.locked_until = Some(Instant::now() + delay);
            }
            return Err(AuthError::Invalid);
        }
        *failures = Failures::default();
        drop(failures);
        let token = to_hex(&random_bytes::<32>()?);
        lock(&self.sessions).insert(token.clone(), Instant::now() + SESSION_TTL);
        Ok(token)
    }

    pub(crate) fn session_valid(&self, token: &str) -> bool {
        let mut sessions = lock(&self.sessions);
        let now = Instant::now();
        sessions.retain(|_, expires| *expires > now);
        sessions.contains_key(token)
    }

    pub(crate) fn logout(&self, token: &str) {
        lock(&self.sessions).remove(token);
        let _ = self.closed_tx.send(token.to_owned());
    }

    pub(crate) fn subscribe_closed(&self) -> broadcast::Receiver<String> {
        self.closed_tx.subscribe()
    }
}

impl SessionToken {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|part| {
            part.trim()
                .strip_prefix(COOKIE_NAME)
                .and_then(|rest| rest.strip_prefix('='))
                .map(str::to_owned)
        })
}

fn session_cookie(token: &str, max_age: u64) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age}"
    ))
    .expect("hex token and numeric attributes are valid header bytes")
}

pub(crate) async fn require_auth(
    State(auth): State<Arc<Auth>>,
    mut request: Request,
    next: Next,
) -> Response {
    let public = matches!(
        (request.method().as_str(), request.uri().path()),
        ("GET", "/login") | ("POST", "/login") | ("POST", "/setup")
    );
    if public {
        return next.run(request).await;
    }
    if let Some(token) = cookie_token(request.headers()) {
        if auth.session_valid(&token) {
            request.extensions_mut().insert(SessionToken(token));
            return next.run(request).await;
        }
    }
    if request.uri().path() == "/" {
        return Redirect::to("/login").into_response();
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "authentication required"})),
    )
        .into_response()
}

pub(crate) fn routes<S: Clone + Send + Sync + 'static>(auth: Arc<Auth>) -> Router<S> {
    Router::new()
        .route("/login", get(login_page).post(login_submit))
        .route("/setup", post(setup_submit))
        .route("/logout", post(logout))
        .with_state(auth)
}

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

#[derive(Deserialize)]
struct SetupForm {
    password: String,
    confirm: String,
}

fn page(configured: bool, status: StatusCode, message: &str) -> Response {
    let notice = if message.is_empty() {
        String::new()
    } else {
        format!("<p class=\"error\" role=\"alert\">{message}</p>")
    };
    let form = if configured {
        "<h1>Pravda-Bot</h1><p>Entrez le mot de passe local.</p>\
         <form method=\"post\" action=\"/login\">\
         <label for=\"login-password\">Mot de passe</label><input id=\"login-password\" type=\"password\" name=\"password\" autocomplete=\"current-password\" autofocus required>\
         <button type=\"submit\">Se connecter</button></form>"
            .to_string()
    } else {
        format!(
            "<h1>Pravda-Bot</h1><p>Première utilisation : définissez un mot de passe local d'au moins {MIN_PASSWORD_CHARS} caractères.</p>\
             <form method=\"post\" action=\"/setup\">\
             <label for=\"setup-password\">Nouveau mot de passe</label><input id=\"setup-password\" type=\"password\" name=\"password\" autocomplete=\"new-password\" minlength=\"{MIN_PASSWORD_CHARS}\" autofocus required>\
             <label for=\"setup-confirm\">Confirmer le mot de passe</label><input id=\"setup-confirm\" type=\"password\" name=\"confirm\" autocomplete=\"new-password\" minlength=\"{MIN_PASSWORD_CHARS}\" required>\
             <button type=\"submit\">Enregistrer</button></form>"
        )
    };
    let body = format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Pravda-Bot</title>\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <style>body{{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;display:grid;place-items:center;min-height:100vh;margin:0}}\
         main{{background:#1e293b;padding:2rem;border-radius:12px;width:min(22rem,90vw)}}\
         input,button{{width:100%;box-sizing:border-box;margin-top:.75rem;padding:.6rem;border-radius:6px;border:1px solid #475569}}\
         button{{background:#2563eb;color:#fff;border:0;cursor:pointer}}.error{{color:#fca5a5}}</style></head>\
         <body><main>{form}{notice}</main></body></html>"
    );
    let mut response = (status, body).into_response();
    let headers = response.headers_mut();
    // The desktop launcher checks this marker before opening its WebView.
    headers.insert("x-pravda-server", HeaderValue::from_static("1"));
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
        ),
    );
    response
}

fn error_page(configured: bool, error: &AuthError) -> Response {
    let (status, message) = match error {
        AuthError::Invalid => (StatusCode::UNAUTHORIZED, "Mot de passe incorrect."),
        AuthError::Locked(_) => (
            StatusCode::TOO_MANY_REQUESTS,
            "Trop de tentatives. Réessayez plus tard.",
        ),
        AuthError::NotConfigured => (StatusCode::CONFLICT, "Aucun mot de passe n'est défini."),
        AuthError::AlreadyConfigured => (StatusCode::CONFLICT, "Un mot de passe est déjà défini."),
        AuthError::WeakPassword => (
            StatusCode::BAD_REQUEST,
            "Le mot de passe doit contenir au moins 15 caractères.",
        ),
        AuthError::Mismatch => (
            StatusCode::BAD_REQUEST,
            "Les mots de passe ne correspondent pas.",
        ),
        AuthError::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne."),
    };
    let mut response = page(configured, status, message);
    if let AuthError::Locked(remaining) = error {
        if let Ok(value) = HeaderValue::from_str(&remaining.as_secs().max(1).to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
    }
    response
}

fn logged_in(token: &str) -> Response {
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(token, SESSION_TTL.as_secs()),
    );
    response
}

async fn login_page(State(auth): State<Arc<Auth>>) -> Response {
    page(auth.is_configured(), StatusCode::OK, "")
}

async fn login_submit(State(auth): State<Arc<Auth>>, Form(form): Form<LoginForm>) -> Response {
    let worker = auth.clone();
    let result = tokio::task::spawn_blocking(move || worker.login(&form.password))
        .await
        .unwrap_or(Err(AuthError::Storage));
    match result {
        Ok(token) => logged_in(&token),
        Err(error) => error_page(auth.is_configured(), &error),
    }
}

async fn setup_submit(State(auth): State<Arc<Auth>>, Form(form): Form<SetupForm>) -> Response {
    if form.password != form.confirm {
        return error_page(false, &AuthError::Mismatch);
    }
    let worker = auth.clone();
    let result = tokio::task::spawn_blocking(move || {
        worker.set_password(&form.password)?;
        worker.login(&form.password)
    })
    .await
    .unwrap_or(Err(AuthError::Storage));
    match result {
        Ok(token) => logged_in(&token),
        Err(error) => error_page(auth.is_configured(), &error),
    }
}

async fn logout(
    State(auth): State<Arc<Auth>>,
    Extension(token): Extension<SessionToken>,
) -> Response {
    auth.logout(token.as_str());
    let mut response = Redirect::to("/login").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie("", 0));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get as get_route};
    use tower::ServiceExt;

    // Test secrets are generated at runtime, never published as fixed values.
    fn test_password() -> String {
        to_hex(&random_bytes::<20>().expect("test randomness must be available"))
    }

    fn temp_path(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "pravda-auth-{label}-{}-{nonce}.json",
            std::process::id()
        ))
    }

    fn auth(label: &str) -> (Arc<Auth>, PathBuf) {
        let path = temp_path(label);
        (Arc::new(Auth::load(&path, 1_000).unwrap()), path)
    }

    #[test]
    fn short_passwords_are_rejected_and_hash_is_salted_and_not_plaintext() {
        let (auth, path) = auth("store");
        let password = test_password();
        assert_eq!(
            auth.set_password(&"x".repeat(MIN_PASSWORD_CHARS - 1)),
            Err(AuthError::WeakPassword)
        );
        assert!(!auth.is_configured());
        auth.set_password(password).unwrap();
        let stored = std::fs::read_to_string(&path).unwrap();
        assert!(!stored.contains(password));
        assert!(stored.contains("pbkdf2-sha256"));
        assert_eq!(
            auth.set_password(password),
            Err(AuthError::AlreadyConfigured)
        );
        let reloaded = Auth::load(&path, 1_000).unwrap();
        assert!(reloaded.login(password).is_ok());
        assert_eq!(
            reloaded.login(&format!("{password}-wrong")),
            Err(AuthError::Invalid)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn independent_servers_cannot_overwrite_existing_password_record() {
        let (first, path) = auth("race");
        let password = test_password();
        let second = Auth::load(&path, 1_000).unwrap();
        first.set_password(password).unwrap();
        assert_eq!(
            second.set_password(&format!("{password}-different")),
            Err(AuthError::AlreadyConfigured)
        );
        let restored = Auth::load(&path, 1_000).unwrap();
        assert!(restored.login(password).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupted_file_fails_closed() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{not json").unwrap();
        assert!(Auth::load(&path, 1_000).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn repeated_failures_lock_out_even_the_correct_password() {
        let (auth, path) = auth("lock");
        let password = test_password();
        auth.set_password(password).unwrap();
        for _ in 0..FREE_ATTEMPTS {
            assert_eq!(auth.login(&format!("{password}-wrong")), Err(AuthError::Invalid));
        }
        assert!(matches!(auth.login(password), Err(AuthError::Locked(_))));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn logout_invalidates_session_and_notifies_subscribers() {
        let (auth, path) = auth("logout");
        let password = test_password();
        auth.set_password(password).unwrap();
        let token = auth.login(password).unwrap();
        let mut closed = auth.subscribe_closed();
        assert!(auth.session_valid(&token));
        auth.logout(&token);
        assert!(!auth.session_valid(&token));
        assert_eq!(closed.try_recv().unwrap(), token);
        let _ = std::fs::remove_file(path);
    }

    fn app(auth: Arc<Auth>) -> Router {
        Router::new()
            .route("/", get_route(|| async { "dashboard" }))
            .route("/api/metrics", get_route(|| async { "metrics" }))
            .merge(routes(auth.clone()))
            .layer(middleware::from_fn_with_state(auth, require_auth))
    }

    async fn call(
        app: &Router,
        method: &str,
        path: &str,
        cookie: Option<&str>,
        body: &str,
    ) -> Response {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        if !body.is_empty() {
            request = request.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        }
        app.clone()
            .oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn dashboard_and_api_require_login_then_logout_revokes_access() {
        let (auth, path) = auth("http");
        let password = test_password();
        let app = app(auth.clone());

        let response = call(&app, "GET", "/api/metrics", None, "").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = call(&app, "GET", "/", None, "").await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/login");
        let forged = format!("{COOKIE_NAME}=forged");
        let response = call(&app, "GET", "/api/metrics", Some(&forged), "").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            call(&app, "GET", "/login", None, "").await.status(),
            StatusCode::OK
        );

        let short = "x".repeat(MIN_PASSWORD_CHARS - 1);
        let weak = call(&app, "POST", "/setup", None, &format!("password={short}&confirm={short}")).await;
        assert_eq!(weak.status(), StatusCode::BAD_REQUEST);
        let body = format!("password={password}&confirm={password}");
        let response = call(&app, "POST", "/setup", None, &body).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let again = call(&app, "POST", "/setup", None, &body).await;
        assert_eq!(again.status(), StatusCode::CONFLICT);

        let bad = call(
            &app,
            "POST",
            "/login",
            None,
            &format!("password={password}-wrong"),
        )
        .await;
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        let response = call(
            &app,
            "POST",
            "/login",
            None,
            &format!("password={password}"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let set_cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_owned();
        assert!(set_cookie.contains("HttpOnly") && set_cookie.contains("SameSite=Strict"));
        let cookie = set_cookie.split(';').next().unwrap().to_owned();

        assert_eq!(
            call(&app, "GET", "/api/metrics", Some(&cookie), "")
                .await
                .status(),
            StatusCode::OK
        );
        let response = call(&app, "POST", "/logout", Some(&cookie), "").await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            call(&app, "GET", "/api/metrics", Some(&cookie), "")
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let _ = std::fs::remove_file(path);
    }
}
