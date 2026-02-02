//! PKCE (Proof Key for Code Exchange) utilities for OAuth 2.0
//!
//! This module provides functions to generate code verifiers and challenges
//! for secure OAuth 2.0 authorization code flows.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use sha2::Digest;

/// Generates a random code verifier for PKCE flow.
///
/// The code verifier is a cryptographically random string using the
/// unreserved characters [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~".
/// This implementation uses hex encoding for simplicity.
///
/// # Returns
/// A 128-character hex string (64 random bytes).
///
/// # Example
/// ```no_run
/// use pkce::generate_code_verifier;
///
/// let verifier = generate_code_verifier();
/// assert_eq!(verifier.len(), 128);
/// ```
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 64];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Generates a code challenge from a code verifier.
///
/// The code challenge is the base64url-encoded SHA-256 hash of the
/// code verifier, as specified in RFC 7636.
///
/// # Arguments
/// * `verifier` - The code verifier string
///
/// # Returns
/// A base64url-encoded string without padding.
///
/// # Example
/// ```no_run
/// use pkce::{generate_code_verifier, generate_code_challenge};
///
/// let verifier = generate_code_verifier();
/// let challenge = generate_code_challenge(&verifier);
/// assert!(!challenge.contains('='));
/// assert!(!challenge.contains('+'));
/// assert!(!challenge.contains('/'));
/// ```
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    
    STANDARD.encode(&result)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 128);
    }

    #[test]
    fn test_code_verifier_is_hex() {
        let verifier = generate_code_verifier();
        assert!(verifier.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_code_challenge_format() {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        
        // Should not contain padding
        assert!(!challenge.contains('='));
        // Should use base64url encoding
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        // Should contain only URL-safe characters
        assert!(challenge.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test_verifier_string";
        let challenge1 = generate_code_challenge(verifier);
        let challenge2 = generate_code_challenge(verifier);
        assert_eq!(challenge1, challenge2);
    }
}
