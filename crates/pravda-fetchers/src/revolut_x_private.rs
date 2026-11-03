//! Authenticated Revolut X read-only balances and offline signed order previews.
//!
//! Ed25519 signing is local. There is no order creation/cancellation HTTP
//! transport or arbitrary HTTP method/path input.
//! API keys and PKCS#8 material are supplied by Rust callers and never logged.
//! See https://developer.revolut.com/docs/api/revolut-x-crypto-exchange
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Response, StatusCode,
};
use ring::signature::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use zeroize::Zeroizing;

const BALANCES_URL: &str = "https://revx.revolut.com/api/1.0/balances";
const BALANCES_PATH: &str = "/api/1.0/balances";
const MAX_BODY_BYTES: usize = 128 * 1024;
const MAX_BALANCES: usize = 500;
const PEM_START: &str = concat!("-----BEGIN ", "PRIVATE KEY-----");
const PEM_END: &str = concat!("-----END ", "PRIVATE KEY-----");

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrivateReadError {
    #[error("Revolut X credentials are absent or malformed")]
    Credentials,
    #[error("Revolut X cryptographic signing failed")]
    Signing,
    #[error("system clock is unavailable")]
    Clock,
    #[error("Revolut X network request failed")]
    Network,
    #[error("Revolut X request was not authorised")]
    Unauthorized,
    #[error("Revolut X rate limit exceeded")]
    RateLimited,
    #[error("Revolut X returned an unsuccessful response")]
    HttpStatus,
    #[error("Revolut X response is too large or invalid")]
    InvalidResponse,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Balance {
    pub currency: String,
    pub available: String,
    pub reserved: String,
    pub total: String,
    #[serde(default)]
    pub staked: Option<String>,
}

fn valid_decimal(input: &str) -> bool {
    if input.is_empty() || input.len() > 64 || input.starts_with('.') || input.ends_with('.') {
        return false;
    }
    let mut dot = false;
    let mut digit = false;
    for byte in input.bytes() {
        match byte {
            b'0'..=b'9' => digit = true,
            b'.' if !dot => dot = true,
            _ => return false,
        }
    }
    digit
}

fn validate_balances(balances: &[Balance]) -> Result<(), PrivateReadError> {
    if balances.len() > MAX_BALANCES {
        return Err(PrivateReadError::InvalidResponse);
    }
    for row in balances {
        if row.currency.is_empty()
            || row.currency.len() > 16
            || !row
                .currency
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            || !valid_decimal(&row.available)
            || !valid_decimal(&row.reserved)
            || !valid_decimal(&row.total)
            || row
                .staked
                .as_deref()
                .is_some_and(|value| !valid_decimal(value))
        {
            return Err(PrivateReadError::InvalidResponse);
        }
    }
    Ok(())
}

/// Revolut's official OpenSSL guidance yields a PEM PKCS#8 v1 Ed25519 key.
/// This parser never logs or persists the decoded plaintext key.
fn parse_ed25519_pem(secret: &[u8]) -> Result<Zeroizing<Vec<u8>>, PrivateReadError> {
    let text = std::str::from_utf8(secret).map_err(|_| PrivateReadError::Credentials)?;
    let mut lines = text.trim().lines();
    if lines.next() != Some(PEM_START) {
        return Err(PrivateReadError::Credentials);
    }
    let mut payload = Zeroizing::new(String::with_capacity(2048));
    let mut closed = false;
    for line in &mut lines {
        if line == PEM_END {
            closed = true;
            break;
        }
        let part = line.trim();
        if part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
        {
            return Err(PrivateReadError::Credentials);
        }
        if part.len() > 2048_usize.saturating_sub(payload.len()) {
            return Err(PrivateReadError::Credentials);
        }
        payload.push_str(part);
    }
    if !closed || lines.next().is_some() {
        return Err(PrivateReadError::Credentials);
    }
    STANDARD
        .decode(payload.as_bytes())
        .map(Zeroizing::new)
        .map_err(|_| PrivateReadError::Credentials)
}

/// Revolut X signing preimage: timestamp + method + fixed path + minified JSON.
/// No separators and no network transport for order preview signing.
fn sign_balances(timestamp: &str, key_pem: &[u8]) -> Result<String, PrivateReadError> {
    sign_fixed_request(timestamp, key_pem, FixedOperation::Balances, "")
}

enum FixedOperation {
    Balances,
    LimitOrder,
    OrderDetails(String),
}

/// Sign only a trusted offline order preview; this does not send any order.
pub(crate) fn sign_prepared_limit_order(
    timestamp: &str,
    key_pem: &[u8],
    validated_json_body: &str,
) -> Result<String, PrivateReadError> {
    if validated_json_body.is_empty() || validated_json_body.len() > 4096 {
        return Err(PrivateReadError::InvalidResponse);
    }
    sign_fixed_request(
        timestamp,
        key_pem,
        FixedOperation::LimitOrder,
        validated_json_body,
    )
}

fn sign_fixed_request(
    timestamp: &str,
    key_pem: &[u8],
    operation: FixedOperation,
    json_body: &str,
) -> Result<String, PrivateReadError> {
    if timestamp.is_empty() || !timestamp.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PrivateReadError::Clock);
    }
    let pair = {
        let der = parse_ed25519_pem(key_pem)?;
        Ed25519KeyPair::from_pkcs8_maybe_unchecked(&der).map_err(|_| PrivateReadError::Signing)?
    };
    let msg = match operation {
        FixedOperation::Balances => format!("{timestamp}GET{BALANCES_PATH}"),
        FixedOperation::LimitOrder => format!("{timestamp}POST/api/1.0/orders{json_body}"),
        FixedOperation::OrderDetails(venue_id) => {
            format!("{timestamp}GET/api/1.0/orders/{venue_id}")
        }
    };
    Ok(STANDARD.encode(pair.sign(msg.as_bytes()).as_ref()))
}

