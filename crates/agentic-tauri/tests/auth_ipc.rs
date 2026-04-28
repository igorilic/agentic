#![cfg(test)]

use std::sync::{Arc, Mutex};

use agentic_core::Db;
use agentic_core::auth::SecretStore;
use agentic_core::auth::secrets::MemSecretStore;
use agentic_core::db::auth::{AuthAccount, AuthRepo};
use agentic_tauri::commands::auth::{
    AuthState, UrlOpener, connect_github, connect_github_via_gh, delete_auth_account,
    list_auth_accounts,
};
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test opener that records every URL passed to it instead of launching a
/// browser. Tests pull the URL back out to drive the loopback callback.
#[derive(Default, Clone)]
struct RecordingOpener {
    urls: Arc<Mutex<Vec<String>>>,
}

impl RecordingOpener {
    fn new() -> Self {
        Self::default()
    }

    fn last(&self) -> Option<String> {
        self.urls.lock().unwrap().last().cloned()
    }
}

impl UrlOpener for RecordingOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        self.urls.lock().unwrap().push(url.to_string());
        Ok(())
    }
}

fn build_app(
    db: &Db,
    secrets: Arc<MemSecretStore>,
    opener: RecordingOpener,
    github_base_url: String,
) -> tauri::App<tauri::test::MockRuntime> {
    build_app_with_gh(
        db,
        secrets,
        opener,
        github_base_url,
        std::path::PathBuf::from("gh"),
    )
}

fn build_app_with_gh(
    db: &Db,
    secrets: Arc<MemSecretStore>,
    opener: RecordingOpener,
    github_base_url: String,
    gh_binary: std::path::PathBuf,
) -> tauri::App<tauri::test::MockRuntime> {
    let state = AuthState {
        repo: AuthRepo::new(db),
        secrets,
        opener: Arc::new(opener),
        github_base_url,
        callback_timeout_secs: 5,
        gh_binary,
    };
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::auth::list_auth_accounts,
            agentic_tauri::commands::auth::delete_auth_account,
            agentic_tauri::commands::auth::connect_github,
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
    let app = build_app(
        &db,
        secrets,
        RecordingOpener::new(),
        "https://example".to_string(),
    );

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

    let app = build_app(
        &db,
        secrets,
        RecordingOpener::new(),
        "https://example".to_string(),
    );

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

    let app = build_app(
        &db,
        secrets.clone(),
        RecordingOpener::new(),
        "https://example".to_string(),
    );

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
    let app = build_app_with_gh(
        &db,
        secrets.clone(),
        RecordingOpener::new(),
        "https://example".to_string(),
        fake_gh.clone(),
    );

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
    let app = build_app_with_gh(
        &db,
        secrets,
        RecordingOpener::new(),
        "https://example".to_string(),
        fake_gh,
    );

    let err = connect_github_via_gh(app.state::<AuthState>())
        .await
        .expect_err("must error when no gh session");
    // Error message should mention `gh auth login` so users know what to do.
    assert!(
        err.contains("gh auth login") || err.to_lowercase().contains("session"),
        "error should be actionable: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn connect_github_full_flow_persists_account_and_secret() {
    // Mock GitHub OAuth token endpoint.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_test_token",
            "token_type": "bearer",
            "scope": "repo,read:user",
            "expires_in": 28800
        })))
        .mount(&server)
        .await;

    let db = Db::open_in_memory().unwrap();
    let secrets = Arc::new(MemSecretStore::with_service("test"));
    let opener = RecordingOpener::new();

    let app = build_app(&db, secrets.clone(), opener.clone(), server.uri());

    // Spawn connect_github — it'll suspend waiting for the loopback callback.
    let app_for_task = app.handle().clone();
    let connect_task = tokio::spawn(async move {
        let state = app_for_task.state::<AuthState>();
        connect_github(state, "Iv1.test_client".to_string()).await
    });

    // Wait for the opener to capture the authorize URL — this means the
    // loopback listener is up and we know the redirect_uri.
    let authorize_url = poll_for_url(&opener).await;

    // Parse the authorize URL to get the redirect_uri + state.
    let (redirect_uri, state_param) = parse_authorize_url(&authorize_url);

    // Drive the callback: pretend GitHub redirected the user back.
    let callback = format!("{redirect_uri}?code=auth_code_xyz&state={state_param}");
    let resp = reqwest::get(&callback).await.expect("callback GET");
    assert!(resp.status().is_success(), "callback should succeed");

    let account = connect_task
        .await
        .expect("task")
        .expect("connect_github should succeed");

    // Account row inserted.
    assert_eq!(account.provider, "github");
    assert_eq!(account.host, "github.com");
    assert_eq!(account.id, "github:github.com");

    // Token stored in keychain under the same id.
    let stored = secrets.get(&account.id).expect("secret should be set");
    assert_eq!(stored, "gho_test_token");
}

// ─── helpers ──────────────────────────────────────────────────────────────────

async fn poll_for_url(opener: &RecordingOpener) -> String {
    for _ in 0..50 {
        if let Some(u) = opener.last() {
            return u;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("opener never received an authorize URL within 1s");
}

fn parse_authorize_url(url: &str) -> (String, String) {
    let parsed = url::Url::parse(url).expect("authorize URL must parse");
    let mut redirect_uri = String::new();
    let mut state = String::new();
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "redirect_uri" => redirect_uri = v.to_string(),
            "state" => state = v.to_string(),
            _ => {}
        }
    }
    assert!(
        !redirect_uri.is_empty(),
        "authorize URL missing redirect_uri"
    );
    assert!(!state.is_empty(), "authorize URL missing state");
    (redirect_uri, state)
}
