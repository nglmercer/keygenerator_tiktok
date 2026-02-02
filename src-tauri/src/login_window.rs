//! Login window management module for TikTok authentication
//!
//! This module handles the creation and management of the TikTok login window,
//! including event listeners for credential capture and URL navigation.

use crate::cookie_interceptor::{get_cookie_interceptor_script, get_cookie_extraction_script};
use crate::pkce::{generate_code_challenge, generate_code_verifier};
use crate::url_utils::{build_streamlabs_auth_url, classify_url, UrlType};
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::{Emitter, Listener, Manager, WebviewWindowBuilder, WebviewUrl};
use url::Url;

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
    pub code_challenge: Arc<Mutex<Option<String>>>,
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
            code_challenge: Arc::new(Mutex::new(None)),
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

    // Store code challenge for later token exchange
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
    let code_challenge = code_challenge.to_string();

    let _listener_id = window.listen("navigation", move |event: tauri::Event| {
        let url = event.payload();
        println!("[TikTok Login] Navigation detected: {}", url);

        match classify_url(url) {
            UrlType::StreamlabsAuthWithCode(code) => {
                println!("[TikTok Login] Streamlabs redirect detected!");
                let mut auth_code_guard = auth_code.lock();
                *auth_code_guard = Some(code.clone());
                let _ = window_clone.emit("auth-code-received", code);
            }
            UrlType::StreamlabsAuth => {
                println!("[TikTok Login] Detected Streamlabs auth page - waiting for redirect...");
            }
            UrlType::TikTokLoggedIn => {
                println!("[TikTok Login] Detected TikTok main page - user already logged in, redirecting to Streamlabs...");

                // If we already have cookies captured, save them
                let cookies_guard = captured_cookies.lock();
                if let Some(creds) = cookies_guard.as_ref() {
                    if let Err(e) = fs::write("cookies.json", serde_json::to_string_pretty(creds).unwrap()) {
                        println!("[TikTok Login] Failed to save cookies: {}", e);
                    } else {
                        println!("[TikTok Login] Cookies saved to cookies.json");
                    }
                }

                // Navigate to Streamlabs auth URL
                let auth_url = build_streamlabs_auth_url(&code_challenge);
                println!("[TikTok Login] Redirecting to: {}", auth_url);

                // Use emit to send navigation script to the webview
                let navigate_script = format!("window.location.href = '{}';", auth_url);
                let _ = window_clone.emit("inject-script", navigate_script);
            }
            _ => {}
        }
    });
}

/// Sets up the cookie interceptor and URL polling
async fn setup_cookie_interceptor(window: &tauri::WebviewWindow, login_state: &LoginState, code_challenge: &str) {
    let window_clone = window.clone();
    let auth_code = login_state.auth_code.clone();
    let captured_cookies = login_state.captured_cookies.clone();
    let code_challenge = code_challenge.to_string();

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let script = get_cookie_interceptor_script();
        let _ = window_clone.emit("inject-script", script);
        println!("[TikTok Login] Cookie interceptor injected");

        // Start URL polling as fallback (every 500ms)
        let window_clone = window_clone.clone();
        let mut last_url = String::new();
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

                    // Get current URL from the window
                    // Note: We'll use the URL from navigation events instead
                    // since the url() method is not available on WebviewWindow
                    let url_str = last_url.clone();

                if url_str != last_url {
                    println!("[TikTok Login] URL changed to: {}", url_str);
                    last_url = url_str.clone();

                    match classify_url(&url_str) {
                        UrlType::StreamlabsAuthWithCode(code) => {
                            println!("[TikTok Login] Streamlabs redirect with code detected!");

                            // Store auth code in state
                            let mut auth_code_store = auth_code.lock();
                            *auth_code_store = Some(code.clone());

                            // Emit Tauri event for frontend to handle
                            let _ = window_clone.emit("auth-code-received", url_str.as_str());

                            // Also inject script to postMessage to parent
                            let postmessage_script = format!(
                                r#"
                                (function() {{
                                    window.parent.postMessage({{ type: 'auth-code-received', url: '{}' }}, '*');
                                }})();
                            "#,
                                url_str
                            );
                            let _ = window_clone.emit("inject-script", postmessage_script);

                            println!("[TikTok Login] Streamlabs auth complete, stopping poll");
                            break;
                        }
                        UrlType::StreamlabsAuth => {
                            println!("[TikTok Login] On Streamlabs login page - waiting for auth redirect...");
                            continue;
                        }
                        UrlType::TikTokLoggedIn => {
                            println!("[TikTok Login] Detected cached TikTok login - redirecting to Streamlabs...");

                            // Check if we already have cookies from before
                            let cookies_guard = captured_cookies.lock();
                            let has_cookies = cookies_guard.is_some();
                            drop(cookies_guard);

                            if has_cookies {
                                println!("[TikTok Login] Already have TikTok cookies from previous capture");
                            } else {
                                // Try to inject script to get cookies from the logged-in page
                                let cookie_script = get_cookie_extraction_script();
                                let _ = window_clone.emit("inject-script", cookie_script);
                            }

                            let auth_url = build_streamlabs_auth_url(&code_challenge);
                            println!("[TikTok Login] Redirecting to: {}", auth_url);

                            // Use emit to send navigation script to the webview
                            let navigate_script = format!("window.location.href = '{}';", auth_url);
                            let _ = window_clone.emit("inject-script", navigate_script);

                            continue;
                        }
                        UrlType::TikTokLogin => {
                            println!("[TikTok Login] On login page - waiting for user to login...");
                            continue;
                        }
                        _ => {}
                    }
                }
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
        let result: std::result::Result<(), tauri::Error> = window.emit("inject-script", script);
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
