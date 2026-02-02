#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#![allow(non_snake_case)]

mod auth;
mod config;
mod api;

use tauri::{Emitter, Manager, Listener, State, WebviewWindowBuilder, WebviewUrl};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// State for tracking login window
pub struct LoginState {
    pub login_window_label: Mutex<String>,
    pub captured_cookies: Arc<Mutex<Option<serde_json::Value>>>,
    pub login_complete: Arc<AtomicBool>,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            login_window_label: Mutex::new(String::new()),
            captured_cookies: Arc::new(Mutex::new(None)),
            login_complete: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[tauri::command]
async fn open_tiktok_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    println!("[TikTok Login] Opening login window...");

    let login_url = "https://www.tiktok.com/login";

    // Try to get existing window first
    if let Some(window) = app.get_webview_window("tiktok-login") {
        let _ = window.set_focus();
        println!("[TikTok Login] Login window already exists, focusing...");
        return Ok(serde_json::json!({
            "success": true,
            "message": "Login window already open"
        }));
    }

    // Create login window using tauri.conf.json configuration
    let login_window = WebviewWindowBuilder::new(
        &app,
        "tiktok-login",
        WebviewUrl::External(login_url.parse().unwrap())
    )
    .title("TikTok Login")
    .inner_size(450.0, 700.0)
    .min_inner_size(450.0, 700.0)
    .resizable(false)
    .center()
    .always_on_top(true)
    .build()
    .map_err(|e| format!("Failed to create login window: {}", e))?;

    println!("[TikTok Login] Login window created successfully");

    // Store window label
    {
        let mut label_guard = login_state.login_window_label.lock();
        *label_guard = "tiktok-login".to_string();
    }

    // Clone the data for the closure
    let captured_cookies = login_state.captured_cookies.clone();
    let login_complete = login_state.login_complete.clone();

    // Listen for messages from the login window webview
    let _listener_id = login_window.listen("credentials-captured", move |event| {
        println!("[TikTok Login] Credentials captured event received");
        login_complete.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // event.payload() returns &str
        let payload = event.payload();
        println!("[TikTok Login] Raw payload: {}", payload);
        
        // Parse the JSON payload
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
            let mut cookies_guard = captured_cookies.lock();
            *cookies_guard = Some(data);
            println!("[TikTok Login] Credentials parsed and stored");
        }
    });

    Ok(serde_json::json!({
        "success": true,
        "message": "Login window opened",
        "window_label": "tiktok-login"
    }))
}

#[tauri::command]
async fn inject_cookie_interceptor(window: tauri::Window) -> Result<serde_json::Value, String> {
    let script = get_cookie_interceptor_script();

    // Use emit to send script to frontend for injection
    let result = window.emit("inject-script", script);
    
    match result {
        Ok(_) => {
            println!("[Cookie Interceptor] Script injection requested");
            Ok(serde_json::json!({
                "success": true,
                "message": "Cookie interceptor injection requested"
            }))
        }
        Err(e) => {
            println!("[Cookie Interceptor] Failed to emit: {}", e);
            Err(format!("Failed to inject script: {}", e))
        }
    }
}

#[tauri::command]
async fn get_captured_credentials(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    let cookies = login_state.captured_cookies.lock();
    if let Some(creds) = cookies.as_ref() {
        Ok(creds.clone())
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "No credentials captured yet"
        }))
    }
}

