//! Local browser boundary, not authentication against local processes.
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, ()> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(());
    }
    first
        .map(|value| value.to_str().map_err(|_| ()))
        .transpose()
}

fn local_authority(authority: &str, port: u16) -> bool {
    authority.eq_ignore_ascii_case(&format!("127.0.0.1:{port}"))
        || authority.eq_ignore_ascii_case(&format!("localhost:{port}"))
        || (port == 80 && (authority.eq_ignore_ascii_case("localhost") || authority == "127.0.0.1"))
}

fn allowed(request: &Request, port: u16) -> bool {
    let Ok(Some(host)) = single_header(request.headers(), "host") else {
        return false;
    };
    if !local_authority(host, port) {
        return false;
    }
    if let Some(authority) = request.uri().authority() {
        if !authority.as_str().eq_ignore_ascii_case(host) {
            return false;
        }
    }
    if request
        .uri()
        .scheme_str()
        .is_some_and(|scheme| scheme != "http")
    {
        return false;
    }
    // A browser may send Sec-Fetch-Site: cross-site on a local Tauri form
    // submission. Such a submission is safe only if its browser-controlled
    // Origin independently matches this exact loopback host and port.
    let mut matched_origin = false;
    match single_header(request.headers(), "origin") {
        Ok(None) => {} // CLI clients and ordinary same-origin navigation may omit Origin.
        Ok(Some(origin)) => {
            let Some(authority) = origin.strip_prefix("http://") else {
                return false;
            };
            // Same host AND port, not simply any loopback address.
            let normalized = if port == 80 {
                authority.strip_suffix(":80").unwrap_or(authority)
            } else {
                authority
            };
            let normalized_host = if port == 80 {
                host.strip_suffix(":80").unwrap_or(host)
            } else {
                host
            };
            if !local_authority(authority, port)
                || !normalized.eq_ignore_ascii_case(normalized_host)
            {
                return false;
            }
            matched_origin = true;
        }
        Err(()) => return false,
    }
    match single_header(request.headers(), "sec-fetch-site") {
        Ok(None | Some("same-origin" | "none")) => true,
        // Chromium/WebView2 can classify browser-internal top-level navigations
        // as cross-site. Only public HTML pages and authenticated-origin form
        // submissions receive an exception; no API, WS, or cross-site script.
        Ok(Some("cross-site")) => {
            let document_navigation = matches!(
                single_header(request.headers(), "sec-fetch-mode"),
                Ok(Some("navigate"))
            ) && matches!(
                single_header(request.headers(), "sec-fetch-dest"),
                Ok(Some("document"))
            );
            document_navigation
                && if request.method() == Method::GET {
                    matches!(request.uri().path(), "/" | "/login" | "/setup")
                        && matches!(single_header(request.headers(), "origin"), Ok(None))
                } else if request.method() == Method::POST {
                    matches!(request.uri().path(), "/login" | "/setup") && matched_origin
                } else {
                    false
                }
        }
        _ => false,
    }
}

