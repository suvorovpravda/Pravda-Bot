//! Dormant, Windows-only registration for the official Tauri updater.
//!
//! This module performs no update checks, downloads or installations. It is
//! excluded from ordinary builds. No plugin command is granted to the WebView.
use base64::{engine::general_purpose::STANDARD, Engine};
use tauri::{Manager, Runtime};
use tauri_plugin_updater::Config;

const STABLE_ENDPOINT: &str =
    "https://github.com/suvorovpravda/Pravda-Bot/releases/latest/download/latest.json";

fn safe_config(config: &Config) -> bool {
    if config.endpoints.len() != 1
        || config.endpoints[0].as_str() != STABLE_ENDPOINT
        || !config.require_signed_version
        || config.allow_downgrades
        || config.dangerous_insecure_transport_protocol
        || config.dangerous_accept_invalid_certs
        || config.dangerous_accept_invalid_hostnames
        || config
            .windows
            .as_ref()
            .is_some_and(|windows| !windows.installer_args.is_empty())
    {
        return false;
    }
    let key = config.pubkey.trim();
    if key.len() < 40 || key.len() > 4096 || key != config.pubkey {
        return false;
    }
    let Ok(decoded) = STANDARD.decode(key) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&decoded) else {
        return false;
    };
    let mut lines = text.lines();
    let Some(comment) = lines.next() else {
        return false;
    };
    if !comment.starts_with("untrusted comment: ")
        || comment.to_ascii_lowercase().contains("private")
        || lines.next().is_none()
        || lines.next().is_some()
    {
        return false;
    }
    minisign_verify::PublicKey::decode(text).is_ok()
}

fn configured_updater(value: Option<&serde_json::Value>, offline: bool) -> bool {
    if offline {
        return false;
    }
    let Some(value) = value else {
        return false;
    };
    let Ok(config) = serde_json::from_value::<Config>(value.clone()) else {
        return false;
    };
    safe_config(&config)
}

/// Registration only: this does not start a task, make an HTTP request or
/// expose any plugin permission to the frontend. Invalid config fails closed
/// without preventing the rest of the offline application from starting.
pub fn register_if_safe<R: Runtime>(app: &tauri::App<R>, offline: bool) {
    let plugin_config = app.config().plugins.0.get("updater");
    if configured_updater(plugin_config, offline)
        && app
            .handle()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .is_err()
    {
        eprintln!("Signed updater registration unavailable");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Public verification key from a non-production test vector; no private
    // key is included or configured as a runtime trust anchor.
    fn test_public_key() -> String {
        STANDARD.encode(concat!(
            "untrusted comment: minisign public key: A49EF4746FA6E6D7\n",
            "RWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe\n"
        ))
    }

    fn valid_config() -> serde_json::Value {
        json!({
            "pubkey": test_public_key(),
            "endpoints": [STABLE_ENDPOINT],
            "requireSignedVersion": true,
            "allowDowngrades": false,
            "dangerousInsecureTransportProtocol": false,
            "dangerousAcceptInvalidCerts": false,
            "dangerousAcceptInvalidHostnames": false
        })
    }

    #[test]
    fn needs_valid_config_and_is_disabled_in_offline_mode() {
        let value = valid_config();
        assert!(configured_updater(Some(&value), false));
        assert!(!configured_updater(Some(&value), true));
        assert!(!configured_updater(None, false));
    }

    #[test]
    fn rejects_untrusted_endpoints_or_keys() {
        let mut value = valid_config();
        for endpoint in [
            "http://github.com/suvorovpravda/Pravda-Bot/releases/latest/download/latest.json",
            "https://example.invalid/latest.json",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/latest/download/latest.json?x=1",
        ] {
            value["endpoints"] = json!([endpoint]);
            assert!(!configured_updater(Some(&value), false));
        }
        value = valid_config();
        value["endpoints"] = json!([STABLE_ENDPOINT, STABLE_ENDPOINT]);
        assert!(!configured_updater(Some(&value), false));
        value = valid_config();
        value["pubkey"] = json!("invalid-or-missing");
        assert!(!configured_updater(Some(&value), false));
        value = valid_config();
        let mut decoded = STANDARD.decode(test_public_key()).unwrap();
        decoded.extend_from_slice(b"unexpected-extra-line\n");
        value["pubkey"] = json!(STANDARD.encode(decoded));
        assert!(!configured_updater(Some(&value), false));
    }

    #[test]
    fn rejects_downgrades_unsigned_versions_and_unsafe_tls() {
        for (field, unsafe_value) in [
            ("requireSignedVersion", false),
            ("allowDowngrades", true),
            ("dangerousInsecureTransportProtocol", true),
            ("dangerousAcceptInvalidCerts", true),
            ("dangerousAcceptInvalidHostnames", true),
        ] {
            let mut value = valid_config();
            value[field] = json!(unsafe_value);
            assert!(!configured_updater(Some(&value), false), "{field}");
        }
        let mut value = valid_config();
        value["windows"] = json!({"installerArgs":["/quiet"]});
        assert!(!configured_updater(Some(&value), false));
    }
}
