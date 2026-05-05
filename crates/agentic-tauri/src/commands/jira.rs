//! Tauri IPC for fetching a Jira ticket by key.
//!
//! Reads `JIRA_URL`, `JIRA_API_TOKEN` (always required) and optionally
//! `JIRA_USER_EMAIL`, `JIRA_AC_FIELD_ID`, `JIRA_AUTH_SCHEME` **per call** so
//! the user can export them in their shell and re-click "Pull" without
//! restarting the binary.
//!
//! The base URL is the bare host (e.g. `https://jira.heidelbergcement.com`).
//! `/rest/api/2` is appended here (Server / Data Centre compatible).
//!
//! Auth scheme:
//! - `bearer` (default): `Authorization: Bearer <JIRA_API_TOKEN>`.
//!   Used by self-hosted Jira Server / Data Centre with PATs.
//! - `basic`: `Authorization: Basic base64(<JIRA_USER_EMAIL>:<JIRA_API_TOKEN>)`.
//!   Used by Atlassian Cloud. Requires `JIRA_USER_EMAIL` to be set.
//!
//! Set `JIRA_AUTH_SCHEME=basic` (case-insensitive) to switch to Basic auth.

use agentic_core::{
    JiraAuth, JiraTicketSource, TicketRef,
    events::TicketKind,
    ticket_sources::{Ticket, TicketSource},
};

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, Clone, Debug)]
pub struct JiraTicketDto {
    pub key: String,
    pub title: String,
    pub body: String,
    pub ac: Option<String>,
}

// ---------------------------------------------------------------------------
// Pure helpers — testable without Tauri runtime
// ---------------------------------------------------------------------------

/// Return the names of any required Jira environment variables that are not
/// currently set. `JIRA_AC_FIELD_ID` is optional and is never listed.
/// `JIRA_USER_EMAIL` is required only when `JIRA_AUTH_SCHEME=basic`.
///
/// Reserved for future direct-from-IPC use (current Tauri command does the
/// per-call read inline). Kept as a named API so a follow-up "show config
/// status" command can reuse it without re-implementing the env-var list.
#[allow(dead_code)]
pub(crate) fn missing_env_vars() -> Vec<&'static str> {
    let scheme = std::env::var("JIRA_AUTH_SCHEME")
        .unwrap_or_default()
        .to_lowercase();
    let mut required: Vec<&'static str> = vec!["JIRA_URL", "JIRA_API_TOKEN"];
    if scheme == "basic" {
        required.push("JIRA_USER_EMAIL");
    }
    required
        .iter()
        .filter(|name| std::env::var(name).is_err())
        .copied()
        .collect()
}

/// Strip any trailing slash from `jira_url`, then append `/rest/api/2`.
///
/// If the input already ends with `/rest/api/2` or `/rest/api/3`, strip that
/// suffix first and then append `/rest/api/2` so the result is always
/// canonicalised to v2 (Server / Data Centre compatible).
pub(crate) fn build_base_url(jira_url: &str) -> String {
    let trimmed = jira_url.trim_end_matches('/');
    let canonical = trimmed
        .strip_suffix("/rest/api/2")
        .or_else(|| trimmed.strip_suffix("/rest/api/3"))
        .unwrap_or(trimmed);
    format!("{canonical}/rest/api/2")
}

