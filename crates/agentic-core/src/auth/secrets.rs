use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret not found: {service}/{key}")]
    NotFound { service: String, key: String },
    #[error("keyring backend error: {0}")]
    Backend(String),
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
#[derive(Debug, Default)]
pub struct MemSecretStore {
    inner: Mutex<HashMap<String, String>>,
}

impl MemSecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemSecretStore {
    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let map = self.inner.lock().map_err(|_| SecretStoreError::Lock)?;
        map.get(key)
            .cloned()
            .ok_or_else(|| SecretStoreError::NotFound {
                service: "mem".to_string(),
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
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        match entry.get_password() {
            Ok(s) => Ok(s),
            Err(keyring::Error::NoEntry) => Err(SecretStoreError::NotFound {
                service: self.service.clone(),
                key: key.to_string(),
            }),
            Err(e) => Err(SecretStoreError::Backend(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // idempotent: silent no-op
            Err(e) => Err(SecretStoreError::Backend(e.to_string())),
        }
    }
}
