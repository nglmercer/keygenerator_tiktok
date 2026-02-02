#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#![allow(non_snake_case)]

// Module declarations
mod auth;
mod config;
mod api;
mod data_capture;
mod pkce;
mod url_utils;
mod cookie_interceptor;
mod credentials;
mod login_window;
mod stream;

// Re-export data capture types for easier use
pub use data_capture::CaptureState;
pub use data_capture::CapturedData;
pub use data_capture::CaptureConfig;
pub use data_capture::CaptureResult;

// Re-export login state for use in commands
pub use login_window::LoginState;

use tauri::State;
use reqwest::Client;

// ============================================================================
// Tauri Commands - Login Window Management
// ============================================================================

#[tauri::command]
async fn open_tiktok_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    login_window::open_tiktok_login_window(app, login_state.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn inject_cookie_interceptor(window: tauri::WebviewWindow) -> Result<serde_json::Value, String> {
    login_window::inject_cookie_interceptor(window)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_captured_credentials(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    Ok(login_window::get_captured_credentials(login_state.inner()))
}

#[tauri::command]
async fn close_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    login_window::close_login_window(app, login_state.inner())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn inject_interceptor_to_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    login_window::inject_interceptor_to_login_window(app, login_state.inner())
        .map_err(|e| e.to_string())
}

// ============================================================================
// Tauri Commands - Credential Management
// ============================================================================

#[tauri::command]
async fn save_credentials_to_file(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    let cookies_guard = login_state.captured_cookies.lock();
    let auth_code_guard = login_state.auth_code.lock();
    let code_challenge_guard = login_state.code_challenge.lock();

    if let Some(creds) = cookies_guard.as_ref() {
        // Save to cookies.json
        credentials::save_cookies(creds, None).map_err(|e| e.to_string())?;
        println!("[Credentials] Saved credentials to cookies.json");

        // Build complete credentials object
        let mut credentials = credentials::Credentials::new();
        credentials.cookies = creds.clone();
        credentials.auth_code = auth_code_guard.clone();
        credentials.code_challenge = code_challenge_guard.clone();

        // Save to credentials.json
        credentials::save_credentials(&credentials, None).map_err(|e| e.to_string())?;
        println!("[Credentials] Saved complete credentials to credentials.json");

        Ok(serde_json::json!({
            "success": true,
            "message": "Credentials saved to cookies.json and credentials.json",
            "cookies_path": credentials::COOKIES_FILE,
            "credentials_path": credentials::CREDENTIALS_FILE,
            "has_auth_code": auth_code_guard.is_some()
        }))
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "No credentials captured yet"
        }))
    }
}

// ============================================================================
// Tauri Commands - Authentication
// ============================================================================

#[tauri::command]
fn get_auth_url(state: State<'_, auth::AuthState>) -> String {
    let auth_manager = state.auth_manager.lock();
    auth_manager.get_auth_url()
}