/// Validate that `key` matches `^[A-Z][A-Z0-9]+-\d+$`.
/// Error message: `"invalid ticket key: \"<key>\""`.
pub(crate) fn validate_key(key: &str) -> Result<(), String> {
    let err = || format!("invalid ticket key: \"{key}\"");
    let (project, num) = key.split_once('-').ok_or_else(err)?;
    if project.len() < 2 || num.is_empty() {
        return Err(err());
    }
    let mut chars = project.chars();
    let first = chars.next().ok_or_else(err)?;
    if !first.is_ascii_uppercase() {
        return Err(err());
    }
    if !chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return Err(err());
    }
    if !num.chars().all(|c| c.is_ascii_digit()) {
        return Err(err());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Inner function — injected fetcher enables unit tests without HTTP
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_jira_ticket_inner<F>(
    fetcher: F,
    env_url: Option<String>,
    env_email: Option<String>,
    env_token: Option<String>,
    env_ac_field: Option<String>,
    env_auth_scheme: Option<String>,
    key: String,
) -> Result<JiraTicketDto, String>
where
    F: AsyncFetcher,
{
    // 1. Validate key format BEFORE consulting env so a malformed key always
    //    returns the same error regardless of environment state.
    validate_key(&key)?;

    // 2. Determine auth scheme first so we know which env vars are required.
    let scheme = env_auth_scheme
        .as_deref()
        .unwrap_or("bearer")
        .to_lowercase();

    // 3. Check required env vars; build a clear message listing every missing one.
    let mut missing: Vec<&'static str> = Vec::new();
    if env_url.is_none() {
        missing.push("JIRA_URL");
    }
    if env_token.is_none() {
        missing.push("JIRA_API_TOKEN");
    }
    // JIRA_USER_EMAIL is only required for basic auth.
    if scheme == "basic" && env_email.is_none() {
        missing.push("JIRA_USER_EMAIL");
    }
    if !missing.is_empty() {
        return Err(format!(
            "missing environment variables: {}",
            missing.join(", ")
        ));
    }

    let raw_url = env_url.expect("checked above");
    let token = env_token.expect("checked above");
    let base_url = build_base_url(&raw_url);

    // 4. Build JiraAuth from the scheme.
    let auth = match scheme.as_str() {
        "basic" => {
            let email = env_email
                .ok_or_else(|| "missing JIRA_USER_EMAIL (required for basic auth)".to_string())?;
            JiraAuth::Basic { email, token }
        }
        "bearer" | "" => JiraAuth::Bearer { token },
        other => {
            return Err(format!(
                "unknown JIRA_AUTH_SCHEME: {other:?} (expected 'bearer' or 'basic')"
            ));
        }
    };

    // 5. Fetch.
    let ticket = fetcher
        .fetch(base_url, auth, env_ac_field, key.clone())
        .await?;

    // 6. Map Ticket → DTO. Append the AC section to body when present.
    let body = match &ticket.ac_field {
        Some(ac) if !ac.is_empty() => {
            format!("{}\n\n## Acceptance Criteria\n{}", ticket.body, ac)
        }
        _ => ticket.body.clone(),
    };

    Ok(JiraTicketDto {
        key,
        title: ticket.title,
        body,
        ac: ticket.ac_field,
    })
}

/// Minimal async-fetcher abstraction so tests can inject a fake without HTTP.
#[async_trait::async_trait]
pub(crate) trait AsyncFetcher: Send + Sync {
    async fn fetch(
        &self,
        base_url: String,
        auth: JiraAuth,
        ac_field: Option<String>,
        key: String,
    ) -> Result<Ticket, String>;
}

/// Production implementation — delegates to `JiraTicketSource`.
pub(crate) struct LiveFetcher;

#[async_trait::async_trait]
impl AsyncFetcher for LiveFetcher {
    async fn fetch(
        &self,
        base_url: String,
        auth: JiraAuth,
        ac_field: Option<String>,
        key: String,
    ) -> Result<Ticket, String> {
        let source = JiraTicketSource::new(base_url, auth, ac_field);
        let ticket_ref = TicketRef {
            kind: TicketKind::Jira,
            reference: key,
            title: None,
        };
        source.fetch(&ticket_ref).await.map_err(|e| format!("{e}"))
    }
}

// ---------------------------------------------------------------------------
// Tauri command — thin wrapper around the inner function
// ---------------------------------------------------------------------------

/// Pull a Jira ticket by key and return a flat DTO.
///
/// Env vars read at call time:
/// - `JIRA_URL` (required): bare host, e.g. `https://jira.example.com`.
/// - `JIRA_API_TOKEN` (required): PAT or API token.
/// - `JIRA_AUTH_SCHEME` (optional, default `bearer`): `bearer` or `basic`.
/// - `JIRA_USER_EMAIL` (required only when `JIRA_AUTH_SCHEME=basic`).
/// - `JIRA_AC_FIELD_ID` (optional): custom field id, e.g. `customfield_10100`.
#[tauri::command]
pub async fn fetch_jira_ticket(key: String) -> Result<JiraTicketDto, String> {
    let env_url = std::env::var("JIRA_URL").ok();
    let env_email = std::env::var("JIRA_USER_EMAIL").ok();
    let env_token = std::env::var("JIRA_API_TOKEN").ok();
    let env_ac_field = std::env::var("JIRA_AC_FIELD_ID").ok();
    let env_auth_scheme = std::env::var("JIRA_AUTH_SCHEME").ok();

    fetch_jira_ticket_inner(
        LiveFetcher,
        env_url,
        env_email,
        env_token,
        env_ac_field,
        env_auth_scheme,
        key,
    )
    .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use agentic_core::ticket_sources::Ticket;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_ticket(title: &str, body: &str, ac: Option<&str>) -> Ticket {
        Ticket {
            title: title.to_string(),
            body: body.to_string(),
            comments: vec![],
            ac_field: ac.map(str::to_string),
            url: None,
        }
    }

    struct FakeFetcher {
        result: Result<Ticket, String>,
    }

    #[async_trait::async_trait]
    impl AsyncFetcher for FakeFetcher {
        async fn fetch(
            &self,
            _base_url: String,
            _auth: JiraAuth,
            _ac_field: Option<String>,
            _key: String,
        ) -> Result<Ticket, String> {
            self.result.clone()
        }
    }

    /// Helper: produces env-var tuple (url, email, token, ac_field, auth_scheme).
    fn bearer_env(
        url: &str,
        token: &str,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            Some(url.to_string()),
            None, // no email needed for bearer
            Some(token.to_string()),
            None,
            None, // defaults to bearer
        )
    }

    fn basic_env(
        url: &str,
        email: &str,
        token: &str,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            Some(url.to_string()),
            Some(email.to_string()),
            Some(token.to_string()),
            None,
            Some("basic".to_string()),
        )
    }

    // -----------------------------------------------------------------------
    // missing_env_vars
    // -----------------------------------------------------------------------

    #[test]
    fn missing_env_vars_returns_unset_names_for_bearer() {
        temp_env::with_vars(
            [
                ("JIRA_URL", None::<&str>),
                ("JIRA_USER_EMAIL", None::<&str>),
                ("JIRA_API_TOKEN", None::<&str>),
                ("JIRA_AUTH_SCHEME", None::<&str>),
            ],
            || {
                let missing = missing_env_vars();
                assert!(
                    missing.contains(&"JIRA_URL"),
                    "expected JIRA_URL in {missing:?}"
                );
                assert!(
                    missing.contains(&"JIRA_API_TOKEN"),
                    "expected JIRA_API_TOKEN in {missing:?}"
                );
                // Bearer mode: JIRA_USER_EMAIL is NOT required
                assert!(
                    !missing.contains(&"JIRA_USER_EMAIL"),
                    "JIRA_USER_EMAIL should not be required for bearer mode, got {missing:?}"
                );
            },
        );
    }

    #[test]
    fn missing_env_vars_includes_email_for_basic_scheme() {
        temp_env::with_vars(
            [
                ("JIRA_URL", None::<&str>),
                ("JIRA_USER_EMAIL", None::<&str>),
                ("JIRA_API_TOKEN", None::<&str>),
                ("JIRA_AUTH_SCHEME", Some("basic")),
            ],
            || {
                let missing = missing_env_vars();
                assert!(
                    missing.contains(&"JIRA_USER_EMAIL"),
                    "expected JIRA_USER_EMAIL in {missing:?} for basic scheme"
                );
            },
        );
    }

    #[test]
    fn missing_env_vars_returns_empty_when_all_set_bearer() {
        temp_env::with_vars(
            [
                ("JIRA_URL", Some("https://jira.example.com")),
                ("JIRA_USER_EMAIL", None::<&str>),
                ("JIRA_API_TOKEN", Some("tok")),
                ("JIRA_AUTH_SCHEME", None::<&str>),
            ],
            || {
                let missing = missing_env_vars();
                assert!(missing.is_empty(), "expected empty, got {missing:?}");
            },
        );
    }

    // -----------------------------------------------------------------------
    // build_base_url
    // -----------------------------------------------------------------------

    #[test]
    fn build_base_url_appends_v2() {
        assert_eq!(
            build_base_url("https://acme.atlassian.net"),
            "https://acme.atlassian.net/rest/api/2"
        );
    }

    #[test]
    fn build_base_url_strips_trailing_slash() {
        assert_eq!(
            build_base_url("https://acme.atlassian.net/"),
            "https://acme.atlassian.net/rest/api/2"
        );
    }

    #[test]
    fn build_base_url_canonicalizes_v3_to_v2() {
        // If the user accidentally passes a v3 URL, we normalize to v2
        // (the Server/Data Centre compatible version).
        assert_eq!(
            build_base_url("https://acme.atlassian.net/rest/api/3"),
            "https://acme.atlassian.net/rest/api/2"
        );
    }

    #[test]
    fn build_base_url_idempotent_on_v2() {
        assert_eq!(
            build_base_url("https://acme.atlassian.net/rest/api/2"),
            "https://acme.atlassian.net/rest/api/2"
        );
    }

    #[test]
    fn build_base_url_strips_trailing_slash_from_host_with_slash() {
        // Matches the user-amendment test: JIRA_URL=https://jira.example.com/ (trailing slash)
        assert_eq!(
            build_base_url("https://jira.example.com/"),
            "https://jira.example.com/rest/api/2"
        );
    }

    // -----------------------------------------------------------------------
    // validate_key
    // -----------------------------------------------------------------------

    #[test]
    fn validate_key_accepts_valid_keys() {
        assert!(validate_key("PROJ-1").is_ok(), "PROJ-1 should be valid");
        assert!(validate_key("ABC-999").is_ok(), "ABC-999 should be valid");
        assert!(validate_key("XY1-42").is_ok(), "XY1-42 should be valid");
    }

    #[test]
    fn validate_key_rejects_empty() {
        let err = validate_key("").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    #[test]
    fn validate_key_rejects_lowercase_project() {
        let err = validate_key("proj-1").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    #[test]
    fn validate_key_rejects_missing_number() {
        let err = validate_key("PROJ-").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    #[test]
    fn validate_key_rejects_leading_dash() {
        let err = validate_key("-1").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    #[test]
    fn validate_key_rejects_alpha_number() {
        let err = validate_key("PROJ-abc").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    #[test]
    fn validate_key_rejects_no_dash() {
        let err = validate_key("PROJ").unwrap_err();
        assert!(err.contains("invalid ticket key"), "got: {err}");
    }

    // -----------------------------------------------------------------------
    // fetch_jira_ticket_inner — DTO mapping
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn fetch_jira_ticket_returns_dto_for_valid_key() {
        let ticket = make_ticket("Fix the bug", "Some description", None);
        let fetcher = FakeFetcher { result: Ok(ticket) };
        let (url, email, token, ac, scheme) =
            bearer_env("https://jira.example.com", "tok");

        let dto = fetch_jira_ticket_inner(fetcher, url, email, token, ac, scheme, "PROJ-1".to_string())
            .await
            .expect("should succeed");

        assert_eq!(dto.key, "PROJ-1");
        assert_eq!(dto.title, "Fix the bug");
        assert_eq!(dto.body, "Some description");
        assert_eq!(dto.ac, None);
    }

    #[tokio::test]
    async fn fetch_jira_ticket_appends_ac_to_body_when_present() {
        let ticket = make_ticket("Title", "Description body", Some("Must work"));
        let fetcher = FakeFetcher { result: Ok(ticket) };
        let (url, email, token, ac, scheme) =
            bearer_env("https://jira.example.com", "tok");

        let dto = fetch_jira_ticket_inner(fetcher, url, email, token, ac, scheme, "ABC-10".to_string())
            .await
            .expect("should succeed");

        assert_eq!(dto.ac, Some("Must work".to_string()));
        assert!(
            dto.body.ends_with("\n\n## Acceptance Criteria\nMust work"),
            "body should end with AC section, got: {:?}",
            dto.body
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_omits_ac_section_when_absent() {
        let ticket = make_ticket("Title", "Raw description", None);
        let fetcher = FakeFetcher { result: Ok(ticket) };
        let (url, email, token, ac, scheme) =
            bearer_env("https://jira.example.com", "tok");

        let dto = fetch_jira_ticket_inner(fetcher, url, email, token, ac, scheme, "XY1-42".to_string())
            .await
            .expect("should succeed");

        assert_eq!(dto.body, "Raw description");
        assert!(
            !dto.body.contains("Acceptance Criteria"),
            "body should not contain AC section"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_rejects_invalid_key_format() {
        let fetcher = FakeFetcher {
            result: Ok(make_ticket("T", "B", None)),
        };
        let (url, email, token, ac, scheme) =
            bearer_env("https://jira.example.com", "tok");

        let err = fetch_jira_ticket_inner(fetcher, url, email, token, ac, scheme, "lowercase".to_string())
            .await
            .unwrap_err();

        assert!(
            err.contains("invalid ticket key"),
            "expected 'invalid ticket key' in error, got: {err}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_rejects_missing_url_and_token() {
        let fetcher = FakeFetcher {
            result: Ok(make_ticket("T", "B", None)),
        };

        // Pass None for JIRA_URL and JIRA_API_TOKEN (bearer mode — email not required).
        let err = fetch_jira_ticket_inner(
            fetcher,
            None, // JIRA_URL missing
            None, // JIRA_USER_EMAIL missing (not required for bearer)
            None, // JIRA_API_TOKEN missing
            None,
            None, // bearer by default
            "PROJ-1".to_string(),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("missing environment variables"),
            "expected 'missing environment variables' in error, got: {err}"
        );
        assert!(
            err.contains("JIRA_URL"),
            "expected JIRA_URL listed, got: {err}"
        );
        assert!(
            err.contains("JIRA_API_TOKEN"),
            "expected JIRA_API_TOKEN listed, got: {err}"
        );
        // Bearer mode: JIRA_USER_EMAIL should NOT be in the missing list
        assert!(
            !err.contains("JIRA_USER_EMAIL"),
            "JIRA_USER_EMAIL should not be listed for bearer mode, got: {err}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_rejects_basic_without_email() {
        let fetcher = FakeFetcher {
            result: Ok(make_ticket("T", "B", None)),
        };

        let err = fetch_jira_ticket_inner(
            fetcher,
            Some("https://jira.example.com".to_string()),
            None, // JIRA_USER_EMAIL missing
            Some("tok".to_string()),
            None,
            Some("basic".to_string()), // basic scheme requires email
            "PROJ-1".to_string(),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("JIRA_USER_EMAIL"),
            "expected JIRA_USER_EMAIL listed for basic auth error, got: {err}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_rejects_unknown_auth_scheme() {
        let fetcher = FakeFetcher {
            result: Ok(make_ticket("T", "B", None)),
        };

        let err = fetch_jira_ticket_inner(
            fetcher,
            Some("https://jira.example.com".to_string()),
            None,
            Some("tok".to_string()),
            None,
            Some("digest".to_string()), // unknown scheme
            "PROJ-1".to_string(),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("unknown JIRA_AUTH_SCHEME"),
            "expected 'unknown JIRA_AUTH_SCHEME' error, got: {err}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_uses_bearer_auth_by_default() {
        // Capture the JiraAuth passed to the fetcher and assert it's Bearer.
        struct CaptureAuthFetcher {
            captured_auth: std::sync::Arc<std::sync::Mutex<Option<String>>>,
        }

        #[async_trait::async_trait]
        impl AsyncFetcher for CaptureAuthFetcher {
            async fn fetch(
                &self,
                _base_url: String,
                auth: JiraAuth,
                _ac_field: Option<String>,
                _key: String,
            ) -> Result<Ticket, String> {
                let kind = match &auth {
                    JiraAuth::Bearer { .. } => "bearer".to_string(),
                    JiraAuth::Basic { .. } => "basic".to_string(),
                };
                *self.captured_auth.lock().unwrap() = Some(kind);
                Ok(Ticket {
                    title: "T".to_string(),
                    body: "B".to_string(),
                    comments: vec![],
                    ac_field: None,
                    url: None,
                })
            }
        }

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let fetcher = CaptureAuthFetcher {
            captured_auth: captured.clone(),
        };

        let _ = fetch_jira_ticket_inner(
            fetcher,
            Some("https://jira.example.com".to_string()),
            None, // no email — bearer doesn't need it
            Some("my-pat".to_string()),
            None,
            None, // no auth scheme → defaults to bearer
            "PROJ-1".to_string(),
        )
        .await
        .expect("should succeed");

        let auth_kind = captured.lock().unwrap().clone().expect("fetcher was called");
        assert_eq!(
            auth_kind, "bearer",
            "default auth scheme should be bearer, got: {auth_kind}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_uses_basic_when_scheme_is_basic() {
        struct CaptureAuthFetcher {
            captured_auth: std::sync::Arc<std::sync::Mutex<Option<String>>>,
        }

        #[async_trait::async_trait]
        impl AsyncFetcher for CaptureAuthFetcher {
            async fn fetch(
                &self,
                _base_url: String,
                auth: JiraAuth,
                _ac_field: Option<String>,
                _key: String,
            ) -> Result<Ticket, String> {
                let kind = match &auth {
                    JiraAuth::Bearer { .. } => "bearer".to_string(),
                    JiraAuth::Basic { .. } => "basic".to_string(),
                };
                *self.captured_auth.lock().unwrap() = Some(kind);
                Ok(Ticket {
                    title: "T".to_string(),
                    body: "B".to_string(),
                    comments: vec![],
                    ac_field: None,
                    url: None,
                })
            }
        }

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let fetcher = CaptureAuthFetcher {
            captured_auth: captured.clone(),
        };

        let _ = fetch_jira_ticket_inner(
            fetcher,
            Some("https://jira.example.com".to_string()),
            Some("user@example.com".to_string()),
            Some("tok".to_string()),
            None,
            Some("basic".to_string()),
            "PROJ-1".to_string(),
        )
        .await
        .expect("should succeed");

        let auth_kind = captured.lock().unwrap().clone().expect("fetcher was called");
        assert_eq!(
            auth_kind, "basic",
            "auth scheme should be basic when JIRA_AUTH_SCHEME=basic, got: {auth_kind}"
        );
    }

    #[tokio::test]
    async fn fetch_jira_ticket_strips_trailing_slash_from_url() {
        // The URL with trailing slash should produce /rest/api/2, not //rest/api/2.
        // We verify by checking the fetcher receives the correct base_url.
        struct CaptureFetcher {
            captured_url: std::sync::Arc<std::sync::Mutex<Option<String>>>,
        }

        #[async_trait::async_trait]
        impl AsyncFetcher for CaptureFetcher {
            async fn fetch(
                &self,
                base_url: String,
                _auth: JiraAuth,
                _ac_field: Option<String>,
                _key: String,
            ) -> Result<Ticket, String> {
                *self.captured_url.lock().unwrap() = Some(base_url);
                Ok(Ticket {
                    title: "T".to_string(),
                    body: "B".to_string(),
                    comments: vec![],
                    ac_field: None,
                    url: None,
                })
            }
        }

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let fetcher = CaptureFetcher {
            captured_url: captured.clone(),
        };

        let _ = fetch_jira_ticket_inner(
            fetcher,
            Some("https://jira.example.com/".to_string()),
            None,
            Some("tok".to_string()),
            None,
            None, // bearer by default
            "PROJ-1".to_string(),
        )
        .await;

        let url = captured
            .lock()
            .unwrap()
            .clone()
            .expect("fetcher was called");
        assert_eq!(
            url, "https://jira.example.com/rest/api/2",
            "trailing slash should be stripped before appending /rest/api/2"
        );
    }
}
