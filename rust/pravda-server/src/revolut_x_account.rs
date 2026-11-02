//! Rust-only bridge between Windows Credential Manager and the official
//! Revolut X authenticated balances endpoint.
//!
//! There is deliberately no Tauri IPC or Axum route for provisioning secrets,
//! reading account balances, cancelling orders or sending real orders. Those
//! capabilities require separate review and explicit user authorization.
use crate::secret_store::{SecretStore, SecretStoreError};
use pravda_fetchers::revolut_x_private::{Balance, PrivateReadError, RevolutXPrivateReadClient};
use thiserror::Error;

const REVX_API_KEY: &str = "revolut_x:api_key";
const REVX_ED25519_PRIVATE_PEM: &str = "revolut_x:ed25519_pkcs8_pem";

#[derive(Debug, Error)]
pub enum AccountConnectionError {
    #[error("Revolut X credentials are not provisioned in Windows Credential Manager")]
    Credentials(#[from] SecretStoreError),
    #[error("Revolut X authenticated read-only balance request failed")]
    Request(#[from] PrivateReadError),
}

/// Instantiated by trusted Rust code with a SecretStore (normally
/// WindowsCredentialStore). The store owns the secret lifecycle; no key is
/// copied into SQLite, webviews, logs, diagnostic HTTP handlers, or config.
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

    /// Requires two independently provisioned secrets in the Windows vault.
    /// The request is signed and GET-only. No real trading calls exist.
    pub async fn balances(&self) -> Result<Vec<Balance>, AccountConnectionError> {
        let api_key = self.store.get(REVX_API_KEY)?;
        let signing_key = self.store.get(REVX_ED25519_PRIVATE_PEM)?;
        let balances = self
            .client
            .balances_with_key(api_key.expose(), signing_key.expose())
            .await?;
        Ok(balances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct EmptyStore;
    impl SecretStore for EmptyStore {
        fn put(
            &self,
            _identifier: &str,
            _secret: &crate::secret_store::SecretValue,
        ) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::UnsupportedPlatform)
        }
        fn get(
            &self,
            _identifier: &str,
        ) -> Result<crate::secret_store::SecretValue, SecretStoreError> {
            Err(SecretStoreError::NotFound)
        }
        fn remove(&self, _identifier: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::UnsupportedPlatform)
        }
    }

    #[tokio::test]
    async fn fails_closed_before_network_when_vault_is_unprovisioned() {
        let account = RevolutXVaultAccount::new(EmptyStore).unwrap();
        assert!(matches!(
            account.balances().await,
            Err(AccountConnectionError::Credentials(
                SecretStoreError::NotFound
            ))
        ));
    }

    #[test]
    fn requires_only_scoped_named_vault_entries() {
        assert_eq!(REVX_API_KEY, "revolut_x:api_key");
        assert_eq!(REVX_ED25519_PRIVATE_PEM, "revolut_x:ed25519_pkcs8_pem");
    }
}
