//! Tauri auth IPC.
//!
//! - `list_auth_accounts` — read-only list for the Settings panel.
//! - `delete_auth_account` — remove a row + its keychain entry.
//! - `connect_github_via_gh` — zero-config delegate flow using the `gh` CLI.

use std::path::PathBuf;
use std::sync::Arc;

use agentic_core::auth::gh_delegate::GhDelegate;
use agentic_core::auth::secrets::SecretStore;
use agentic_core::db::auth::{AuthAccount, AuthRepo};
use serde::Serialize;
use tauri::State;

/// Per-app shared auth state. Constructed once at startup and registered
/// with `app.manage`.
pub struct AuthState {
    pub repo: AuthRepo,
    pub secrets: Arc<dyn SecretStore>,
    /// Path to the `gh` binary used by `connect_github_via_gh`. Defaults
    /// to `gh` (PATH lookup) in production; tests inject a fake script.
    pub gh_binary: PathBuf,
}

const GITHUB_HOST: &str = "github.com";

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
