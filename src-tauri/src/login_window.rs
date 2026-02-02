//! Login window management module for TikTok authentication
//!
//! This module handles the creation and management of the TikTok login window,
//! including event listeners for credential capture and URL navigation.

use crate::cookie_interceptor::get_cookie_interceptor_script;
use crate::pkce::{generate_code_challenge, generate_code_verifier};
use crate::url_utils::{classify_url, UrlType};
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::{Emitter, Listener, Manager, WebviewWindowBuilder, WebviewUrl};

/// Configuration for the login window
pub struct LoginWindowConfig {
    /// The window label
    pub label: String,
    /// The window title
    pub title: String,
    /// The initial URL to load
    pub url: String,
    /// Window width
    pub width: f64,
    /// Window height
    pub height: f64,
    /// Whether the window is resizable
    pub resizable: bool,
    /// Whether the window is always on top
    pub always_on_top: bool,
}

impl Default for LoginWindowConfig {
    fn default() -> Self {
        Self {
            label: "tiktok-login".to_string(),
            title: "TikTok Login".to_string(),
            url: "https://www.tiktok.com/login".to_string(),
            width: 450.0,
            height: 700.0,
            resizable: false,
            always_on_top: true,
        }
    }
}

/// State for tracking login window
pub struct LoginState {
    pub login_window_label: Mutex<String>,
    pub captured_cookies: Arc<Mutex<Option<serde_json::Value>>>,
    pub captured_local_storage: Arc<Mutex<Option<serde_json::Value>>>,
    pub captured_session_storage: Arc<Mutex<Option<serde_json::Value>>>,
    pub login_complete: Arc<AtomicBool>,
    pub auth_code: Arc<Mutex<Option<String>>>,
    pub code_verifier: Arc<Mutex<Option<String>>>,
    pub code_challenge: Arc<Mutex<Option<String>>>,
    pub streamlabs_token: Arc<Mutex<Option<String>>>,
    pub tiktok_session_data: Arc<Mutex<Option<serde_json::Value>>>,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            login_window_label: Mutex::new(String::new()),
            captured_cookies: Arc::new(Mutex::new(None)),
            captured_local_storage: Arc::new(Mutex::new(None)),
            captured_session_storage: Arc::new(Mutex::new(None)),
            login_complete: Arc::new(AtomicBool::new(false)),
            auth_code: Arc::new(Mutex::new(None)),
            code_verifier: Arc::new(Mutex::new(None)),
            code_challenge: Arc::new(Mutex::new(None)),
            streamlabs_token: Arc::new(Mutex::new(None)),
            tiktok_session_data: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for LoginState {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for login window operations
#[derive(Debug, thiserror::Error)]
pub enum LoginWindowError {
    #[error("Failed to create window: {0}")]
    WindowCreationFailed(String),
    #[error("Window not found: {0}")]
    WindowNotFound(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for login window operations
pub type Result<T> = std::result::Result<T, LoginWindowError>;

/// Opens the TikTok login window
///
/// # Arguments
/// * `app` - The Tauri app handle
/// * `login_state` - The login state
///
/// # Returns
/// A JSON response indicating success or failure
pub async fn open_tiktok_login_window(
    app: tauri::AppHandle,
    login_state: &LoginState,
) -> Result<serde_json::Value> {
    println!("[TikTok Login] Opening login window...");

    let config = LoginWindowConfig::default();

    // Try to get existing window first
    if let Some(window) = app.get_webview_window(&config.label) {
        let _ = window.set_focus();
        println!("[TikTok Login] Login window already exists, focusing...");
        return Ok(json!({
            "success": true,
            "message": "Login window already open"
        }));
    }

    // Create login window
    let login_window = create_login_window(&app, &config)?;

    println!("[TikTok Login] Login window created successfully");

    // Store window label
    {
        let mut label_guard = login_state.login_window_label.lock();
        *label_guard = config.label.clone();
    }

    // Generate code verifier and challenge
    let code_verifier = generate_code_verifier();
    let code_challenge_str = generate_code_challenge(&code_verifier);

    // Store code verifier and challenge for later token exchange
    {
        let mut verifier_guard = login_state.code_verifier.lock();
        *verifier_guard = Some(code_verifier.clone());
    }
    {
        let mut challenge_guard = login_state.code_challenge.lock();
        *challenge_guard = Some(code_challenge_str.clone());
    }

    // Setup event listeners
    setup_credentials_listener(&login_window, login_state);
    setup_cookies_listener(&login_window, login_state);
    setup_navigation_listener(&login_window, login_state, &code_challenge_str);

    // Inject cookie interceptor and start URL polling
    setup_cookie_interceptor(&login_window, login_state, &code_challenge_str).await;

    Ok(json!({
        "success": true,
        "message": "Login window opened",
        "window_label": config.label
    }))
}

/// Creates a new login window with the given configuration
fn create_login_window(
    app: &tauri::AppHandle,
    config: &LoginWindowConfig,
) -> Result<tauri::WebviewWindow> {
    let url = config
        .url
        .parse()
        .map_err(|e: url::ParseError| LoginWindowError::InvalidUrl(e.to_string()))?;

    let window = WebviewWindowBuilder::new(app, &config.label, WebviewUrl::External(url))
        .title(&config.title)
        .inner_size(config.width, config.height)
        .min_inner_size(config.width, config.height)
        .resizable(config.resizable)
        .center()
        .always_on_top(config.always_on_top)
        .build()
        .map_err(|e| LoginWindowError::WindowCreationFailed(e.to_string()))?;

    Ok(window)
}

/// Sets up the credentials capture listener
fn setup_credentials_listener(window: &tauri::WebviewWindow, login_state: &LoginState) {
    let captured_cookies = login_state.captured_cookies.clone();
    let captured_local_storage = login_state.captured_local_storage.clone();
    let captured_session_storage = login_state.captured_session_storage.clone();
    let tiktok_session_data = login_state.tiktok_session_data.clone();
    let login_complete = login_state.login_complete.clone();

    let _listener_id = window.listen("credentials-captured", move |event: tauri::Event| {
        println!("[TikTok Login] Credentials captured event received");
        login_complete.store(true, std::sync::atomic::Ordering::SeqCst);

        let payload = event.payload();
        println!("[TikTok Login] Raw payload: {}", payload);

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
            // Store cookies
            let mut cookies_guard = captured_cookies.lock();
            *cookies_guard = Some(data.clone());

            // Store localStorage
            let mut ls_guard = captured_local_storage.lock();
            *ls_guard = data.get("localStorage").cloned();

            // Store sessionStorage
            let mut ss_guard = captured_session_storage.lock();
            *ss_guard = data.get("sessionStorage").cloned();

            // Store complete session data
            let mut session_guard = tiktok_session_data.lock();
            *session_guard = Some(data.clone());

            println!("[TikTok Login] All credentials parsed and stored");
            println!(
                "[TikTok Login] Cookies: {}",
                data.get("cookies")
                    .map(|c| c.as_object().map(|o| o.len()).unwrap_or(0))
                    .unwrap_or(0)
            );
            println!(
                "[TikTok Login] localStorage: {}",
                data.get("localStorage")
                    .map(|l| l.as_object().map(|o| o.len()).unwrap_or(0))
                    .unwrap_or(0)
            );
            println!(
                "[TikTok Login] sessionStorage: {}",
                data.get("sessionStorage")
                    .map(|s| s.as_object().map(|o| o.len()).unwrap_or(0))
                    .unwrap_or(0)
            );
        }
    });
}

/// Sets up the cookies listener for cached login detection
fn setup_cookies_listener(window: &tauri::WebviewWindow, login_state: &LoginState) {
    let captured_cookies = login_state.captured_cookies.clone();

    let _listener_id = window.listen("tiktok-cookies", move |event: tauri::Event| {
        println!("[TikTok Login] TikTok cookies message received");

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let cookies = if let Some(cookies_obj) = data.get("cookies") {
                cookies_obj.clone()
            } else if let Some(cookies_obj) = data.as_object() {
                json!(cookies_obj)
            } else {
                json!({})
            };

            if cookies.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                let creds_json = json!({
                    "success": true,
                    "data": {
                        "cookies": cookies
                    }
                });

                let mut cookies_guard = captured_cookies.lock();
                *cookies_guard = Some(creds_json.clone());
                println!("[TikTok Login] Cached TikTok cookies captured and stored");

                // Save cookies to file
                let cookies_str = serde_json::to_string_pretty(&creds_json).unwrap();
                if let Err(e) = fs::write("cookies.json", &cookies_str) {
                    println!("[TikTok Login] Failed to save cached cookies: {}", e);
                } else {
                    println!("[TikTok Login] Cached cookies saved to cookies.json");
                }
            }
        }
    });
}

/// Sets up the navigation listener for URL change detection
fn setup_navigation_listener(window: &tauri::WebviewWindow, login_state: &LoginState, code_challenge: &str) {
    let window_clone = window.clone();
    let auth_code = login_state.auth_code.clone();
    let captured_cookies = login_state.captured_cookies.clone();
    let streamlabs_token = login_state.streamlabs_token.clone();
    let code_challenge = code_challenge.to_string();

    // Clone variables for use in second listener
    let auth_code_for_url = auth_code.clone();
    let captured_cookies_for_url = captured_cookies.clone();
    let streamlabs_token_for_url = streamlabs_token.clone();
    let _code_challenge_for_url = code_challenge.clone();

    // Listen for navigation events
    let _listener_id = window.listen("navigation", move |event: tauri::Event| {
        let url = event.payload();
        println!("[TikTok Login] Navigation detected: {}", url);

        match classify_url(url) {
            UrlType::StreamlabsAuthWithCode(code) => {
                println!("[TikTok Login] Streamlabs redirect detected!");
                let mut auth_code_guard = auth_code.lock();
                *auth_code_guard = Some(code.clone());
                let _ = window_clone.emit("auth-code-received", code);

                // Trigger token exchange after receiving auth code
                let window_for_token = window_clone.clone();
                let streamlabs_token_clone = streamlabs_token.clone();
                tokio::spawn(async move {
                    println!("[Streamlabs Token] Auth code received, initiating token exchange...");
                    
                    // Wait a moment for the redirect to complete
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    
                    // Check if we already have a token
                    {
                        let token_guard = streamlabs_token_clone.lock();
                        if token_guard.is_some() {
                            println!("[Streamlabs Token] Token already exists, skipping exchange");
                            return;
                        }
                    }
                    
                    // Emit event to trigger token exchange from the main thread
                    let _ = window_for_token.emit("exchange-streamlabs-token", json!({}));
                });
            }
            UrlType::StreamlabsAuth => {
                println!("[TikTok Login] Detected Streamlabs auth page - waiting for redirect...");
            }
            UrlType::TikTokLoggedIn => {
                println!("[TikTok Login] Detected TikTok main page - user already logged in, will redirect to Streamlabs...");
                // Do NOT close window - will be redirected to Streamlabs by Tauri
            }
            _ => {}
        }
    });

    // Listen for current-url events from URL polling
    let window_for_url = window.clone();
    let _url_listener_id = window.listen("current-url", move |event: tauri::Event| {
        let url = event.payload();
        println!("[TikTok Login] Current URL from polling: {}", url);

        match classify_url(url) {
            UrlType::StreamlabsAuthWithCode(code) => {
                println!("[TikTok Login] Streamlabs redirect with code detected (from polling)!");
                let mut auth_code_guard = auth_code_for_url.lock();
                *auth_code_guard = Some(code.clone());
                let _ = window_for_url.emit("auth-code-received", code);

                // Trigger token exchange after receiving auth code
                let window_for_token = window_for_url.clone();
                let streamlabs_token_clone = streamlabs_token_for_url.clone();
                tokio::spawn(async move {
                    println!("[Streamlabs Token] Auth code received (from polling), initiating token exchange...");
                    
                    // Wait a moment for the redirect to complete
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    
                    // Check if we already have a token
                    {
                        let token_guard = streamlabs_token_clone.lock();
                        if token_guard.is_some() {
                            println!("[Streamlabs Token] Token already exists, skipping exchange");
                            return;
                        }
                    }
                    
                    // Emit event to trigger token exchange from the main thread
                    let _ = window_for_token.emit("exchange-streamlabs-token", json!({}));
                });
            }
            UrlType::TikTokLoggedIn => {
                println!("[TikTok Login] Detected TikTok login (from polling) - will redirect to Streamlabs...");
                // Do NOT close window - will be redirected to Streamlabs by Tauri
            }
            _ => {}
        }
    });
}

/// Sets up the cookie interceptor and URL polling
async fn setup_cookie_interceptor(window: &tauri::WebviewWindow, _login_state: &LoginState, _code_challenge: &str) {
    let window_clone = window.clone();

    // Setup listener for script injection - this actually executes the JavaScript
    let window_for_script = window.clone();
    let _script_listener = window.listen("inject-script", move |event: tauri::Event| {
        let script = event.payload();
        println!("[Cookie Interceptor] Script injection requested for login window");
        
        // Execute the script in the webview
        if let Err(e) = window_for_script.eval(script) {
            println!("[Cookie Interceptor] Failed to inject script: {}", e);
        } else {
            println!("[Cookie Interceptor] Script injected successfully");
        }
    });

    // Setup listener for token exchange event
    let window_for_token = window.clone();
    let _token_listener = window.listen("exchange-streamlabs-token", move |_event: tauri::Event| {
        println!("[Streamlabs Token] Token exchange event received");
        let window_token = window_for_token.clone();
        
        // Spawn async task to perform token exchange
        tokio::spawn(async move {
            println!("[Streamlabs Token] Token exchange task started");
            
            // Emit event to notify frontend that token exchange is in progress
            let _ = window_token.emit("token-exchange-started", json!({}));
        });
    });

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let script = get_cookie_interceptor_script();
        let _ = window_clone.emit("inject-script", script);
        println!("[TikTok Login] Cookie interceptor injected");

        // Start URL polling as fallback (every 500ms)
        let window_clone = window_clone.clone();
        let _last_url = String::new();
        let mut attempts = 0;
        let max_attempts = 240; // 2 minutes

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                attempts += 1;

                if attempts >= max_attempts {
                    println!("[TikTok Login] URL polling timeout");
                    break;
                }

                // Inject script to get current URL and emit it as an event
                let get_url_script = r#"
                    (function() {
                        window.__tauri__.emit('current-url', window.location.href);
                    })();
                "#;
                let _ = window_clone.emit("inject-script", get_url_script);

                // Wait a bit for the URL to be emitted
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Note: The actual URL will be received via 'current-url' event
                // which is handled by navigation listener
                // This polling is just a fallback to ensure we don't miss URL changes
                continue;
            }
        });
    });
}