/// Exchange UUIDs must not introduce path/query segments into signed requests.
fn valid_exchange_uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, b)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// Minimal, validated read-only snapshot used to reconcile ambiguous orders.
/// Quantities deliberately remain decimal strings, never floating point.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrderDetails {
    pub venue_order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub status: String,
    pub quantity: String,
    pub filled_quantity: String,
}

fn decode_order_details_bytes(
    body: &[u8],
    expected_id: &str,
) -> Result<OrderDetails, PrivateReadError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(PrivateReadError::InvalidResponse);
    }
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| PrivateReadError::InvalidResponse)?;
    let data = json
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or(PrivateReadError::InvalidResponse)?;
    let field = |key: &str| -> Result<String, PrivateReadError> {
        let value = data
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or(PrivateReadError::InvalidResponse)?;
        if value.is_empty()
            || value.len() > 128
            || value.bytes().any(|byte| byte == 13 || byte == 10)
        {
            return Err(PrivateReadError::InvalidResponse);
        }
        Ok(value.to_owned())
    };
    let venue_order_id = field("id")?;
    let client_order_id = field("client_order_id")?;
    let symbol = field("symbol")?;
    let status = field("status")?;
    let quantity = field("quantity")?;
    let filled_quantity = field("filled_quantity")?;
    if venue_order_id != expected_id
        || !valid_exchange_uuid(&venue_order_id)
        || !valid_exchange_uuid(&client_order_id)
        || symbol.len() > 24
        || !symbol
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'/'))
        || !status.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        || !valid_decimal(&quantity)
        || !valid_decimal(&filled_quantity)
    {
        return Err(PrivateReadError::InvalidResponse);
    }
    Ok(OrderDetails {
        venue_order_id,
        client_order_id,
        symbol,
        status,
        quantity,
        filled_quantity,
    })
}

fn signed_order_details_headers(
    timestamp: &str,
    api_key: &[u8],
    key_pem: &[u8],
    venue_id: &str,
) -> Result<HeaderMap, PrivateReadError> {
    if !valid_exchange_uuid(venue_id)
        || api_key.len() != 64
        || !api_key.iter().all(u8::is_ascii_alphanumeric)
    {
        return Err(PrivateReadError::Credentials);
    }
    let signature = Zeroizing::new(sign_fixed_request(
        timestamp,
        key_pem,
        FixedOperation::OrderDetails(venue_id.to_owned()),
        "",
    )?);
    let mut key = HeaderValue::from_bytes(api_key).map_err(|_| PrivateReadError::Credentials)?;
    key.set_sensitive(true);
    let mut signed = HeaderValue::from_str(&signature).map_err(|_| PrivateReadError::Signing)?;
    signed.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert("X-Revx-API-Key", key);
    headers.insert(
        "X-Revx-Timestamp",
        HeaderValue::from_str(timestamp).map_err(|_| PrivateReadError::Clock)?,
    );
    headers.insert("X-Revx-Signature", signed);
    Ok(headers)
}