fn get_cookie_interceptor_script() -> String {
    r#"
    (function() {
        console.log('[Cookie Interceptor] Starting interceptor...');

        // Storage for captured data
        window.__tiktok_credentials = {
            cookies: {},
            localStorage: {},
            sessionStorage: {},
            headers: {},
            fetch_intercepted: [],
            xhr_intercepted: [],
            login_detected: false,
            login_time: null
        };

        // Capture all cookies
        function captureCookies() {
            const cookies = document.cookie.split(';');
            const cookieObj = {};
            cookies.forEach(cookie => {
                const [name, value] = cookie.trim().split('=');
                if (name) {
                    cookieObj[name] = value || '';
                }
            });
            window.__tiktok_credentials.cookies = cookieObj;
            console.log('[Cookie Interceptor] Cookies captured:', Object.keys(cookieObj).length);
            return cookieObj;
        }

        // Capture localStorage
        function captureLocalStorage() {
            try {
                const ls = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    ls[key] = localStorage.getItem(key);
                }
                window.__tiktok_credentials.localStorage = ls;
                console.log('[Cookie Interceptor] localStorage captured:', Object.keys(ls).length);
            } catch (e) {
                console.log('[Cookie Interceptor] localStorage capture failed:', e);
            }
        }

        // Capture sessionStorage
        function captureSessionStorage() {
            try {
                const ss = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const key = sessionStorage.key(i);
                    ss[key] = sessionStorage.getItem(key);
                }
                window.__tiktok_credentials.sessionStorage = ss;
                console.log('[Cookie Interceptor] sessionStorage captured:', Object.keys(ss).length);
            } catch (e) {
                console.log('[Cookie Interceptor] sessionStorage capture failed:', e);
            }
        }

        // Intercept fetch requests
        const originalFetch = window.fetch;
        window.fetch = async function(...args) {
            const [resource, config] = args;
            console.log('[Cookie Interceptor] Fetch intercepted:', resource);

            try {
                const response = await originalFetch.apply(this, args);

                // Clone response to read body
                const cloned = response.clone();
                window.__tiktok_credentials.fetch_intercepted.push({
                    url: resource.toString(),
                    method: config?.method || 'GET',
                    status: response.status,
                    timestamp: new Date().toISOString()
                });

                return response;
            } catch (error) {
                console.log('[Cookie Interceptor] Fetch error:', error);
                throw error;
            }
        };

        // Intercept XMLHttpRequest
        const originalXHR = window.XMLHttpRequest;
        window.XMLHttpRequest = function() {
            const xhr = new originalXHR();
            xhr.addEventListener('load', function() {
                console.log('[Cookie Interceptor] XHR loaded:', xhr.readyState, xhr.status);
                window.__tiktok_credentials.xhr_intercepted.push({
                    url: xhr.responseURL,
                    status: xhr.status,
                    timestamp: new Date().toISOString()
                });
            });
            return xhr;
        };

        // Detect login page and login completion
        function detectLoginState() {
            const url = window.location.href;
            const pathname = window.location.pathname;

            // Check if on login page
            if (pathname.includes('/login') || url.includes('login')) {
                console.log('[Cookie Interceptor] On login page');
            }

            // Check for successful login indicators
            const userElements = document.querySelector('[data-e2e="user-avatar"], [data-e2e="user-info"]');
            if (userElements) {
                console.log('[Cookie Interceptor] User element detected - likely logged in');
                window.__tiktok_credentials.login_detected = true;
                window.__tiktok_credentials.login_time = new Date().toISOString();
            }
        }

        // Monitor for URL changes (SPA navigation)
        function monitorNavigation() {
            let currentUrl = window.location.href;
            const observer = new MutationObserver(() => {
                if (window.location.href !== currentUrl) {
                    console.log('[Cookie Interceptor] URL changed:', currentUrl, '->', window.location.href);
                    currentUrl = window.location.href;
                    detectLoginState();
                    captureAllData();
                }
            });

            observer.observe(document.body, { childList: true, subtree: true });
        }

        // Capture all data
        function captureAllData() {
            captureCookies();
            captureLocalStorage();
            captureSessionStorage();
            detectLoginState();
        }

        // Send data to Tauri
        function sendDataToTauri() {
            const data = {
                ...window.__tiktok_credentials,
                url: window.location.href,
                timestamp: new Date().toISOString()
            };

            if (window.__TAURI__) {
                window.__TAURI__.event.emit('credentials-captured', JSON.stringify(data));
                console.log('[Cookie Interceptor] Data sent via Tauri event');
            } else {
                window.parent.postMessage({ type: 'credentials-captured', data: data }, '*');
                console.log('[Cookie Interceptor] Data sent via postMessage');
            }
        }

        // Create global function to manually trigger capture
        window.__captureTikTokCredentials = function() {
            console.log('[Cookie Interceptor] Manual capture triggered');
            captureAllData();
            sendDataToTauri();
            return window.__tiktok_credentials;
        };

        // Initial capture
        captureAllData();
        monitorNavigation();

        // Periodic capture (every 2 seconds)
        setInterval(() => {
            captureAllData();
            if (window.__tiktok_credentials.login_detected) {
                sendDataToTauri();
            }
        }, 2000);

        // Listen for messages from parent window
        window.addEventListener('message', function(event) {
            if (event.data && event.data.type === 'capture-credentials') {
                console.log('[Cookie Interceptor] Capture requested via message');
                window.__captureTikTokCredentials();
            }
            if (event.data && event.data.type === 'get-credentials') {
                console.log('[Cookie Interceptor] Get credentials requested');
                event.source.postMessage({
                    type: 'credentials-result',
                    data: window.__tiktok_credentials
                }, '*');
            }
        });

        // Listen for form submissions on login page
        document.addEventListener('submit', function(e) {
            const form = e.target;
            console.log('[Cookie Interceptor] Form submitted:', form.action);
            setTimeout(() => {
                console.log('[Cookie Interceptor] Form submitted, re-capturing...');
                captureAllData();
                setTimeout(captureAllData, 3000);
            }, 1000);
        });

        console.log('[Cookie Interceptor] Interceptor initialized');
    })();
    "#.to_string()
}

