//! Authenticated Revolut X **read-only** balances API.
//!
//! The private API uses Ed25519 signatures. This module deliberately has NO
//! order creation/cancellation functions or arbitrary HTTP method/path input.
//! API keys and PKCS#8 material are supplied by Rust callers and never logged.
//! See https://developer.revolut.com/docs/api/revolut-x-crypto-exchange
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{header::HeaderValue, Client, StatusCode};
use ring::signature::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

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
fn parse_ed25519_pem(secret: &[u8]) -> Result<Vec<u8>, PrivateReadError> {
    let text = std::str::from_utf8(secret).map_err(|_| PrivateReadError::Credentials)?;
    let mut lines = text.trim().lines();
    if lines.next() != Some(PEM_START) {
        return Err(PrivateReadError::Credentials);
    }
    let mut payload = String::new();
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
        payload.push_str(part);
        if payload.len() > 2048 {
            return Err(PrivateReadError::Credentials);
        }
    }
    if !closed || lines.next().is_some() {
        return Err(PrivateReadError::Credentials);
    }
    STANDARD
        .decode(payload)
        .map_err(|_| PrivateReadError::Credentials)
}

/// The fixed request preimage is timestamp + "GET" + full API path.
/// Revolut X does not add separators or a body for this request.
fn sign_balances(timestamp: &str, key_pem: &[u8]) -> Result<String, PrivateReadError> {
    if timestamp.is_empty() || !timestamp.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PrivateReadError::Clock);
    }
    let mut der = parse_ed25519_pem(key_pem)?;
    let pair =
        Ed25519KeyPair::from_pkcs8_maybe_unchecked(&der).map_err(|_| PrivateReadError::Signing);
    for b in &mut der {
        // Best-effort wipe of temporary DER; upstream key internals remain
        // subject to OS process-memory threats documented in SECURITY.md.
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    let pair = pair?;
    let msg = format!("{timestamp}GET{BALANCES_PATH}");
    Ok(STANDARD.encode(pair.sign(msg.as_bytes()).as_ref()))
}

#[derive(Clone)]
pub struct RevolutXPrivateReadClient {
    client: Client,
}

impl RevolutXPrivateReadClient {
    pub fn new() -> Result<Self, PrivateReadError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| PrivateReadError::Network)?;
        Ok(Self { client })
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
        if api_key.len() != 64 || !api_key.iter().all(u8::is_ascii_alphanumeric) {
            return Err(PrivateReadError::Credentials);
        }
        let key_header =
            HeaderValue::from_bytes(api_key).map_err(|_| PrivateReadError::Credentials)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| PrivateReadError::Clock)?
            .as_millis()
            .to_string();
        let signature = sign_balances(&timestamp, private_pkcs8_pem)?;
        let response = self
            .client
            .get(BALANCES_URL)
            .header("X-Revx-API-Key", key_header)
            .header("X-Revx-Timestamp", timestamp)
            .header("X-Revx-Signature", signature)
            .send()
            .await
            .map_err(|_| PrivateReadError::Network)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{KeyPair, UnparsedPublicKey, ED25519};

    #[test]
    fn authenticates_only_the_documented_fixed_balances_path() {
        let rng = ring::rand::SystemRandom::new();
        let der = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(der.as_ref()).unwrap();
        let pem = format!(
            "{PEM_START}\n{}\n{PEM_END}",
            STANDARD.encode(der.as_ref())
        );
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
    fn invalid_keys_are_rejected_before_network_access() {
        for invalid in [b"not a key".as_slice(), b"".as_slice()] {
            assert!(sign_balances("1785313433816", invalid).is_err());
        }
        assert_eq!(sign_balances("wrong", b""), Err(PrivateReadError::Clock));
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