#[derive(Clone)]
pub struct RevolutXPrivateReadClient {
    client: Client,
}

impl RevolutXPrivateReadClient {
    pub fn new() -> Result<Self, PrivateReadError> {
        let client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| PrivateReadError::Network)?;
        Ok(Self { client })
    }

    /// Read-only authenticated reconciliation by strict venue UUID.
    /// SEC: No POST, DELETE, redirect, retry, or configurable destination.
    pub async fn order_details_with_key(
        &self,
        venue_id: &str,
        api_key: &[u8],
        private_pkcs8_pem: &[u8],
    ) -> Result<OrderDetails, PrivateReadError> {
        if !valid_exchange_uuid(venue_id) {
            return Err(PrivateReadError::InvalidResponse);
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| PrivateReadError::Clock)?
            .as_millis()
            .to_string();
        let headers =
            signed_order_details_headers(&timestamp, api_key, private_pkcs8_pem, venue_id)?;
        let response = self
            .client
            .get(format!(
                "https://revx.revolut.com/api/1.0/orders/{venue_id}"
            ))
            .headers(headers)
            .send()
            .await
            .map_err(|_| PrivateReadError::Network)?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(PrivateReadError::Unauthorized)
            }
            StatusCode::TOO_MANY_REQUESTS => return Err(PrivateReadError::RateLimited),
            status if !status.is_success() => return Err(PrivateReadError::HttpStatus),
            _ => {}
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY_BYTES as u64)
        {
            return Err(PrivateReadError::InvalidResponse);
        }
        let mut response = response;
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| PrivateReadError::Network)?
        {
            if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                return Err(PrivateReadError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        decode_order_details_bytes(&body, venue_id)
    }

    /// Only a signed GET on the fixed /api/1.0/balances path is possible.
    ///
    /// Never call from the public browser/IPC layer. Provision credentials
    /// through a Windows-only Rust flow backed by Windows Credential Manager.
    pub async fn balances_with_key(
        &self,
        api_key: &[u8],
        private_pkcs8_pem: &[u8],
    ) -> Result<Vec<Balance>, PrivateReadError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| PrivateReadError::Clock)?
            .as_millis()
            .to_string();
        let headers = signed_balance_headers(&timestamp, api_key, private_pkcs8_pem)?;
        let response = self
            .client
            .get(BALANCES_URL)
            .headers(headers)
            .send()
            .await
            .map_err(|_| PrivateReadError::Network)?;
        decode_balances_response(response).await
    }
}

fn signed_balance_headers(
    timestamp: &str,
    api_key: &[u8],
    key_pem: &[u8],
) -> Result<HeaderMap, PrivateReadError> {
    if api_key.len() != 64 || !api_key.iter().all(u8::is_ascii_alphanumeric) {
        return Err(PrivateReadError::Credentials);
    }
    let mut key_header =
        HeaderValue::from_bytes(api_key).map_err(|_| PrivateReadError::Credentials)?;
    key_header.set_sensitive(true);
    let signed = Zeroizing::new(sign_balances(timestamp, key_pem)?);
    let mut signature = HeaderValue::from_str(&signed).map_err(|_| PrivateReadError::Signing)?;
    signature.set_sensitive(true);
    let timestamp = HeaderValue::from_str(timestamp).map_err(|_| PrivateReadError::Clock)?;
    let mut headers = HeaderMap::new();
    headers.insert("X-Revx-API-Key", key_header);
    headers.insert("X-Revx-Timestamp", timestamp);
    headers.insert("X-Revx-Signature", signature);
    Ok(headers)
}

/// Validate credential formats and local signing without contacting Revolut X.
/// This does not establish that the API key belongs to the signing key.
pub fn validate_credentials(api_key: &[u8], key_pem: &[u8]) -> Result<(), PrivateReadError> {
    signed_balance_headers("1", api_key, key_pem).map(|_| ())
}

