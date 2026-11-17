//! Redacted, read-only view of the private Revolut X connection for the UI.
//!
//! The vault, the signing client and the credential bundle stay inside this
//! crate: callers only ever receive closed vocabulary and decimal strings.
//! Nothing here accepts a credential, and no order route exists.
use crate::revolut_x_account::RevolutXVaultAccount;
use crate::secret_store::{SecretStore, WindowsCredentialStore};

const MAX_BALANCE_ROWS: usize = 64;

fn vault_account() -> Result<RevolutXVaultAccount<WindowsCredentialStore>, &'static str> {
    RevolutXVaultAccount::new(WindowsCredentialStore).map_err(|error| error.code())
}

fn connection_state<S: SecretStore>(account: &RevolutXVaultAccount<S>) -> &'static str {
    match account.configured() {
        Ok(true) => "configured",
        Ok(false) => "unconfigured",
        Err(error) => match error.code() {
            "PRIVATE_UNSUPPORTED_PLATFORM" => "unsupported",
            _ => "unavailable",
        },
    }
}

fn state_rows<S: SecretStore>(
    account: &RevolutXVaultAccount<S>,
    consent_enabled: bool,
) -> Vec<String> {
    let state = connection_state(account);
    let consent = if consent_enabled { "enabled" } else { "disabled" };
    let vault = if state == "unsupported" {
        "unsupported"
    } else {
        "windows_credential_manager"
    };
    vec![
        format!("connection|{state}"),
        format!("consent|{consent}"),
        "scope|read_only".to_owned(),
        "orders|disabled".to_owned(),
        format!("vault|{vault}"),
    ]
}

/// Local connection state. No network call, no key material, no raw error.
pub fn status_rows(consent_enabled: bool) -> Vec<String> {
    let consent = if consent_enabled { "enabled" } else { "disabled" };
    match vault_account() {
        Ok(account) => state_rows(&account, consent_enabled),
        Err(_) => vec![
            "connection|unavailable".to_owned(),
            format!("consent|{consent}"),
            "scope|read_only".to_owned(),
            "orders|disabled".to_owned(),
            "vault|windows_credential_manager".to_owned(),
        ],
    }
}

fn valid_currency(value: &str) -> bool {
    (2..=10).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

/// Decimal strings only, as published by the venue. No float conversion, no
/// sign, no exponent, no separator and no injected markup.
fn valid_amount(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value.bytes().filter(|byte| *byte == b'.').count() <= 1
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

fn validated_balance_row(
    currency: &str,
    available: &str,
    reserved: &str,
    total: &str,
    staked: Option<&str>,
) -> Result<String, &'static str> {
    if !valid_currency(currency)
        || !valid_amount(available)
        || !valid_amount(reserved)
        || !valid_amount(total)
    {
        return Err("PRIVATE_INVALID_RESPONSE");
    }
    let staked = match staked {
        None => "none",
        Some(value) if valid_amount(value) => value,
        Some(_) => return Err("PRIVATE_INVALID_RESPONSE"),
    };
    Ok(format!("balance|{currency}|{available}|{reserved}|{total}|{staked}"))
}

async fn read_rows<S: SecretStore>(
    account: &RevolutXVaultAccount<S>,
) -> Result<Vec<String>, &'static str> {
    // `balances()` loads and validates the stored pair before any request, so
    // an unconfigured vault never produces network traffic.
    let balances = account.balances().await.map_err(|error| error.code())?;
    if balances.len() > MAX_BALANCE_ROWS {
        return Err("PRIVATE_INVALID_RESPONSE");
    }
    let observed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "PRIVATE_UNAVAILABLE")?
        .as_millis();
    let mut rows = vec![
        "status|ok".to_owned(),
        format!("observed_at|{observed_at}"),
        format!("count|{}", balances.len()),
    ];
    for balance in &balances {
        rows.push(validated_balance_row(
            &balance.currency,
            &balance.available,
            &balance.reserved,
            &balance.total,
            balance.staked.as_deref(),
        )?);
    }
    Ok(rows)
}