#[tauri::command]
async fn close_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    let label_guard = login_state.login_window_label.lock();
    let label = label_guard.clone();
    
    if !label.is_empty() {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
            println!("[TikTok Login] Closed login window: {}", label);
            
            // Clear the label
            let mut label_mut = login_state.login_window_label.lock();
            *label_mut = String::new();
            
            return Ok(serde_json::json!({
                "success": true,
                "message": format!("Login window {} closed", label)
            }));
        }
    }
    
    Ok(serde_json::json!({
        "success": false,
        "message": "No login window was open"
    }))
}

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
async fn inject_credential_extractor(window: tauri::Window) {
    let script = r#"
    (function() {
        function extractCredentials() {
            const credentials = {
                cookies: {},
                localStorage: {},
                sessionStorage: {}
            };

            if (document.cookie) {
                document.cookie.split(';').forEach(cookie => {
                    const [name, value] = cookie.trim().split('=');
                    if (name) {
                        credentials.cookies[name] = value || '';
                    }
                });
            }

            try {
                Object.keys(localStorage).forEach(key => {
                    credentials.localStorage[key] = localStorage.getItem(key);
                });
            } catch (e) {}

            try {
                Object.keys(sessionStorage).forEach(key => {
                    credentials.sessionStorage[key] = sessionStorage.getItem(key);
                });
            } catch (e) {}

            if (window.__TAURI__) {
                window.__TAURI__.event.emit('credentials', credentials);
            } else {
                window.postMessage({ type: 'credentials', data: credentials }, '*');
            }

            return credentials;
        }

        window.__extractCredentials = extractCredentials;
        window.addEventListener('load', function() {
            setTimeout(extractCredentials, 2000);
        });
        window.addEventListener('message', function(event) {
            if (event.data && event.data.type === 'extract_credentials') {
                const credentials = extractCredentials();
                event.source.postMessage({ type: 'credentials_result', data: credentials }, '*');
            }
        });
        console.log('[Credentials] Extractor script injected');
    })();
    "#;

    let _ = window.emit("inject-script", script);
}

#[tauri::command]
async fn navigate_to_url(window: tauri::Window, url: String) -> Result<(), String> {
    let _ = window.emit("navigate-url", url);
    Ok(())
}

#[tauri::command]
async fn get_page_credentials(window: tauri::Window) -> Result<serde_json::Value, String> {
    let _ = window.emit("get-credentials", ());
    Ok(serde_json::json!({
        "error": "Use frontend-based credential extraction",
        "note": "Tauri 2.x requires different approach for JS execution"
    }))
}

#[tauri::command]
fn save_token(state: State<'_, auth::AuthState>, token: String) -> Result<(), String> {
    let auth_manager = state.auth_manager.lock();

    let token_data = auth::AuthData {
        success: true,
        data: None,
        oauth_token: Some(token.clone()),
    };

    std::fs::write(
        &auth_manager.tokens_path,
        serde_json::to_string_pretty(&token_data).map_err(|e| e.to_string())?
    ).map_err(|e| e.to_string())?;

    Ok(())
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
async fn inject_interceptor_to_login_window(
    app: tauri::AppHandle,
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    let label_guard = login_state.login_window_label.lock();
    let label = label_guard.clone();
    
    if label.is_empty() {
        return Err("No login window open".to_string());
    }
    
    if let Some(window) = app.get_webview_window(&label) {
        let script = get_cookie_interceptor_script();
        let result = window.emit("inject-script", script);
        match result {
            Ok(_) => {
                println!("[Cookie Interceptor] Script injection requested for login window");
                Ok(serde_json::json!({
                    "success": true,
                    "message": "Interceptor injection requested"
                }))
            }
            Err(e) => {
                println!("[Cookie Interceptor] Failed to emit: {}", e);
                Err(format!("Failed to inject script: {}", e))
            }
        }
    } else {
        Err("Login window not found".to_string())
    }
}

#[tauri::command]
async fn save_credentials_to_file(
    login_state: State<'_, LoginState>,
) -> Result<serde_json::Value, String> {
    let cookies_guard = login_state.captured_cookies.lock();
    
    if let Some(creds) = cookies_guard.as_ref() {
        // Save to cookies.json
        let cookies_path = std::path::Path::new("cookies.json");
        std::fs::write(
            cookies_path,
            serde_json::to_string_pretty(creds).map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;
        
        println!("[Credentials] Saved credentials to cookies.json");
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Credentials saved to cookies.json",
            "path": cookies_path.to_string_lossy().as_ref()
        }))
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "No credentials captured yet"
        }))
    }
}

fn main() {
    let context = tauri::generate_context!();

    tauri::Builder::default()
        .manage(auth::AuthState::new())
        .manage(LoginState::new())
        .invoke_handler(tauri::generate_handler![
            get_auth_url,
            retrieve_credentials,
            inject_credential_extractor,
            navigate_to_url,
            get_page_credentials,
            save_token,
            check_credentials,
            open_tiktok_login_window,
            inject_cookie_interceptor,
            get_captured_credentials,
            close_login_window
        ])
        .run(context)
        .expect("Error running Tauri application");
}
