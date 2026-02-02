//! URL utilities for parsing and extracting authentication codes
//!
//! This module provides helper functions for working with URLs in the
//! OAuth 2.0 flow, particularly for extracting authorization codes from
//! redirect URLs.

/// Represents the type of URL detected during navigation
#[derive(Debug, Clone, PartialEq)]
pub enum UrlType {
    /// TikTok login page
    TikTokLogin,
    /// TikTok main page (logged in)
    TikTokLoggedIn,
    /// Streamlabs login page
    #[allow(dead_code)]
    StreamlabsLogin,
    /// Streamlabs auth page with code
    StreamlabsAuthWithCode(String),
    /// Streamlabs auth page without code
    StreamlabsAuth,
    /// Unknown URL type
    Unknown,
}

/// Extracts the authorization code from a URL.
///
/// This function looks for the `code` parameter in the URL query string
/// and returns its value if found.
///
/// # Arguments
/// * `url` - The URL string to parse
///
/// # Returns
/// `Some(code)` if the code parameter is found, `None` otherwise.
///
/// # Example
/// ```no_run
/// use url_utils::extract_auth_code;
///
/// let url = "https://streamlabs.com/m/login?code=abc123&state=xyz";
/// assert_eq!(extract_auth_code(url), Some("abc123".to_string()));
/// ```
pub fn extract_auth_code(url: &str) -> Option<String> {
    if let Some(code_start) = url.find("code=") {
        let code_section = &url[code_start + 5..];
        let code_end = code_section.find('&').unwrap_or(code_section.len());
        Some(code_section[..code_end].to_string())
    } else {
        None
    }
}

/// Determines the type of URL based on its content.
///
/// This function analyzes the URL string and categorizes it into
/// different types useful for the authentication flow.
///
/// # Arguments
/// * `url` - The URL string to analyze
///
/// # Returns
/// A [`UrlType`] enum variant representing the URL type.
///
/// # Example
/// ```no_run
/// use url_utils::{UrlType, classify_url};
///
/// let url = "https://streamlabs.com/m/login?code=abc123";
/// assert!(matches!(classify_url(url), UrlType::StreamlabsAuthWithCode(_)));
/// ```
pub fn classify_url(url: &str) -> UrlType {
    // Check for Streamlabs redirect with code
    if url.contains("streamlabs.com") && url.contains("code=") {
        if let Some(code) = extract_auth_code(url) {
            return UrlType::StreamlabsAuthWithCode(code);
        }
    }
    
    // Check for Streamlabs login page
    if url.contains("streamlabs.com/m/login") || url.contains("streamlabs.com/tiktok/auth") {
        return UrlType::StreamlabsAuth;
    }
    
    // Check for TikTok login page
    if url.contains("tiktok.com/login") {
        return UrlType::TikTokLogin;
    }
    
    // Check for TikTok logged in state
    let is_tiktok_logged_in = (url.contains("tiktok.com") && 
                               !url.contains("/login") && 
                               !url.contains("webcast") &&
                               !url.contains("streamlabs")) ||
                              url.contains("tiktok.com/foryou") ||
                              url.contains("tiktok.com/discover") ||
                              (url.contains("tiktok.com/") && !url.contains("/login"));
    
    if is_tiktok_logged_in {
        return UrlType::TikTokLoggedIn;
    }
    
    UrlType::Unknown
}

/// Checks if a URL is a TikTok domain.
///
/// # Arguments
/// * `url` - The URL string to check
///
/// # Returns
/// `true` if the URL contains tiktok.com, `false` otherwise.
#[allow(dead_code)]
pub fn is_tiktok_url(url: &str) -> bool {
    url.contains("tiktok.com")
}

/// Checks if a URL is a Streamlabs domain.
///
/// # Arguments
/// * `url` - The URL string to check
///
/// # Returns
/// `true` if the URL contains streamlabs.com, `false` otherwise.
#[allow(dead_code)]
pub fn is_streamlabs_url(url: &str) -> bool {
    url.contains("streamlabs.com")
}

