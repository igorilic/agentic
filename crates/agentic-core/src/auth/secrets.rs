#[cfg(any(test, feature = "testing"))]
use std::collections::HashMap;
#[cfg(any(test, feature = "testing"))]
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret not found: {service}/{key}")]
    NotFound { service: String, key: String },
    // #40 — preserve keyring::Error cause chain via #[source]
    #[error("keyring backend error")]
    Backend {
        #[source]
        source: keyring::Error,
    },
    /// Wraps a poisoned mutex from MemSecretStore — shouldn't happen in practice.
    #[error("internal lock poisoned")]
    Lock,
}

/// Abstract secret storage. Production uses OS keyring; tests use in-memory.
pub trait SecretStore: Send + Sync {
    /// Retrieve the secret for the given key. Returns
    /// `Err(SecretStoreError::NotFound)` if no secret is stored.
    fn get(&self, key: &str) -> Result<String, SecretStoreError>;

    /// Set or overwrite the secret for the given key.
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;

    /// Delete the secret. Idempotent: deleting a missing key returns `Ok(())`.
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
}

/// In-memory secret store for tests and fixtures. Thread-safe via Mutex.
// #41 — carries a `service` field so NotFound reports the real service name
#[cfg(any(test, feature = "testing"))]
#[derive(Debug)]
pub struct MemSecretStore {
    service: String,
    inner: Mutex<HashMap<String, String>>,
}

#[cfg(any(test, feature = "testing"))]
impl MemSecretStore {
    pub fn new() -> Self {
        Self::with_service("mem".to_string())
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            inner: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for MemSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "testing"))]
impl SecretStore for MemSecretStore {
    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let map = self.inner.lock().map_err(|_| SecretStoreError::Lock)?;
        map.get(key)
            .cloned()
            .ok_or_else(|| SecretStoreError::NotFound {
                service: self.service.clone(),
                key: key.to_string(),
            })
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let mut map = self.inner.lock().map_err(|_| SecretStoreError::Lock)?;
        map.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let mut map = self.inner.lock().map_err(|_| SecretStoreError::Lock)?;
        map.remove(key);
        Ok(())
    }
}

/// Production secret store backed by the OS keyring.
pub struct KeyringSecretStore {
    /// Service name registered with the OS keyring (e.g. "agentic").
    service: String,
}

impl KeyringSecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|source| SecretStoreError::Backend { source })?;
        match entry.get_password() {
            Ok(s) => Ok(s),
            Err(keyring::Error::NoEntry) => Err(SecretStoreError::NotFound {
                service: self.service.clone(),
                key: key.to_string(),
            }),
            Err(source) => Err(SecretStoreError::Backend { source }),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|source| SecretStoreError::Backend { source })?;
        entry
            .set_password(value)
            .map_err(|source| SecretStoreError::Backend { source })
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|source| SecretStoreError::Backend { source })?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // idempotent: silent no-op
            Err(source) => Err(SecretStoreError::Backend { source }),
        }
    }
}