/// Closes the login window
///
/// # Arguments
/// * `app` - The Tauri app handle
/// * `login_state` - The login state
///
/// # Returns
/// A JSON response indicating success or failure
pub fn close_login_window(
    app: tauri::AppHandle,
    login_state: &LoginState,
) -> Result<serde_json::Value> {
    let label_guard = login_state.login_window_label.lock();
    let label = label_guard.clone();

    if !label.is_empty() {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
            println!("[TikTok Login] Closed login window: {}", label);

            let mut label_mut = login_state.login_window_label.lock();
            *label_mut = String::new();

            return Ok(json!({
                "success": true,
                "message": format!("Login window {} closed", label)
            }));
        }
    }

    Ok(json!({
        "success": false,
        "message": "No login window was open"
    }))
}

/// Injects the cookie interceptor script into the login window
///
/// # Arguments
/// * `app` - The Tauri app handle
/// * `login_state` - The login state
///
/// # Returns
/// A JSON response indicating success or failure
pub fn inject_interceptor_to_login_window(
    app: tauri::AppHandle,
    login_state: &LoginState,
) -> Result<serde_json::Value> {
    let label_guard = login_state.login_window_label.lock();
    let label = label_guard.clone();

    if label.is_empty() {
        return Err(LoginWindowError::OperationFailed("No login window open".to_string()));
    }

    if let Some(window) = app.get_webview_window(&label) {
        let script = get_cookie_interceptor_script();
        let result = window.emit("inject-script", script);
        match result {
            Ok(_) => {
                println!("[Cookie Interceptor] Script injection requested for login window");
                Ok(json!({
                    "success": true,
                    "message": "Interceptor injection requested"
                }))
            }
            Err(e) => {
                println!("[Cookie Interceptor] Failed to emit: {}", e);
                Err(LoginWindowError::OperationFailed(format!("Failed to inject script: {}", e)))
            }
        }
    } else {
        Err(LoginWindowError::WindowNotFound(label))
    }
}

