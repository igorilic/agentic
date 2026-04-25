use base64::Engine;

/// PKCE method per RFC 7636. We only emit S256 (the recommended method).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceChallenge {
    /// 43–128 chars, [A-Z][a-z][0-9]-._~
    pub verifier: String,
    /// base64url(sha256(verifier)) — 43 chars unpadded
    pub challenge: String,
}

impl PkceChallenge {
    /// Method string for the OAuth `code_challenge_method` parameter.
    pub const METHOD: &'static str = "S256";

    /// Generate a fresh verifier + derived challenge using OsRng.
    pub fn generate() -> Self {
        let verifier = generate_verifier(96); // 96 bytes → 128 base64url chars
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

fn generate_verifier(byte_count: usize) -> String {
    use rand::RngCore;
    let mut bytes = vec![0u8; byte_count];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

pub(crate) fn derive_challenge(verifier: &str) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}
