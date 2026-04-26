use base64::Engine;
use zeroize::ZeroizeOnDrop;

/// PKCE method per RFC 7636. We only emit S256 (the recommended method).
// #46 — verifier is zeroized on drop; challenge is public hash so skipped
#[derive(Clone, PartialEq, Eq, ZeroizeOnDrop)]
pub struct PkceChallenge {
    /// Public. Send to the OAuth authorization endpoint.
    /// Derived hash — not secret. Skip zeroize.
    #[zeroize(skip)]
    pub challenge: String,

    /// Secret. NEVER log or expose. Send only to the OAuth token-exchange endpoint.
    /// Zeroed on drop.
    pub verifier: String,
}

impl std::fmt::Debug for PkceChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PkceChallenge")
            .field("verifier", &"[redacted]")
            .field("challenge", &self.challenge)
            .finish()
    }
}

impl PkceChallenge {
    /// Method string for the OAuth `code_challenge_method` parameter.
    pub const METHOD: &'static str = "S256";

    /// Generate a fresh verifier + derived challenge using OsRng.
    pub fn generate() -> Self {
        let verifier = generate_verifier(); // 96 bytes → 128 base64url chars
        let challenge = derive_challenge(&verifier);
        Self {
            verifier,
            challenge,
        }
    }
}

/// Generate a cryptographically random `state` parameter (128+ bits of entropy).
/// Returned as a base64url-encoded string with no padding.
pub fn generate_state() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32]; // 256 bits — well above the 128-bit floor
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// #45 — byte_count was an unused knob; inlined as a named constant
fn generate_verifier() -> String {
    use rand::RngCore;
    const VERIFIER_BYTE_COUNT: usize = 96;
    let mut bytes = [0u8; VERIFIER_BYTE_COUNT];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn derive_challenge(verifier: &str) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}