async fn decode_balances_response(response: Response) -> Result<Vec<Balance>, PrivateReadError> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            return Err(PrivateReadError::Unauthorized);
        }
        StatusCode::TOO_MANY_REQUESTS => return Err(PrivateReadError::RateLimited),
        status if !status.is_success() => return Err(PrivateReadError::HttpStatus),
        _ => {}
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BODY_BYTES as u64)
    {
        return Err(PrivateReadError::InvalidResponse);
    }
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| PrivateReadError::Network)?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
            return Err(PrivateReadError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    let balances: Vec<Balance> =
        serde_json::from_slice(&bytes).map_err(|_| PrivateReadError::InvalidResponse)?;
    validate_balances(&balances)?;
    Ok(balances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{KeyPair, UnparsedPublicKey, ED25519};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn temporary_pem() -> String {
        let rng = ring::rand::SystemRandom::new();
        let der = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        format!("{PEM_START}\n{}\n{PEM_END}", STANDARD.encode(der.as_ref()))
    }

    async fn local_response(status: &str, body: &[u8], declared_length: Option<usize>) -> Response {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let header = match declared_length {
            Some(length) => format!(
                "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
            ),
            None => format!("HTTP/1.1 {status}\r\nConnection: close\r\n\r\n"),
        };
        let body = body.to_vec();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await;
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&body).await;
        });
        Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
            .get(format!("http://{address}/balances"))
            .send()
            .await
            .unwrap()
    }

    #[test]
    fn readonly_order_details_signature_uses_exact_documented_path() {
        let rng = ring::rand::SystemRandom::new();
        let der = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(der.as_ref()).unwrap();
        let pem = format!("{PEM_START}\n{}\n{PEM_END}", STANDARD.encode(der.as_ref()));
        let id = "7a52e92e-8639-4fe1-abaa-68d3a2d5234b";
        let signed = sign_fixed_request(
            "1785313433816",
            pem.as_bytes(),
            FixedOperation::OrderDetails(id.to_owned()),
            "",
        )
        .unwrap();
        let decoded = STANDARD.decode(signed).unwrap();
        let expected = format!("1785313433816GET/api/1.0/orders/{id}");
        UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref())
            .verify(expected.as_bytes(), &decoded)
            .unwrap();
        assert!(!valid_exchange_uuid("../balances?x=1"));
        assert!(!valid_exchange_uuid("abc"));
    }

    #[test]
    fn readonly_reconciliation_rejects_malformed_and_wrong_order_identity() {
        let id = "7a52e92e-8639-4fe1-abaa-68d3a2d5234b";
        let body = format!(
            r#"{{"data":{{"id":"{id}","client_order_id":"984a4d8a-2a9b-4950-822f-2a40037f02bd","symbol":"BTC/USD","status":"new","quantity":"0.002","filled_quantity":"0"}}}}"#
        );
        let result = decode_order_details_bytes(body.as_bytes(), id).unwrap();
        assert_eq!(result.quantity, "0.002");
        assert!(decode_order_details_bytes(
            body.as_bytes(),
            "123e4567-e89b-12d3-a456-426614174000"
        )
        .is_err());
        assert!(decode_order_details_bytes(b"{}", id).is_err());
        assert!(decode_order_details_bytes(&vec![b'x'; MAX_BODY_BYTES + 1], id).is_err());
        assert!(decode_order_details_bytes(body.replace("0.002", "NaN").as_bytes(), id).is_err());
    }

    #[test]
    fn authenticates_only_the_documented_fixed_balances_path() {
        let rng = ring::rand::SystemRandom::new();
        let der = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(der.as_ref()).unwrap();
        let pem = format!("{PEM_START}\n{}\n{PEM_END}", STANDARD.encode(der.as_ref()));
        let signature = sign_balances("1785313433816", pem.as_bytes()).unwrap();
        let signature = STANDARD.decode(signature).unwrap();
        UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref())
            .verify(b"1785313433816GET/api/1.0/balances", &signature)
            .unwrap();
        assert!(UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref())
            .verify(b"1785313433816POST/api/1.0/orders", &signature)
            .is_err());
    }

    #[test]
    fn signs_offline_limit_order_exact_body_without_sending() {
        let rng = ring::rand::SystemRandom::new();
        let der = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(der.as_ref()).unwrap();
        let pem = format!("{PEM_START}\n{}\n{PEM_END}", STANDARD.encode(der.as_ref()));
        let body = r#"{"client_order_id":"3b364427-1f4f-4f66-9935-86b6fb115d26","symbol":"BTC-USD","side":"BUY","order_configuration":{"limit":{"base_size":"0.01","price":"90000"}}}"#;
        let sig = sign_prepared_limit_order("1785313433816", pem.as_bytes(), body).unwrap();
        let sig = STANDARD.decode(sig).unwrap();
        let msg = format!("1785313433816POST/api/1.0/orders{body}");
        UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref())
            .verify(msg.as_bytes(), &sig)
            .unwrap();
        assert!(UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref())
            .verify(b"1785313433816GET/api/1.0/balances", &sig)
            .is_err());
    }

    #[test]
    fn invalid_keys_are_rejected_before_network_access() {
        for invalid in [b"not a key".as_slice(), b"".as_slice()] {
            assert!(sign_balances("1785313433816", invalid).is_err());
        }
        assert_eq!(sign_balances("wrong", b""), Err(PrivateReadError::Clock));
    }

    #[test]
    fn sensitive_headers_are_redacted_in_debug_output() {
        let api_key = "A".repeat(64);
        let headers = signed_balance_headers(
            "1785313433816",
            api_key.as_bytes(),
            temporary_pem().as_bytes(),
        )
        .unwrap();
        let key = headers.get("X-Revx-API-Key").unwrap();
        let signature = headers.get("X-Revx-Signature").unwrap();
        assert!(key.is_sensitive());
        assert!(signature.is_sensitive());
        let debug = format!("{headers:?}");
        assert!(!debug.contains(&api_key));
        assert!(!debug.contains(signature.to_str().unwrap()));
        assert_eq!(headers.get("X-Revx-Timestamp").unwrap(), "1785313433816");
        assert!(matches!(
            signed_balance_headers("1785313433816", b"bad", temporary_pem().as_bytes()),
            Err(PrivateReadError::Credentials)
        ));
    }

    #[tokio::test]
    async fn response_decoder_accepts_official_balances_and_rejects_http_errors() {
        let body = br#"[{"currency":"BTC","available":"1.25000000","reserved":"0.10000000","total":"1.35000000"},{"currency":"ETH","available":"10.00000000","reserved":"0.00000000","staked":"32.00000000","total":"42.00000000"}]"#;
        let response = local_response("200 OK", body, Some(body.len())).await;
        let balances = decode_balances_response(response).await.unwrap();
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[1].staked.as_deref(), Some("32.00000000"));
        for (status, expected) in [
            ("401 Unauthorized", PrivateReadError::Unauthorized),
            ("403 Forbidden", PrivateReadError::Unauthorized),
            ("429 Too Many Requests", PrivateReadError::RateLimited),
            ("500 Internal Server Error", PrivateReadError::HttpStatus),
        ] {
            let response = local_response(status, b"", Some(0)).await;
            assert_eq!(
                decode_balances_response(response).await.unwrap_err(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn response_decoder_rejects_invalid_or_oversized_bodies() {
        let invalid = br#"[{"currency":"BTC","available":"-1","reserved":"0","total":"0"}]"#;
        let response = local_response("200 OK", invalid, Some(invalid.len())).await;
        assert_eq!(
            decode_balances_response(response).await.unwrap_err(),
            PrivateReadError::InvalidResponse
        );
        let response = local_response("200 OK", b"{}", Some(MAX_BODY_BYTES + 1)).await;
        assert_eq!(
            decode_balances_response(response).await.unwrap_err(),
            PrivateReadError::InvalidResponse
        );
        let body = vec![b'x'; MAX_BODY_BYTES + 1];
        let response = local_response("200 OK", &body, None).await;
        assert_eq!(
            decode_balances_response(response).await.unwrap_err(),
            PrivateReadError::InvalidResponse
        );
    }

    #[test]
    fn balances_reject_invalid_quantities_and_symbols_without_floating_point() {
        let sample = r#"[{"currency":"BTC","available":"0.025","reserved":"0","total":"0.025"}]"#;
        let valid: Vec<Balance> = serde_json::from_str(sample).unwrap();
        validate_balances(&valid).unwrap();
        for invalid in ["NaN", "-1", "1e200", "0.1.2", "1."] {
            let bad = vec![Balance {
                currency: "USD".to_owned(),
                available: invalid.to_owned(),
                reserved: "0".to_owned(),
                total: "0".to_owned(),
                staked: None,
            }];
            assert_eq!(
                validate_balances(&bad),
                Err(PrivateReadError::InvalidResponse)
            );
        }
    }
}