pub(crate) async fn local_only(State(port): State<u16>, request: Request, next: Next) -> Response {
    let mut response = if allowed(&request, port) {
        next.run(request).await
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"request origin not allowed"})),
        )
            .into_response()
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get, Router};
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new()
            .route("/", get(|| async { "dashboard page" }))
            .route("/login", get(|| async { "login page" }))
            .route("/setup", get(|| async { "setup page" }))
            .route("/api/metrics", get(|| async { "local metrics" }))
            .route("/ws/ticks", get(|| async { "upgrade boundary" }))
            .layer(middleware::from_fn_with_state(8081_u16, local_only))
    }
    async fn send(
        host: Option<&str>,
        origin: Option<&str>,
        site: Option<&str>,
        path: &str,
    ) -> Response {
        let mut request = Request::builder().uri(path);
        if let Some(value) = host {
            request = request.header(header::HOST, value);
        }
        if let Some(value) = origin {
            request = request.header(header::ORIGIN, value);
        }
        if let Some(value) = site {
            request = request.header("sec-fetch-site", value);
        }
        app()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn accepts_same_origin_and_local_cli_on_configured_port() {
        for host in ["127.0.0.1:8081", "localhost:8081"] {
            let origin = format!("http://{host}");
            assert_eq!(
                send(
                    Some(host),
                    Some(&origin),
                    Some("same-origin"),
                    "/api/metrics"
                )
                .await
                .status(),
                StatusCode::OK
            );
            assert_eq!(
                send(Some(host), None, None, "/api/metrics").await.status(),
                StatusCode::OK
            );
        }
    }
    #[tokio::test]
    async fn permits_only_cross_site_top_level_document_navigation() {
        // Matches the browser fetch metadata emitted by WebView2 on initial launch.
        for path in ["/", "/login", "/setup"] {
            let request = Request::builder()
                .method(Method::GET)
                .uri(path)
                .header(header::HOST, "127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document")
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::OK
            );
        }
        // Cross-site scripts, API reads, mutations and WebSocket handshakes
        // are still denied, even if they forge navigation metadata.
        for path in ["/api/metrics", "/ws/ticks", "/setup", "/logout"] {
            let request = Request::builder()
                .method(Method::GET)
                .uri(path)
                .header(header::HOST, "127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document")
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN
            );
        }
        for (method, path, origin, mode, dest) in [
            (Method::POST, "/login", None, "navigate", "document"),
            (Method::GET, "/login", Some("null"), "navigate", "document"),
            (Method::GET, "/login", None, "cors", "empty"),
            (Method::GET, "/login", None, "no-cors", "document"),
            (Method::GET, "/login", None, "navigate", "empty"),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(path)
                .header(header::HOST, "127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", mode)
                .header("sec-fetch-dest", dest);
            if let Some(value) = origin {
                request = request.header(header::ORIGIN, value);
            }
            assert_eq!(
                app()
                    .oneshot(request.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    #[tokio::test]
    async fn cross_site_setup_post_requires_exact_same_origin_and_document_navigation() {
        // A local WebView2 password form can carry cross-site Fetch Metadata.
        // The independent Origin header must still be the exact local origin.
        for path in ["/login", "/setup"] {
            let request = Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(header::HOST, "127.0.0.1:8081")
                .header(header::ORIGIN, "http://127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document")
                .body(Body::empty())
                .unwrap();
            // Router handles GET only; 405 confirms the security boundary passed.
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::METHOD_NOT_ALLOWED
            );
        }
        for origin in [
            None,
            Some("null"),
            Some("http://localhost:8081"),
            Some("http://127.0.0.1:8082"),
            Some("https://127.0.0.1:8081"),
            Some("https://attacker.invalid"),
        ] {
            let mut request = Request::builder()
                .method(Method::POST)
                .uri("/setup")
                .header(header::HOST, "127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document");
            if let Some(origin) = origin {
                request = request.header(header::ORIGIN, origin);
            }
            assert_eq!(
                app()
                    .oneshot(request.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
        for (mode, dest) in [("cors", "empty"), ("navigate", "empty"), ("no-cors", "document")] {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/setup")
                .header(header::HOST, "127.0.0.1:8081")
                .header(header::ORIGIN, "http://127.0.0.1:8081")
                .header("sec-fetch-site", "cross-site")
                .header("sec-fetch-mode", mode)
                .header("sec-fetch-dest", dest)
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN
            );
        }
    }

    #[tokio::test]
    async fn rejects_rebinding_cross_origin_and_wrong_port_for_http_and_ws() {
        for path in ["/api/metrics", "/ws/ticks"] {
            for host in [
                None,
                Some("attacker.example:8081"),
                Some("127.0.0.1:8080"),
                Some("localhost.attacker.example:8081"),
            ] {
                assert_eq!(
                    send(host, None, None, path).await.status(),
                    StatusCode::FORBIDDEN
                );
            }
            for origin in [
                "null",
                "https://localhost:8081",
                "http://localhost:8082",
                "http://attacker.example",
                "http://127.0.0.1:8081/",
            ] {
                assert_eq!(
                    send(Some("127.0.0.1:8081"), Some(origin), None, path)
                        .await
                        .status(),
                    StatusCode::FORBIDDEN
                );
            }
            assert_eq!(
                send(Some("127.0.0.1:8081"), None, Some("cross-site"), path)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                send(
                    Some("127.0.0.1:8081"),
                    Some("http://localhost:8081"),
                    None,
                    path
                )
                .await
                .status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    #[tokio::test]
    async fn rejects_duplicate_security_headers_and_absolute_target_mismatch() {
        for name in ["host", "origin", "sec-fetch-site"] {
            let mut request = Request::builder()
                .uri("/api/metrics")
                .header(header::HOST, "127.0.0.1:8081")
                .body(Body::empty())
                .unwrap();
            let value = match name {
                "origin" => "http://127.0.0.1:8081",
                "sec-fetch-site" => "same-origin",
                _ => "127.0.0.1:8081",
            };
            let header_name = axum::http::HeaderName::from_static(name);
            request
                .headers_mut()
                .append(header_name.clone(), HeaderValue::from_static(value));
            request
                .headers_mut()
                .append(header_name, HeaderValue::from_static(value));
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN
            );
        }
        assert_eq!(
            send(
                Some("127.0.0.1:8081"),
                None,
                None,
                "http://attacker.example/api/metrics"
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }
    #[tokio::test]
    async fn security_headers_cover_errors_and_success() {
        for host in ["127.0.0.1:8081", "bad.example"] {
            let response = send(Some(host), None, None, "/api/metrics").await;
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            assert_eq!(response.headers()["referrer-policy"], "no-referrer");
            assert!(!response
                .headers()
                .contains_key("access-control-allow-origin"));
        }
    }
}