/// One explicit user action performs both the connection test and the
/// read-only balance read: the venue exposes a single allowlisted signed GET.
/// The caller must hold an authenticated session and an explicit opt-in.
pub async fn balance_rows() -> Result<Vec<String>, &'static str> {
    read_rows(&vault_account()?).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::{SecretStoreError, SecretValue};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, SecretValue>>);
    impl SecretStore for MemoryStore {
        fn put(&self, name: &str, value: &SecretValue) -> Result<(), SecretStoreError> {
            self.0
                .lock()
                .unwrap()
                .insert(name.to_owned(), SecretValue::new(value.expose().to_vec()));
            Ok(())
        }
        fn get(&self, name: &str) -> Result<SecretValue, SecretStoreError> {
            self.0
                .lock()
                .unwrap()
                .get(name)
                .map(|value| SecretValue::new(value.expose().to_vec()))
                .ok_or(SecretStoreError::NotFound)
        }
        fn remove(&self, name: &str) -> Result<(), SecretStoreError> {
            self.0
                .lock()
                .unwrap()
                .remove(name)
                .map(|_| ())
                .ok_or(SecretStoreError::NotFound)
        }
    }

    #[test]
    fn status_rows_use_a_closed_vocabulary_and_no_key_material() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        assert_eq!(
            state_rows(&account, false),
            vec![
                "connection|unconfigured".to_owned(),
                "consent|disabled".to_owned(),
                "scope|read_only".to_owned(),
                "orders|disabled".to_owned(),
                "vault|windows_credential_manager".to_owned(),
            ]
        );
        let enabled = state_rows(&account, true);
        assert_eq!(enabled[1], "consent|enabled");
        assert!(enabled.iter().all(|row| row.len() <= 40));
        assert!(enabled
            .iter()
            .all(|row| row.split('|').count() == 2 && !row.contains(char::is_whitespace)));
    }

    #[test]
    fn corrupt_bundle_reports_unavailable_instead_of_claiming_configuration() {
        let store = MemoryStore::default();
        store
            .put("revolut_x:credentials_v1", &SecretValue::new(vec![9, 9]))
            .unwrap();
        let account = RevolutXVaultAccount::new(store).unwrap();
        assert_eq!(state_rows(&account, false)[0], "connection|unavailable");
    }

    #[tokio::test]
    async fn unconfigured_vault_refuses_without_network_traffic() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        assert_eq!(read_rows(&account).await, Err("PRIVATE_NOT_CONFIGURED"));
    }

    #[test]
    fn balance_rows_accept_only_decimal_strings_and_ticker_symbols() {
        assert_eq!(
            validated_balance_row("BTC", "0.5", "0", "0.5", None).unwrap(),
            "balance|BTC|0.5|0|0.5|none"
        );
        assert_eq!(
            validated_balance_row("ETH", "1", "0.25", "1.25", Some("0.25")).unwrap(),
            "balance|ETH|1|0.25|1.25|0.25"
        );
        for bad in [
            ("btc", "1", "0", "1", None),
            ("BTC", "-1", "0", "1", None),
            ("BTC", "1e3", "0", "1", None),
            ("BTC", "1", "0", "1", Some("NaN")),
            ("BTC", "1", "0", ".5", None),
            ("BTC", "1", "0", "1.", None),
            ("BTC", "1", "0", "1.2.3", None),
            ("<img src=x>", "1", "0", "1", None),
            ("BTC", "1", "0", "", None),
        ] {
            assert_eq!(
                validated_balance_row(bad.0, bad.1, bad.2, bad.3, bad.4),
                Err("PRIVATE_INVALID_RESPONSE")
            );
        }
        assert!(!valid_amount(&"9".repeat(65)));
        assert!(!valid_currency(&"A".repeat(11)));
    }
}