/// Injects the cookie interceptor script into a given window
///
/// # Arguments
/// * `window` - The window to inject into
///
/// # Returns
/// A JSON response indicating success or failure
pub fn inject_cookie_interceptor(window: tauri::WebviewWindow) -> Result<serde_json::Value> {
    let script = get_cookie_interceptor_script();
    let result = window.emit("inject-script", script);

    match result {
        Ok(_) => {
            println!("[Cookie Interceptor] Script injection requested");
            Ok(json!({
                "success": true,
                "message": "Cookie interceptor injection requested"
            }))
        }
        Err(e) => {
            println!("[Cookie Interceptor] Failed to emit: {}", e);
            Err(LoginWindowError::OperationFailed(format!("Failed to inject script: {}", e)))
        }
    }
}

/// Gets the captured credentials from the login state
///
/// # Arguments
/// * `login_state` - The login state
///
/// # Returns
/// The captured credentials or an error message
pub fn get_captured_credentials(login_state: &LoginState) -> serde_json::Value {
    let cookies = login_state.captured_cookies.lock();
    if let Some(creds) = cookies.as_ref() {
        creds.clone()
    } else {
        json!({
            "success": false,
            "message": "No credentials captured yet"
        })
    }
}

/// Exchanges the code verifier for a Streamlabs OAuth token
///
/// This function implements the PKCE token exchange with the Streamlabs API.
/// It sends the code_verifier to the Streamlabs auth endpoint and receives
/// an OAuth token in response.
///
/// # Arguments
/// * `login_state` - The login state containing the code_verifier
///
/// # Returns
/// A JSON response containing the OAuth token or an error message
///
/// # Example
/// ```no_run
/// use login_window::exchange_streamlabs_token;
///
/// let result = exchange_streamlabs_token(&login_state).await;
/// ```
pub async fn exchange_streamlabs_token(login_state: &LoginState) -> Result<serde_json::Value> {
    println!("[Streamlabs Token] Starting token exchange...");

    // Get the code_verifier from login state
    let code_verifier = {
        let verifier_guard = login_state.code_verifier.lock();
        match verifier_guard.as_ref() {
            Some(v) => v.clone(),
            None => {
                return Err(LoginWindowError::TokenExchangeFailed(
                    "Code verifier not found in login state".to_string()
                ));
            }
        }
    };

    println!("[Streamlabs Token] Code verifier found, making API request...");

    // Streamlabs API endpoint for token exchange
    const STREAMLABS_API_URL: &str = "https://streamlabs.com/api/v5/slobs/auth/data";

    // Build the request with code_verifier
    let client = reqwest::Client::new();
    let response = client
        .get(STREAMLABS_API_URL)
        .query(&[("code_verifier", &code_verifier)])
        .send()
        .await
        .map_err(|e| LoginWindowError::TokenExchangeFailed(format!("Request failed: {}", e)))?;

    println!("[Streamlabs Token] API response status: {}", response.status());

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(LoginWindowError::TokenExchangeFailed(
            format!("API returned status {}: {}", status, error_text)
        ));
    }

    // Parse the response
    let response_text = response
        .text()
        .await
        .map_err(|e| LoginWindowError::InvalidResponse(format!("Failed to read response: {}", e)))?;

    println!("[Streamlabs Token] Response received: {}", response_text);

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| LoginWindowError::InvalidResponse(format!("Failed to parse JSON: {}", e)))?;

    // Check if the response indicates success
    if response_json.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
        // Extract the OAuth token from the response
        if let Some(data) = response_json.get("data") {
            if let Some(oauth_token) = data.get("oauth_token").and_then(|t| t.as_str()) {
                println!("[Streamlabs Token] Successfully obtained OAuth token: {}...", &oauth_token[..8]);

                // Store the token in login state
                {
                    let mut token_guard = login_state.streamlabs_token.lock();
                    *token_guard = Some(oauth_token.to_string());
                }

                return Ok(json!({
                    "success": true,
                    "message": "Token obtained successfully",
                    "token": oauth_token
                }));
            }
        }
    }

    // If we get here, the token was not found in the expected location
    Err(LoginWindowError::TokenExchangeFailed(
        format!("Token not found in response: {}", response_text)
    ))
}

