#![cfg(test)]

use std::sync::Arc;

use agentic_core::Db;
use agentic_core::auth::SecretStore;
use agentic_core::auth::secrets::MemSecretStore;
use agentic_core::db::auth::{AuthAccount, AuthRepo};
use agentic_tauri::commands::auth::{
    AuthState, connect_github_via_gh, delete_auth_account, list_auth_accounts,
};
use tauri::Manager;

fn build_app(
    db: &Db,
    secrets: Arc<MemSecretStore>,
    gh_binary: std::path::PathBuf,
) -> tauri::App<tauri::test::MockRuntime> {
    use tauri::test::{mock_builder, mock_context, noop_assets};
    let state = AuthState {
        repo: AuthRepo::new(db),
        secrets,
        gh_binary,
    };
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::auth::list_auth_accounts,
            agentic_tauri::commands::auth::delete_auth_account,
            agentic_tauri::commands::auth::connect_github_via_gh,
        ])
        .manage(state)
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

#[tokio::test(flavor = "multi_thread")]
async fn list_auth_accounts_returns_empty_initially() {
    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));
    let app = build_app(&db, secrets, std::path::PathBuf::from("gh"));

    let rows = list_auth_accounts(app.state::<AuthState>())
        .await
        .expect("list");
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn list_auth_accounts_returns_inserted_rows() {
    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));
    AuthRepo::new(&db)
        .insert(&AuthAccount {
            id: "github:github.com".to_string(),
            provider: "github".to_string(),
            host: "github.com".to_string(),
            username: Some("octocat".to_string()),
            client_id: Some("Iv1.abc".to_string()),
            token_expires_at: None,
            created_at: 100,
            last_used_at: None,
        })
        .unwrap();

    let app = build_app(&db, secrets, std::path::PathBuf::from("gh"));

    let rows = list_auth_accounts(app.state::<AuthState>())
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "github:github.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_auth_account_removes_db_row_and_keychain_entry() {
    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));

    AuthRepo::new(&db)
        .insert(&AuthAccount {
            id: "github:github.com".to_string(),
            provider: "github".to_string(),
            host: "github.com".to_string(),
            username: None,
            client_id: None,
            token_expires_at: None,
            created_at: 1,
            last_used_at: None,
        })
        .unwrap();
    secrets.set("github:github.com", "gho_secret").unwrap();

    let app = build_app(&db, secrets.clone(), std::path::PathBuf::from("gh"));

    let deleted = delete_auth_account(app.state::<AuthState>(), "github:github.com".to_string())
        .await
        .expect("delete");
    assert!(deleted);

    // DB row gone.
    assert!(AuthRepo::new(&db).list().unwrap().is_empty());
    // Keychain entry gone.
    assert!(secrets.get("github:github.com").is_err());

    // Idempotent: second delete returns false.
    let again = delete_auth_account(app.state::<AuthState>(), "github:github.com".to_string())
        .await
        .expect("delete");
    assert!(!again);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn connect_github_via_gh_imports_token_and_persists_account() {
    use std::os::unix::fs::PermissionsExt;

    // Fake `gh` script: `auth status` → exit 0; `auth token` → print fake token.
    let tmp = tempfile::tempdir().unwrap();
    let fake_gh = tmp.path().join("fake-gh.sh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\n\
         case \"$1 $2\" in\n\
           'auth status') exit 0 ;;\n\
           'auth token') echo 'gho_from_gh_cli'; exit 0 ;;\n\
           *) exit 99 ;;\n\
         esac\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_gh, std::fs::Permissions::from_mode(0o755)).unwrap();

    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));
    let app = build_app(&db, secrets.clone(), fake_gh.clone());

    let account = connect_github_via_gh(app.state::<AuthState>())
        .await
        .expect("connect_github_via_gh");

    assert_eq!(account.id, "github:github.com");
    assert_eq!(account.provider, "github");
    assert_eq!(account.host, "github.com");

    // DB row inserted.
    let rows = AuthRepo::new(&db).list().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "github:github.com");

    // Token stored in keychain under the account id.
    assert_eq!(secrets.get(&account.id).unwrap(), "gho_from_gh_cli");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn connect_github_via_gh_returns_actionable_error_when_no_session() {
    use std::os::unix::fs::PermissionsExt;

    // Fake `gh` script that always reports no session.
    let tmp = tempfile::tempdir().unwrap();
    let fake_gh = tmp.path().join("fake-gh.sh");
    std::fs::write(&fake_gh, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&fake_gh, std::fs::Permissions::from_mode(0o755)).unwrap();

    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));
    let app = build_app(&db, secrets, fake_gh);

    let err = connect_github_via_gh(app.state::<AuthState>())
        .await
        .expect_err("must error when no gh session");
    // Error message should mention `gh auth login` so users know what to do.
    assert!(
        err.contains("gh auth login") || err.to_lowercase().contains("session"),
        "error should be actionable: {err}"
    );
}