/// Checks if a URL indicates a logged-in state on TikTok.
///
/// This function checks various indicators that suggest the user
/// is logged into TikTok.
///
/// # Arguments
/// * `url` - The URL string to check
///
/// # Returns
/// `true` if the URL suggests a logged-in state, `false` otherwise.
#[allow(dead_code)]
pub fn is_tiktok_logged_in(url: &str) -> bool {
    matches!(classify_url(url), UrlType::TikTokLoggedIn)
}

/// Checks if a URL contains an authorization code.
///
/// # Arguments
/// * `url` - The URL string to check
///
/// # Returns
/// `true` if the URL contains a code parameter, `false` otherwise.
#[allow(dead_code)]
pub fn has_auth_code(url: &str) -> bool {
    url.contains("code=")
}

/// Builds a Streamlabs authorization URL with the given code challenge.
///
/// # Arguments
/// * `code_challenge` - The PKCE code challenge
///
/// # Returns
/// A formatted Streamlabs authorization URL.
#[allow(dead_code)]
pub fn build_streamlabs_auth_url(code_challenge: &str) -> String {
    format!(
        "https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge={}",
        code_challenge
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_auth_code() {
        let url = "https://streamlabs.com/m/login?code=abc123&state=xyz";
        assert_eq!(extract_auth_code(url), Some("abc123".to_string()));
        
        let url2 = "https://streamlabs.com/m/login?code=xyz789";
        assert_eq!(extract_auth_code(url2), Some("xyz789".to_string()));
        
        let url3 = "https://example.com?state=xyz";
        assert_eq!(extract_auth_code(url3), None);
    }

    #[test]
    fn test_classify_url() {
        assert!(matches!(
            classify_url("https://streamlabs.com/m/login?code=abc123"),
            UrlType::StreamlabsAuthWithCode(_)
        ));
        
        assert_eq!(
            classify_url("https://streamlabs.com/m/login"),
            UrlType::StreamlabsAuth
        );
        
        assert_eq!(
            classify_url("https://www.tiktok.com/login"),
            UrlType::TikTokLogin
        );
        
        assert_eq!(
            classify_url("https://www.tiktok.com/foryou"),
            UrlType::TikTokLoggedIn
        );
        
        assert_eq!(
            classify_url("https://example.com"),
            UrlType::Unknown
        );
    }

    #[test]
    fn test_is_tiktok_url() {
        assert!(is_tiktok_url("https://www.tiktok.com/login"));
        assert!(is_tiktok_url("https://tiktok.com/foryou"));
        assert!(!is_tiktok_url("https://streamlabs.com"));
    }

    #[test]
    fn test_is_streamlabs_url() {
        assert!(is_streamlabs_url("https://streamlabs.com/m/login"));
        assert!(is_streamlabs_url("https://www.streamlabs.com"));
        assert!(!is_streamlabs_url("https://tiktok.com"));
    }

    #[test]
    fn test_is_tiktok_logged_in() {
        assert!(is_tiktok_logged_in("https://www.tiktok.com/foryou"));
        assert!(is_tiktok_logged_in("https://www.tiktok.com/discover"));
        assert!(is_tiktok_logged_in("https://www.tiktok.com/@user"));
        assert!(!is_tiktok_logged_in("https://www.tiktok.com/login"));
    }

    #[test]
    fn test_has_auth_code() {
        assert!(has_auth_code("https://streamlabs.com/m/login?code=abc123"));
        assert!(!has_auth_code("https://streamlabs.com/m/login"));
    }

    #[test]
    fn test_build_streamlabs_auth_url() {
        let url = build_streamlabs_auth_url("test_challenge");
        assert!(url.contains("streamlabs.com/m/login"));
        assert!(url.contains("code_challenge=test_challenge"));
        assert!(url.contains("force_verify=1"));
        assert!(url.contains("tiktok"));
    }
}