#[tauri::command]
async fn retrieve_credentials(state: State<'_, auth::AuthState>) -> Result<serde_json::Value, String> {
    let tokens_path = {
        let auth_manager = state.auth_manager.lock();
        auth_manager.tokens_path.clone()
    };

    if let Ok(saved) = std::fs::read_to_string(&tokens_path) {
        if let Ok(parsed) = serde_json::from_str::<auth::AuthData>(&saved) {
            if let Some(token) = parsed.oauth_token {
                return Ok(serde_json::json!({
                    "success": true,
                    "oauth_token": token,
                    "source": "cache"
                }));
            }
        }
    }

    let (code_verifier, code_challenge, tokens_path_clone) = {
        let auth_manager = state.auth_manager.lock();
        (
            auth_manager.code_verifier.clone(),
            auth_manager.code_challenge.clone(),
            auth_manager.tokens_path.clone()
        )
    };

    let result = auth::AuthManager::async_retrieve_credentials(
        code_verifier,
        code_challenge,
        tokens_path_clone
    ).await.map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
async fn check_credentials(state: State<'_, auth::AuthState>) -> Result<serde_json::Value, String> {
    let auth_manager = state.auth_manager.lock();

    if let Ok(saved) = std::fs::read_to_string(&auth_manager.tokens_path) {
        if let Ok(parsed) = serde_json::from_str::<auth::AuthData>(&saved) {
            if let Some(token) = parsed.oauth_token {
                return Ok(serde_json::json!({
                    "ready": true,
                    "credentials": {
                        "oauth_token": token
                    }
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "ready": false
    }))
}

#[tauri::command]
async fn check_tiktok_login_state(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    println!("[TikTok Login] Checking existing TikTok login state...");

    // Check if we already have captured cookies
    {
        let cookies_guard = login_state.captured_cookies.lock();
        if let Some(creds) = cookies_guard.as_ref() {
            let has_cookies = creds.get("data")
                .and_then(|d| d.get("cookies"))
                .map(|c| c.is_object())
                .unwrap_or(false);

            if has_cookies {
                println!("[TikTok Login] Already have TikTok cookies in memory");
                return Ok(serde_json::json!({
                    "success": true,
                    "state": "tiktok_logged_in",
                    "source": "memory"
                }));
            }
        }
    }

    // Check if we have saved cookies.json
    if credentials::has_cookies_file(None) {
        println!("[TikTok Login] Found saved TikTok cookies in cookies.json");

        // Load and store in memory
        if let Ok(cookies) = credentials::load_cookies(None) {
            let mut cookies_guard = login_state.captured_cookies.lock();
            *cookies_guard = Some(cookies.clone());

            return Ok(serde_json::json!({
                "success": true,
                "state": "tiktok_logged_in",
                "source": "file"
            }));
        }
    }

    // Check if we already have auth code
    {
        let auth_code_guard = login_state.auth_code.lock();
        if let Some(auth_code) = auth_code_guard.as_ref() {
            println!("[TikTok Login] Found existing auth code");
            return Ok(serde_json::json!({
                "success": true,
                "state": "streamlabs_auth_pending",
                "auth_code": auth_code
            }));
        }
    }

    // Check if we have saved tokens (already authenticated)
    if credentials::has_tokens_file(None) {
        println!("[TikTok Login] Found existing Streamlabs token - already authenticated");
        return Ok(serde_json::json!({
            "success": true,
            "state": "fully_authenticated"
        }));
    }

    Ok(serde_json::json!({
        "success": false,
        "state": "not_logged_in"
    }))
}

#[tauri::command]
async fn complete_authentication(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    // Release guards before await
    let (auth_code, code_challenge) = {
        let auth_code_guard = login_state.auth_code.lock();
        let code_challenge_guard = login_state.code_challenge.lock();
        (
            auth_code_guard.clone(),
            code_challenge_guard.clone()
        )
    };

    if let (Some(auth_code), Some(code_challenge)) = (auth_code, code_challenge) {
        println!("[Auth] Exchanging auth code for token...");

        // Exchange code for token using reqwest
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| e.to_string())?;

        let token_response = client
            .post("https://streamlabs.com/api/v5/slobs/auth/token")
            .form(&[
                ("code", auth_code.as_str()),
                ("code_verifier", code_challenge.as_str()),
                ("grant_type", "authorization_code"),
                ("client_key", "awdjaq9ide8frtz"),
                ("redirect_uri", "https://streamlabs.com/tiktok/auth"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let token_json: serde_json::Value = token_response
            .json()
            .await
            .map_err(|e| e.to_string())?;

        println!("[Auth] Token response: {:?}", token_json);

        // Extract token
        let oauth_token = credentials::extract_oauth_token(&token_json);

        if let Some(token) = oauth_token {
            // Save tokens
            credentials::save_tokens(&token_json, None).map_err(|e| e.to_string())?;
            println!("[Auth] Tokens saved to tokens.json");

            // Also save credentials.json with all info
            let mut credentials = credentials::Credentials::new();
            credentials.oauth_token = Some(token.clone());
            credentials.tokens = Some(token_json.clone());

            credentials::save_credentials(&credentials, None).map_err(|e| e.to_string())?;
            println!("[Auth] Credentials saved to credentials.json");

            return Ok(serde_json::json!({
                "success": true,
                "message": "Authentication completed successfully",
                "oauth_token": token
            }));
        }
    }

    Ok(serde_json::json!({
        "success": false,
        "message": "No auth code available"
    }))
}

/// Exchanges the code_verifier for a Streamlabs OAuth token
///
/// This command implements the PKCE token exchange with the Streamlabs API.
/// It sends the code_verifier to the Streamlabs auth endpoint and receives
/// an OAuth token in response.
#[tauri::command]
async fn exchange_streamlabs_token(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    Ok(login_window::perform_streamlabs_token_exchange(login_state.inner()).await)
}

/// Gets the Streamlabs OAuth token from the login state
#[tauri::command]
async fn get_streamlabs_token(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    if let Some(token) = login_window::get_streamlabs_token(login_state.inner()) {
        Ok(serde_json::json!({
            "success": true,
            "token": token
        }))
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "No Streamlabs token available"
        }))
    }
}

// ============================================================================
// Tauri Commands - Stream Operations
// ============================================================================

#[tauri::command]
async fn stream_search(query: String) -> Result<serde_json::Value, String> {
    let categories = stream::stream_search(query).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(categories).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn stream_start(title: String, category: String) -> Result<serde_json::Value, String> {
    let response = stream::stream_start(title, category).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(response).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn stream_end() -> Result<bool, String> {
    stream::stream_end().await.map_err(|e| e.to_string())
}

// ============================================================================
// Tauri Commands - Test Commands
// ============================================================================

/// Get the JavaScript capture script for webview injection
#[tauri::command]
fn get_capture_script() -> Result<serde_json::Value, String> {
    let script = data_capture::generate_capture_script();
    Ok(serde_json::json!({
        "success": true,
        "script": script,
        "type": "capture"
    }))
}

/// Validate captured data
#[tauri::command]
async fn validate_captured_data(
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    println!("[Test] Validating captured data...");

    // Parse the data into CapturedData
    let captured_data = CapturedData {
        cookies: data.get("cookies").cloned().unwrap_or(serde_json::json!({})),
        local_storage: data.get("localStorage").cloned().unwrap_or(serde_json::json!({})),
        session_storage: data.get("sessionStorage").cloned().unwrap_or(serde_json::json!({})),
        headers: serde_json::json!({}),
        url: data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        timestamp: data.get("timestamp").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        login_detected: data.get("login_detected").and_then(|v| v.as_bool()).unwrap_or(false),
        streamlabs_code: data.get("streamlabs_code").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    let has_data = captured_data.has_data();
    let has_tiktok = captured_data.has_tiktok_data();
    let total_items = captured_data.total_items();

    Ok(serde_json::json!({
        "success": true,
        "validation": {
            "has_data": has_data,
            "has_tiktok_data": has_tiktok,
            "total_items": total_items,
            "is_valid": has_data && has_tiktok
        },
        "data": {
            "cookies_count": captured_data.cookies.as_object().map(|o| o.len()).unwrap_or(0),
            "localStorage_count": captured_data.local_storage.as_object().map(|o| o.len()).unwrap_or(0),
            "sessionStorage_count": captured_data.session_storage.as_object().map(|o| o.len()).unwrap_or(0)
        }
    }))
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() {
    let context = tauri::generate_context!();

    tauri::Builder::default()
        .manage(auth::AuthState::new())
        .manage(LoginState::new())
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            get_auth_url,
            retrieve_credentials,
            check_credentials,
            check_tiktok_login_state,
            complete_authentication,
            exchange_streamlabs_token,
            get_streamlabs_token,
            // Login window commands
            open_tiktok_login_window,
            inject_cookie_interceptor,
            get_captured_credentials,
            close_login_window,
            inject_interceptor_to_login_window,
            // Credential commands
            save_credentials_to_file,
            // Stream commands
            stream_search,
            stream_start,
            stream_end,
            // Test commands
            get_capture_script,
            validate_captured_data
        ])
        .run(context)
        .expect("Error running Tauri application");
}
