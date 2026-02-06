//! PKCE (Proof Key for Code Exchange) implementation.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a cryptographically random code verifier.
///
/// Returns a 43-128 character string using unreserved characters [A-Z, a-z, 0-9, "-", ".", "_", "~"].
pub fn generate_verifier() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    const VERIFIER_LENGTH: usize = 64; // Use 64 chars (mid-range of 43-128)

    let mut rng = rand::thread_rng();
    (0..VERIFIER_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate the code challenge from a verifier using S256 method.
///
/// Returns BASE64URL(SHA256(verifier)) with no padding.
pub fn generate_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_length() {
        let verifier = generate_verifier();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c)));
    }

    #[test]
    fn test_challenge_deterministic() {
        let verifier = "test_verifier_12345";
        let challenge1 = generate_challenge(verifier);
        let challenge2 = generate_challenge(verifier);
        assert_eq!(challenge1, challenge2);
        assert!(!challenge1.contains('='));
    }
}
