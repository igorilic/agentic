//! Jira auth scheme smoke test for self-hosted servers.
//!
//! Probes 4 auth schemes against a real Jira instance to identify which one
//! the server accepts. Diagnoses 401 failures from the existing Basic
//! email+token scheme used by `JiraTicketSource`.
//!
//! Run via:
//!   cargo test -p agentic-core --test jira_auth_smoke -- --ignored --nocapture
//!
//! Required env vars:
//!   JIRA_URL          — e.g. https://jira.heidelbergcement.com
//!   JIRA_API_TOKEN    — API token or Personal Access Token
//!   JIRA_TEST_KEY     — a real issue key the user has read access to, e.g. AGT-1
//!
//! Optional env vars (at least one is needed for scheme A/B/C):
//!   JIRA_USER_EMAIL   — full email address, e.g. user@example.com
//!   JIRA_USERNAME     — bare username for self-hosted Jira, e.g. john.doe

use base64::Engine as _;

#[ignore = "live: probes a real Jira server. Run via:\n  cargo test -p agentic-core --test jira_auth_smoke -- --ignored --nocapture\nRequires JIRA_URL, JIRA_API_TOKEN, JIRA_TEST_KEY. Optionally JIRA_USER_EMAIL and/or JIRA_USERNAME."]
#[tokio::test]
async fn smoke_test_jira_auth_schemes() {
    // -- Required env vars ------------------------------------------------
    let base_url = match std::env::var("JIRA_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: JIRA_URL not set");
            return;
        }
    };
    let token = match std::env::var("JIRA_API_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: JIRA_API_TOKEN not set");
            return;
        }
    };
    let test_key = match std::env::var("JIRA_TEST_KEY") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: JIRA_TEST_KEY not set");
            return;
        }
    };

    // -- Optional env vars ------------------------------------------------
    let email = std::env::var("JIRA_USER_EMAIL").ok();
    let username = std::env::var("JIRA_USERNAME").ok();

    if email.is_none() && username.is_none() {
        eprintln!(
            "warning: neither JIRA_USER_EMAIL nor JIRA_USERNAME set — \
             only scheme D (Bearer) will be tested"
        );
    }

    // -- Build endpoint URL -----------------------------------------------
    let endpoint = format!(
        "{}/rest/api/2/issue/{}",
        base_url.trim_end_matches('/'),
        test_key
    );
    eprintln!("\nProbing endpoint: {endpoint}");

    let client = reqwest::Client::new();
    let mut successes: Vec<&'static str> = Vec::new();

    // -- Scheme A: Basic email+token --------------------------------------
    // This matches the current JiraTicketSource implementation.
    match &email {
        Some(em) => {
            eprintln!("\n=== Scheme A: Basic email+token ({em}) ===");
            let credential = format!("{em}:{token}");
            let b64 = base64::engine::general_purpose::STANDARD.encode(credential.as_bytes());
            probe(
                &client,
                &endpoint,
                format!("Basic {b64}"),
                "A: Basic email+token",
                &mut successes,
            )
            .await;
        }
        None => {
            eprintln!("\n=== Scheme A: Basic email+token — SKIPPED (no JIRA_USER_EMAIL) ===");
        }
    }

    // -- Scheme B: Basic localpart-of-email+token -------------------------
    // Self-hosted Jira often rejects the full email and accepts only the
    // local part (the portion before '@').
    match &email {
        Some(em) => {
            let localpart = em.split('@').next().unwrap_or(em.as_str());
            eprintln!("\n=== Scheme B: Basic localpart+token ({localpart}) ===");
            let credential = format!("{localpart}:{token}");
            let b64 = base64::engine::general_purpose::STANDARD.encode(credential.as_bytes());
            probe(
                &client,
                &endpoint,
                format!("Basic {b64}"),
                "B: Basic localpart+token",
                &mut successes,
            )
            .await;
        }
        None => {
            eprintln!("\n=== Scheme B: Basic localpart+token — SKIPPED (no JIRA_USER_EMAIL) ===");
        }
    }

    // -- Scheme C: Basic JIRA_USERNAME+token ------------------------------
    // Lets the user explicitly supply the username independent of the email.
    match &username {
        Some(u) => {
            eprintln!("\n=== Scheme C: Basic JIRA_USERNAME+token ({u}) ===");
            let credential = format!("{u}:{token}");
            let b64 = base64::engine::general_purpose::STANDARD.encode(credential.as_bytes());
            probe(
                &client,
                &endpoint,
                format!("Basic {b64}"),
                "C: Basic JIRA_USERNAME+token",
                &mut successes,
            )
            .await;
        }
        None => {
            eprintln!("\n=== Scheme C: Basic JIRA_USERNAME+token — SKIPPED (no JIRA_USERNAME) ===");
        }
    }

    // -- Scheme D: Bearer (Personal Access Token) -------------------------
    // Self-hosted Jira 8.14+ supports PATs via Bearer auth.
    eprintln!("\n=== Scheme D: Bearer (PAT) ===");
    probe(
        &client,
        &endpoint,
        format!("Bearer {token}"),
        "D: Bearer (PAT)",
        &mut successes,
    )
    .await;

    // -- Summary ----------------------------------------------------------
    eprintln!("\n=== SUMMARY ===");
    if successes.is_empty() {
        eprintln!(
            "NO SCHEME WORKED. The token may be invalid, the issue key may \
             not exist or be inaccessible, or the server may require a \
             different scheme not covered here."
        );
    } else {
        eprintln!("Successful schemes:");
        for s in &successes {
            eprintln!("  - {s}");
        }
    }

    assert!(
        !successes.is_empty(),
        "no auth scheme returned 2xx — see --nocapture output above for details"
    );
}

/// Send one GET request with the supplied `Authorization` header value,
/// print the result, and push `label` onto `successes` if the response was 2xx.
async fn probe(
    client: &reqwest::Client,
    endpoint: &str,
    auth_header: String,
    label: &'static str,
    successes: &mut Vec<&'static str>,
) {
    match client
        .get(endpoint)
        .header("Authorization", auth_header)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            eprintln!("  status : {status}");
            eprintln!("  body[..200]: {preview}");
            if status.is_success() {
                successes.push(label);
            }
        }
        Err(e) => {
            eprintln!("  network error: {e}");
        }
    }
}