/// Gets the Streamlabs OAuth token from the login state
///
/// # Arguments
/// * `login_state` - The login state
///
/// # Returns
/// The OAuth token if available, None otherwise
pub fn get_streamlabs_token(login_state: &LoginState) -> Option<String> {
    let token_guard = login_state.streamlabs_token.lock();
    token_guard.as_ref().cloned()
}

/// Performs the Streamlabs token exchange and returns the result
///
/// This is a wrapper function that can be called from Tauri commands
/// to initiate the token exchange process.
///
/// # Arguments
/// * `login_state` - The login state containing the code_verifier
///
/// # Returns
/// A JSON response containing the OAuth token or an error message
pub async fn perform_streamlabs_token_exchange(login_state: &LoginState) -> serde_json::Value {
    match exchange_streamlabs_token(login_state).await {
        Ok(result) => result,
        Err(e) => {
            println!("[Streamlabs Token] Token exchange failed: {}", e);
            json!({
                "success": false,
                "message": format!("Token exchange failed: {}", e)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_window_config_default() {
        let config = LoginWindowConfig::default();
        assert_eq!(config.label, "tiktok-login");
        assert_eq!(config.title, "TikTok Login");
        assert_eq!(config.width, 450.0);
        assert_eq!(config.height, 700.0);
    }

    #[test]
    fn test_login_state_new() {
        let state = LoginState::new();
        assert!(!state.login_complete.load(std::sync::atomic::Ordering::SeqCst));
    }
}