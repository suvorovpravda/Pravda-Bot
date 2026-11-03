//! Windows-vault access to the fixed, read-only Revolut X balances endpoint.
//! No order, cancellation, Tauri IPC or HTTP route is provided here.
use crate::secret_store::{SecretStore, SecretStoreError, SecretValue};
use pravda_fetchers::revolut_x_private::{
    validate_credentials, Balance, OrderDetails, PrivateReadError, RevolutXPrivateReadClient,
};
use thiserror::Error;

const REVX_BUNDLE: &str = "revolut_x:credentials_v1";
const REVX_API_KEY: &str = "revolut_x:api_key";
const REVX_ED25519_PRIVATE_PEM: &str = "revolut_x:ed25519_pkcs8_pem";
const BUNDLE_VERSION: u8 = 1;
const API_KEY_LEN: usize = 64;
const MAX_BUNDLE_LEN: usize = 1024;

#[derive(Debug, Error)]
pub enum AccountConnectionError {
    #[error("Revolut X credential storage is unavailable")]
    Credentials(#[from] SecretStoreError),
    #[error("Revolut X authenticated read-only balance request failed")]
    Request(#[from] PrivateReadError),
    #[error("Revolut X credentials already exist; no changes were made")]
    AlreadyConfigured,
    #[error("Revolut X stored credentials are invalid")]
    InvalidBundle,
}

enum Credentials {
    Bundle(SecretValue),
    Legacy(SecretValue, SecretValue),
}

impl Credentials {
    fn api_key(&self) -> &[u8] {
        match self {
            Self::Bundle(bundle) => &bundle.expose()[1..1 + API_KEY_LEN],
            Self::Legacy(api_key, _) => api_key.expose(),
        }
    }

    fn private_pem(&self) -> &[u8] {
        match self {
            Self::Bundle(bundle) => &bundle.expose()[1 + API_KEY_LEN..],
            Self::Legacy(_, pem) => pem.expose(),
        }
    }
}

/// Service-layer only. Secrets never cross a Tauri command or HTTP endpoint.
pub struct RevolutXVaultAccount<S: SecretStore> {
    store: S,
    client: RevolutXPrivateReadClient,
}

impl<S: SecretStore> RevolutXVaultAccount<S> {
    pub fn new(store: S) -> Result<Self, AccountConnectionError> {
        Ok(Self {
            store,
            client: RevolutXPrivateReadClient::new()?,
        })
    }

    /// Store a validated pair in one record, refusing to replace existing entries.
    /// This validates local structure/signing, not ownership or API permissions.
    pub fn provision_new(&self, api_key: &[u8], pem: &[u8]) -> Result<(), AccountConnectionError> {
        for name in [REVX_BUNDLE, REVX_API_KEY, REVX_ED25519_PRIVATE_PEM] {
            match self.store.get(name) {
                Ok(_) => return Err(AccountConnectionError::AlreadyConfigured),
                Err(SecretStoreError::NotFound) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let size = 1_usize
            .saturating_add(api_key.len())
            .saturating_add(pem.len());
        if size > MAX_BUNDLE_LEN {
            return Err(SecretStoreError::InvalidSize.into());
        }
        validate_credentials(api_key, pem)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.push(BUNDLE_VERSION);
        bytes.extend_from_slice(api_key);
        bytes.extend_from_slice(pem);
        self.store.put(REVX_BUNDLE, &SecretValue::new(bytes))?;
        Ok(())
    }

    fn load_credentials(&self) -> Result<Credentials, AccountConnectionError> {
        match self.store.get(REVX_BUNDLE) {
            Ok(bundle) => {
                let data = bundle.expose();
                if data.len() <= 1 + API_KEY_LEN || data[0] != BUNDLE_VERSION {
                    return Err(AccountConnectionError::InvalidBundle);
                }
                validate_credentials(&data[1..1 + API_KEY_LEN], &data[1 + API_KEY_LEN..])
                    .map_err(|_| AccountConnectionError::InvalidBundle)?;
                Ok(Credentials::Bundle(bundle))
            }
            Err(SecretStoreError::NotFound) => Ok(Credentials::Legacy(
                self.store.get(REVX_API_KEY)?,
                self.store.get(REVX_ED25519_PRIVATE_PEM)?,
            )),
            Err(error) => Err(error.into()),
        }
    }

    /// Only a signed GET request is available; no real order is sent.
    pub async fn balances(&self) -> Result<Vec<Balance>, AccountConnectionError> {
        let credentials = self.load_credentials()?;
        Ok(self
            .client
            .balances_with_key(credentials.api_key(), credentials.private_pem())
            .await?)
    }
    /// Authenticated reconciliation uses only an allowlisted GET path.
    /// Credential bytes never leave the backend except signed HTTPS headers.
    pub async fn order_details(
        &self,
        venue_id: &str,
    ) -> Result<OrderDetails, AccountConnectionError> {
        let credentials = self.load_credentials()?;
        Ok(self
            .client
            .order_details_with_key(venue_id, credentials.api_key(), credentials.private_pem())
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ring::signature::Ed25519KeyPair;
    use std::{collections::HashMap, sync::Mutex};

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

    fn temporary_pem() -> String {
        let key = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----",
            "PRIVATE KEY",
            STANDARD.encode(key.as_ref()),
            "PRIVATE KEY"
        )
    }

    #[tokio::test]
    async fn order_details_unprovisioned_fails_before_any_request() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        let result = account
            .order_details("7a52e92e-8639-4fe1-abaa-68d3a2d5234b")
            .await;
        assert!(matches!(
            result,
            Err(AccountConnectionError::Credentials(
                SecretStoreError::NotFound
            ))
        ));
    }

    #[tokio::test]
    async fn unprovisioned_store_fails_before_network() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        assert!(matches!(
            account.balances().await,
            Err(AccountConnectionError::Credentials(
                SecretStoreError::NotFound
            ))
        ));
    }

    #[test]
    fn provision_valid_pair_once_and_load_the_same_pair() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        let key = "A".repeat(API_KEY_LEN);
        let pem = temporary_pem();
        account
            .provision_new(key.as_bytes(), pem.as_bytes())
            .unwrap();
        let loaded = account.load_credentials().unwrap();
        assert_eq!(loaded.api_key(), key.as_bytes());
        assert_eq!(loaded.private_pem(), pem.as_bytes());
        assert_eq!(account.store.0.lock().unwrap().len(), 1);
        assert!(matches!(
            account.provision_new(key.as_bytes(), pem.as_bytes()),
            Err(AccountConnectionError::AlreadyConfigured)
        ));
    }

    #[test]
    fn invalid_or_oversized_credentials_do_not_write() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        let key = "A".repeat(API_KEY_LEN);
        assert!(account
            .provision_new(b"bad", temporary_pem().as_bytes())
            .is_err());
        assert!(account.provision_new(key.as_bytes(), b"bad").is_err());
        assert!(account
            .provision_new(key.as_bytes(), &vec![b'X'; MAX_BUNDLE_LEN])
            .is_err());
        assert!(account.store.0.lock().unwrap().is_empty());
    }

    #[test]
    fn legacy_pair_loads_but_cannot_be_overwritten() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        let key = "A".repeat(API_KEY_LEN);
        let pem = temporary_pem();
        account
            .store
            .put(REVX_API_KEY, &SecretValue::new(key.as_bytes().to_vec()))
            .unwrap();
        account
            .store
            .put(
                REVX_ED25519_PRIVATE_PEM,
                &SecretValue::new(pem.as_bytes().to_vec()),
            )
            .unwrap();
        assert_eq!(
            account.load_credentials().unwrap().private_pem(),
            pem.as_bytes()
        );
        assert!(matches!(
            account.provision_new(key.as_bytes(), pem.as_bytes()),
            Err(AccountConnectionError::AlreadyConfigured)
        ));
    }

    #[test]
    fn corrupt_bundle_fails_closed_instead_of_using_legacy() {
        let account = RevolutXVaultAccount::new(MemoryStore::default()).unwrap();
        account
            .store
            .put(REVX_BUNDLE, &SecretValue::new(vec![2, 0]))
            .unwrap();
        account
            .store
            .put(REVX_API_KEY, &SecretValue::new(vec![b'A'; API_KEY_LEN]))
            .unwrap();
        account
            .store
            .put(
                REVX_ED25519_PRIVATE_PEM,
                &SecretValue::new(temporary_pem().into_bytes()),
            )
            .unwrap();
        assert!(matches!(
            account.load_credentials(),
            Err(AccountConnectionError::InvalidBundle)
        ));
    }
}
