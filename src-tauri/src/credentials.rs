//! Credential storage and retrieval module
//!
//! This module provides utilities for saving, loading, and managing
//! authentication credentials including cookies, auth codes, and tokens.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Default file paths for storing credentials
pub const COOKIES_FILE: &str = "cookies.json";
pub const CREDENTIALS_FILE: &str = "credentials.json";
pub const TOKENS_FILE: &str = "tokens.json";

/// Represents the complete set of captured credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// Captured cookies
    pub cookies: serde_json::Value,
    /// OAuth authorization code
    pub auth_code: Option<String>,
    /// PKCE code challenge
    pub code_challenge: Option<String>,
    /// OAuth token
    pub oauth_token: Option<String>,
    /// Full token response from Streamlabs
    pub tokens: Option<serde_json::Value>,
    /// Timestamp when credentials were saved
    pub saved_at: String,
}

impl Credentials {
    /// Creates a new empty Credentials instance
    pub fn new() -> Self {
        Self {
            cookies: serde_json::json!({}),
            auth_code: None,
            code_challenge: None,
            oauth_token: None,
            tokens: None,
            saved_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Checks if the credentials contain any data
    pub fn has_data(&self) -> bool {
        self.cookies.as_object().map(|o| !o.is_empty()).unwrap_or(false)
            || self.auth_code.is_some()
            || self.oauth_token.is_some()
    }

    /// Checks if the credentials contain TikTok cookies
    pub fn has_tiktok_cookies(&self) -> bool {
        self.cookies
            .get("data")
            .and_then(|d| d.get("cookies"))
            .map(|c| c.is_object())
            .unwrap_or(false)
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for credential operations
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("No credentials available")]
    NoCredentials,
    #[error("Invalid credential format")]
    InvalidFormat,
}

/// Result type for credential operations
pub type Result<T> = std::result::Result<T, CredentialError>;

/// Saves cookies to a JSON file
///
/// # Arguments
/// * `cookies` - The cookies data to save
/// * `path` - Optional file path (defaults to COOKIES_FILE)
///
/// # Returns
/// `Ok(())` if successful, `Err(CredentialError)` otherwise
pub fn save_cookies(cookies: &serde_json::Value, path: Option<&Path>) -> Result<()> {
    let path = path.unwrap_or_else(|| Path::new(COOKIES_FILE));
    let json = serde_json::to_string_pretty(cookies)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads cookies from a JSON file
///
/// # Arguments
/// * `path` - Optional file path (defaults to COOKIES_FILE)
///
/// # Returns
/// `Ok(cookies)` if successful, `Err(CredentialError)` otherwise
pub fn load_cookies(path: Option<&Path>) -> Result<serde_json::Value> {
    let path = path.unwrap_or_else(|| Path::new(COOKIES_FILE));
    let content = fs::read_to_string(path)?;
    let cookies: serde_json::Value = serde_json::from_str(&content)?;
    Ok(cookies)
}

/// Saves complete credentials to a JSON file
///
/// # Arguments
/// * `credentials` - The credentials to save
/// * `path` - Optional file path (defaults to CREDENTIALS_FILE)
///
/// # Returns
/// `Ok(())` if successful, `Err(CredentialError)` otherwise
pub fn save_credentials(credentials: &Credentials, path: Option<&Path>) -> Result<()> {
    let path = path.unwrap_or_else(|| Path::new(CREDENTIALS_FILE));
    let json = serde_json::to_string_pretty(credentials)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads credentials from a JSON file
///
/// # Arguments
/// * `path` - Optional file path (defaults to CREDENTIALS_FILE)
///
/// # Returns
/// `Ok(credentials)` if successful, `Err(CredentialError)` otherwise
pub fn load_credentials(path: Option<&Path>) -> Result<Credentials> {
    let path = path.unwrap_or_else(|| Path::new(CREDENTIALS_FILE));
    let content = fs::read_to_string(path)?;
    let credentials: Credentials = serde_json::from_str(&content)?;
    Ok(credentials)
}

/// Saves tokens to a JSON file
///
/// # Arguments
/// * `tokens` - The token data to save
/// * `path` - Optional file path (defaults to TOKENS_FILE)
///
/// # Returns
/// `Ok(())` if successful, `Err(CredentialError)` otherwise
pub fn save_tokens(tokens: &serde_json::Value, path: Option<&Path>) -> Result<()> {
    let path = path.unwrap_or_else(|| Path::new(TOKENS_FILE));
    let json = serde_json::to_string_pretty(tokens)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads tokens from a JSON file
///
/// # Arguments
/// * `path` - Optional file path (defaults to TOKENS_FILE)
///
/// # Returns
/// `Ok(tokens)` if successful, `Err(CredentialError)` otherwise
pub fn load_tokens(path: Option<&Path>) -> Result<serde_json::Value> {
    let path = path.unwrap_or_else(|| Path::new(TOKENS_FILE));
    let content = fs::read_to_string(path)?;
    let tokens: serde_json::Value = serde_json::from_str(&content)?;
    Ok(tokens)
}

/// Checks if cookies file exists and contains valid data
///
/// # Arguments
/// * `path` - Optional file path (defaults to COOKIES_FILE)
///
/// # Returns
/// `true` if the file exists and contains valid cookies, `false` otherwise
pub fn has_cookies_file(path: Option<&Path>) -> bool {
    let path = path.unwrap_or_else(|| Path::new(COOKIES_FILE));
    if !path.exists() {
        return false;
    }
    
    match load_cookies(Some(path)) {
        Ok(cookies) => cookies
            .get("data")
            .and_then(|d| d.get("cookies"))
            .map(|c| c.is_object())
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// Checks if tokens file exists and contains an OAuth token
///
/// # Arguments
/// * `path` - Optional file path (defaults to TOKENS_FILE)
///
/// # Returns
/// `true` if the file exists and contains a valid token, `false` otherwise
pub fn has_tokens_file(path: Option<&Path>) -> bool {
    let path = path.unwrap_or_else(|| Path::new(TOKENS_FILE));
    if !path.exists() {
        return false;
    }
    
    match load_tokens(Some(path)) {
        Ok(tokens) => tokens
            .get("oauth_token")
            .or(tokens.get("access_token"))
            .is_some(),
        Err(_) => false,
    }
}

/// Extracts the OAuth token from a token response
///
/// # Arguments
/// * `tokens` - The token JSON value
///
/// # Returns
/// `Some(token)` if found, `None` otherwise
pub fn extract_oauth_token(tokens: &serde_json::Value) -> Option<String> {
    tokens
        .get("oauth_token")
        .or(tokens.get("access_token"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// A thread-safe credential manager for runtime credential storage
pub struct CredentialManager {
    cookies: Arc<Mutex<Option<serde_json::Value>>>,
    auth_code: Arc<Mutex<Option<String>>>,
    code_challenge: Arc<Mutex<Option<String>>>,
    oauth_token: Arc<Mutex<Option<String>>>,
}

impl CredentialManager {
    /// Creates a new CredentialManager
    pub fn new() -> Self {
        Self {
            cookies: Arc::new(Mutex::new(None)),
            auth_code: Arc::new(Mutex::new(None)),
            code_challenge: Arc::new(Mutex::new(None)),
            oauth_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the captured cookies
    pub fn set_cookies(&self, cookies: serde_json::Value) {
        *self.cookies.lock() = Some(cookies);
    }

    /// Gets the captured cookies
    pub fn get_cookies(&self) -> Option<serde_json::Value> {
        self.cookies.lock().clone()
    }

    /// Sets the authorization code
    pub fn set_auth_code(&self, code: String) {
        *self.auth_code.lock() = Some(code);
    }

    /// Gets the authorization code
    pub fn get_auth_code(&self) -> Option<String> {
        self.auth_code.lock().clone()
    }

    /// Sets the code challenge
    pub fn set_code_challenge(&self, challenge: String) {
        *self.code_challenge.lock() = Some(challenge);
    }

    /// Gets the code challenge
    pub fn get_code_challenge(&self) -> Option<String> {
        self.code_challenge.lock().clone()
    }

    /// Sets the OAuth token
    pub fn set_oauth_token(&self, token: String) {
        *self.oauth_token.lock() = Some(token);
    }

    /// Gets the OAuth token
    pub fn get_oauth_token(&self) -> Option<String> {
        self.oauth_token.lock().clone()
    }

    /// Builds a Credentials struct from the current state
    pub fn build_credentials(&self) -> Credentials {
        Credentials {
            cookies: self.get_cookies().unwrap_or_else(|| serde_json::json!({})),
            auth_code: self.get_auth_code(),
            code_challenge: self.get_code_challenge(),
            oauth_token: self.get_oauth_token(),
            tokens: None,
            saved_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Saves all credentials to files
    pub fn save_all(&self) -> Result<()> {
        if let Some(cookies) = self.get_cookies() {
            save_cookies(&cookies, None)?;
        }

        let credentials = self.build_credentials();
        save_credentials(&credentials, None)?;

        if let Some(token) = self.get_oauth_token() {
            let tokens = serde_json::json!({ "oauth_token": token });
            save_tokens(&tokens, None)?;
        }

        Ok(())
    }

    /// Clears all stored credentials
    pub fn clear(&self) {
        *self.cookies.lock() = None;
        *self.auth_code.lock() = None;
        *self.code_challenge.lock() = None;
        *self.oauth_token.lock() = None;
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_new() {
        let creds = Credentials::new();
        assert!(!creds.has_data());
        assert!(!creds.has_tiktok_cookies());
    }

    #[test]
    fn test_credentials_has_data() {
        let mut creds = Credentials::new();
        assert!(!creds.has_data());
        
        creds.auth_code = Some("test_code".to_string());
        assert!(creds.has_data());
    }

    #[test]
    fn test_extract_oauth_token() {
        let tokens = serde_json::json!({ "oauth_token": "test_token" });
        assert_eq!(extract_oauth_token(&tokens), Some("test_token".to_string()));
        
        let tokens2 = serde_json::json!({ "access_token": "test_token2" });
        assert_eq!(extract_oauth_token(&tokens2), Some("test_token2".to_string()));
        
        let tokens3 = serde_json::json!({});
        assert_eq!(extract_oauth_token(&tokens3), None);
    }

    #[test]
    fn test_credential_manager() {
        let manager = CredentialManager::new();
        
        assert!(manager.get_cookies().is_none());
        assert!(manager.get_auth_code().is_none());
        
        manager.set_cookies(serde_json::json!({ "test": "value" }));
        assert!(manager.get_cookies().is_some());
        
        manager.set_auth_code("test_code".to_string());
        assert_eq!(manager.get_auth_code(), Some("test_code".to_string()));
        
        let creds = manager.build_credentials();
        assert!(creds.has_data());
    }
}
