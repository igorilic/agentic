use agentic_core::auth::{MemSecretStore, SecretStore, SecretStoreError};

#[test]
fn mem_set_then_get_returns_value() {
    let store = MemSecretStore::new();
    store.set("github_token", "ghp_xxx").unwrap();
    assert_eq!(store.get("github_token").unwrap(), "ghp_xxx");
}

#[test]
fn mem_get_missing_returns_not_found() {
    let store = MemSecretStore::new();
    let err = store.get("nope").unwrap_err();
    assert!(matches!(err, SecretStoreError::NotFound { .. }));
}

#[test]
fn mem_set_overwrites_existing_value() {
    let store = MemSecretStore::new();
    store.set("k", "v1").unwrap();
    store.set("k", "v2").unwrap();
    assert_eq!(store.get("k").unwrap(), "v2");
}

#[test]
fn mem_delete_removes_entry() {
    let store = MemSecretStore::new();
    store.set("k", "v").unwrap();
    store.delete("k").unwrap();
    assert!(matches!(
        store.get("k"),
        Err(SecretStoreError::NotFound { .. })
    ));
}

#[test]
fn mem_delete_missing_key_is_silent_no_op() {
    let store = MemSecretStore::new();
    // Per contract: deleting a missing key returns Ok(()).
    store.delete("never_existed").unwrap();
}

#[test]
fn mem_store_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MemSecretStore>();
}
