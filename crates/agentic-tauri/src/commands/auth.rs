//! Tauri auth IPC.
//!
//! - `list_auth_accounts` — read-only list for the Settings panel.
//! - `delete_auth_account` — remove a row + its keychain entry.
//! - `connect_github` — full OAuth flow:
//!     1. Generate PKCE + CSRF state.
//!     2. Start a loopback listener on an ephemeral port.
//!     3. Build the authorize URL.
//!     4. Open the URL in the user's default browser via [`UrlOpener`].
//!     5. Wait for the browser to redirect back to the loopback.
//!     6. Validate state (constant-time).
//!     7. Exchange the code for an access token via [`GithubOauthClient`].
//!     8. Store the token in the [`SecretStore`] keyed by the account id.
//!     9. Insert the metadata row.
//!
//! The OAuth client's base URL and the URL opener are both injected through
//! [`AuthState`] so integration tests can swap in a wiremock server and a
//! recording opener.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agentic_core::auth::gh_delegate::GhDelegate;
use agentic_core::auth::loopback::{self, LoopbackListener};
use agentic_core::auth::oauth_github::GithubOauthClient;
use agentic_core::auth::pkce::{PkceChallenge, generate_state};
use agentic_core::auth::secrets::SecretStore;
use agentic_core::auth::{AccessToken, validate_state};
use agentic_core::db::auth::{AuthAccount, AuthRepo};
use serde::Serialize;
use tauri::State;

/// Abstraction so tests can capture the URL we'd open instead of launching
/// a browser.
pub trait UrlOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
}

/// Production opener — uses the `webbrowser` crate, which dispatches to the
/// platform's default URL handler.
pub struct WebbrowserOpener;

impl UrlOpener for WebbrowserOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        webbrowser::open(url).map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Per-app shared auth state. Constructed once at startup and registered
/// with `app.manage`.
pub struct AuthState {
    pub repo: AuthRepo,
    pub secrets: Arc<dyn SecretStore>,
    pub opener: Arc<dyn UrlOpener>,
    /// Base URL for the OAuth provider. Production is `https://github.com`;
    /// integration tests inject a wiremock URI.
    pub github_base_url: String,
    /// How long to wait for the browser to redirect back before giving up.
    /// 5 minutes is reasonable for the user to complete login + 2FA.
    pub callback_timeout_secs: u64,
    /// Path to the `gh` binary used by `connect_github_via_gh`. Defaults
    /// to `gh` (PATH lookup) in production; tests inject a fake script.
    pub gh_binary: PathBuf,
}

const GITHUB_HOST: &str = "github.com";
const GITHUB_SCOPES: &str = "repo,read:user";

#[derive(Debug, Serialize)]
pub struct AuthAccountDto {
    #[serde(flatten)]
    pub account: AuthAccount,
}

// ─── list / delete ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_auth_accounts(state: State<'_, AuthState>) -> Result<Vec<AuthAccount>, String> {
    state.repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_auth_account(
    state: State<'_, AuthState>,
    account_id: String,
) -> Result<bool, String> {
    let removed = state.repo.delete(&account_id).map_err(|e| e.to_string())?;
    // Always attempt to delete the keychain entry — even if the row was
    // missing, we don't want a stale secret hanging around. SecretStore::
    // delete is documented as idempotent so this is safe.
    state
        .secrets
        .delete(&account_id)
        .map_err(|e| e.to_string())?;
    Ok(removed)
}

// ─── connect_github_via_gh — zero-config delegate flow ───────────────────────

