//! Transport-independent password storage, PBKDF2 validation, session lifecycle and throttling.
//! This module must never import Axum or Tauri.
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

pub const MIN_PASSWORD_CHARS: usize = 15;
const MAX_PASSWORD_BYTES: usize = 1024;
pub const PBKDF2_ITERATIONS: u32 = 600_000;
pub(super) const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);
pub(super) const FREE_ATTEMPTS: u32 = 5;
const BASE_LOCKOUT: Duration = Duration::from_secs(30);
const MAX_LOCKOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
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
pub(super) struct SessionToken(pub(super) String);

pub struct Auth {
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

pub(super) fn to_hex(bytes: &[u8]) -> String {
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

pub(super) fn random_bytes<const N: usize>() -> Result<[u8; N], AuthError> {
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
    pub fn load(path: impl Into<PathBuf>, iterations: u32) -> Result<Self, String> {
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

    pub fn is_configured(&self) -> bool {
        lock(&self.record).is_some()
    }

    /// Calcul PBKDF2 coûteux : à appeler depuis `spawn_blocking`.
    pub fn set_password(&self, password: &str) -> Result<(), AuthError> {
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
    pub fn login(&self, password: &str) -> Result<String, AuthError> {
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

    pub fn session_valid(&self, token: &str) -> bool {
        let mut sessions = lock(&self.sessions);
        let now = Instant::now();
        sessions.retain(|_, expires| *expires > now);
        sessions.contains_key(token)
    }

    pub fn logout(&self, token: &str) {
        lock(&self.sessions).remove(token);
        let _ = self.closed_tx.send(token.to_owned());
    }

    pub fn subscribe_closed(&self) -> broadcast::Receiver<String> {
        self.closed_tx.subscribe()
    }
}

impl SessionToken {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

