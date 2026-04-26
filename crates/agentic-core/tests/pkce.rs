use agentic_core::auth::pkce::{PkceChallenge, generate_state};

#[test]
fn verifier_length_within_rfc_7636_bounds() {
    let pkce = PkceChallenge::generate();
    let len = pkce.verifier.chars().count();
    assert!(len >= 43, "verifier too short: {len} chars");
    assert!(len <= 128, "verifier too long: {len} chars");
}

#[test]
fn verifier_uses_unreserved_alphabet() {
    let pkce = PkceChallenge::generate();
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~');
    assert!(
        pkce.verifier.chars().all(allowed),
        "verifier contains forbidden character: {}",
        pkce.verifier
    );
}

#[test]
fn challenge_is_base64url_sha256_of_verifier() {
    let pkce = PkceChallenge::generate();
    use base64::Engine;
    use sha2::Digest;
    let hash = sha2::Sha256::digest(pkce.verifier.as_bytes());
    let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);
    assert_eq!(pkce.challenge, expected);
}

#[test]
fn challenge_is_43_chars_no_padding() {
    let pkce = PkceChallenge::generate();
    assert_eq!(
        pkce.challenge.chars().count(),
        43,
        "S256 challenge is base64url(sha256) which is exactly 43 chars unpadded"
    );
    assert!(!pkce.challenge.contains('='));
}

#[test]
fn state_meets_minimum_entropy_threshold() {
    let s = generate_state();
    assert!(s.len() >= 22, "state too short: {} chars", s.len());
    assert!(!s.contains('='));
}

#[test]
fn state_uses_base64url_alphabet() {
    let s = generate_state();
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    assert!(s.chars().all(allowed));
}

#[test]
fn method_constant_is_s256() {
    assert_eq!(PkceChallenge::METHOD, "S256");
}

proptest::proptest! {
    /// Drive PkceChallenge::generate across 1000 proptest iterations.
    /// The seed is unused (generate uses OsRng); this just smoke-tests
    /// that generation never panics and the challenge is always
    /// the spec'd 43 chars. Distinctness is covered by the separate
    /// `one_thousand_verifiers_yield_one_thousand_distinct_challenges` test.
    #[test]
    fn challenge_is_always_43_chars_across_iterations(_seed in 0u64..1000) {
        let pkce = PkceChallenge::generate();
        proptest::prop_assert_eq!(pkce.challenge.len(), 43);
    }
}

#[test]
fn debug_impl_redacts_verifier() {
    let pkce = PkceChallenge::generate();
    let dbg = format!("{pkce:?}");
    assert!(
        dbg.contains("[redacted]"),
        "verifier should be redacted in Debug, got: {dbg}"
    );
    assert!(
        !dbg.contains(&pkce.verifier),
        "raw verifier leaked in Debug output: {dbg}"
    );
    assert!(
        dbg.contains(&pkce.challenge),
        "challenge should still appear in Debug output"
    );
}

#[test]
fn one_thousand_verifiers_yield_one_thousand_distinct_challenges() {
    use std::collections::HashSet;
    let challenges: HashSet<String> = (0..1000)
        .map(|_| PkceChallenge::generate().challenge)
        .collect();
    assert_eq!(
        challenges.len(),
        1000,
        "duplicate challenges detected — RNG broken or output truncated"
    );
}

// #46 — PkceChallenge implements ZeroizeOnDrop (compile-time check)
#[test]
fn pkce_challenge_zeroizes_on_drop() {
    fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}
    assert_zeroize_on_drop::<PkceChallenge>();
}