/// Tauri command. Imports the user's existing `gh` CLI session into our
/// keychain and inserts an account row. This is the spec §15.4 "Always-On
/// Fallback: Delegate to existing CLI session" path — zero browser
/// interaction, zero OAuth-app registration, just one click.
///
/// Returns `Err` with an actionable message if `gh` is missing, not
/// authenticated, or returns an empty token. The frontend surfaces the
/// message verbatim — it always tells the user to run `gh auth login`.
#[tauri::command]
pub async fn connect_github_via_gh(state: State<'_, AuthState>) -> Result<AuthAccount, String> {
    let gh = GhDelegate::with_binary(&state.gh_binary);
    let account_id = format!("github:{GITHUB_HOST}");

    gh.import_token(state.secrets.as_ref(), &account_id)
        .await
        .map_err(|e| match e {
            agentic_core::auth::gh_delegate::GhDelegateError::NoExistingSession => {
                "no existing gh session — run `gh auth login` and try again".to_string()
            }
            agentic_core::auth::gh_delegate::GhDelegateError::GhNotAvailable(msg) => {
                format!("gh CLI not available: {msg}. Install it from https://cli.github.com/")
            }
            other => other.to_string(),
        })?;

    let now = unix_ms();
    let account = AuthAccount {
        id: account_id,
        provider: "github".to_string(),
        host: GITHUB_HOST.to_string(),
        username: None,
        client_id: None, // delegated session has no Agentic-owned client_id
        token_expires_at: None,
        created_at: now,
        last_used_at: None,
    };
    state.repo.insert(&account).map_err(|e| e.to_string())?;
    Ok(account)
}

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ─── connect_github ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn connect_github(
    state: State<'_, AuthState>,
    client_id: String,
) -> Result<AuthAccount, String> {
    let pkce = PkceChallenge::generate();
    let csrf_state = generate_state();

    // 1. Spin up loopback first — we need its ephemeral port for the
    //    redirect_uri inside the authorize URL.
    let timeout = Duration::from_secs(state.callback_timeout_secs);
    let mut listener: LoopbackListener = loopback::start(timeout)
        .await
        .map_err(|e| format!("start loopback: {e}"))?;
    let port = listener.port;
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // 2. Build the authorize URL the user's browser will visit. (We never
    //    POST anything from here — GitHub redirects the browser back to
    //    `redirect_uri` once they sign in.)
    let authorize_url = build_authorize_url(
        &state.github_base_url,
        &client_id,
        &redirect_uri,
        &pkce.challenge,
        &csrf_state,
    );

    // 3. Open the URL in the user's default browser.
    state.opener.open(&authorize_url)?;

    // 4. Wait for the callback. take_callback returns a JoinHandle; the
    //    inner Result is the loopback's "did the GET land" outcome.
    let callback = listener
        .take_callback()
        .await
        .map_err(|e| format!("loopback task: {e}"))?
        .map_err(|e| format!("loopback callback: {e}"))?;

    // 5. CSRF check before doing anything with the code.
    validate_state(&csrf_state, &callback.state).map_err(|e| e.to_string())?;

    // 6. Exchange code for token.
    let oauth = GithubOauthClient::new(state.github_base_url.clone(), client_id.clone(), None);
    let token: AccessToken = oauth
        .exchange_code(&callback.code, &pkce.verifier, &redirect_uri)
        .await
        .map_err(|e| format!("token exchange: {e}"))?;

    // 7. Persist: keychain (token) + DB (metadata).
    let account_id = format!("github:{GITHUB_HOST}");
    state
        .secrets
        .set(&account_id, &token.token)
        .map_err(|e| e.to_string())?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let account = AuthAccount {
        id: account_id,
        provider: "github".to_string(),
        host: GITHUB_HOST.to_string(),
        username: None,
        client_id: Some(client_id),
        token_expires_at: token.expires_at,
        created_at: now,
        last_used_at: None,
    };
    state.repo.insert(&account).map_err(|e| e.to_string())?;
    Ok(account)
}

fn build_authorize_url(
    base_url: &str,
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state_param: &str,
) -> String {
    // Manual encoding to avoid pulling in `url` as a runtime dep — these
    // are all already-safe characters except the redirect_uri's colons
    // and slashes which the URL parser on the GitHub side will tolerate
    // unencoded. To be safe, percent-encode the characters that need it.
    let redirect_encoded = percent_encode(redirect_uri);
    format!(
        "{base_url}/login/oauth/authorize?\
         client_id={client_id}\
         &redirect_uri={redirect_encoded}\
         &response_type=code\
         &scope={scope}\
         &state={state_param}\
         &code_challenge={code_challenge}\
         &code_challenge_method=S256",
        scope = percent_encode(GITHUB_SCOPES),
    )
}

/// Minimal RFC-3986 unreserved-character percent-encoder. Encodes everything
/// except `A-Z`, `a-z`, `0-9`, and `-_.~`.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let safe = b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~';
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
