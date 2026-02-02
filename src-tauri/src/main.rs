#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#![allow(non_snake_case)]

mod auth;
mod config;
mod api;
mod data_capture;
mod test_utils;
mod integration_tests;

// Re-export data capture types for easier use
pub use data_capture::CaptureState;
pub use data_capture::CapturedData;
pub use data_capture::CaptureConfig;
pub use data_capture::CaptureResult;

use tauri::{Emitter, Manager, Listener, State, WebviewWindowBuilder, WebviewUrl};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::fs;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use sha2::Digest;

// State for tracking login window
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

fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 64];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    
    STANDARD.encode(&result)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
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

    // Create login window
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
    let captured_local_storage = login_state.captured_local_storage.clone();
    let captured_session_storage = login_state.captured_session_storage.clone();
    let tiktok_session_data = login_state.tiktok_session_data.clone();
    let captured_cookies_for_nav = login_state.captured_cookies.clone();
    let captured_cookies_for_cache = login_state.captured_cookies.clone();
    let auth_code = login_state.auth_code.clone();
    let login_complete = login_state.login_complete.clone();
    let code_challenge = login_state.code_challenge.clone();
    
    // Generate code verifier and challenge
    let code_verifier = generate_code_verifier();
    let code_challenge_str = generate_code_challenge(&code_verifier);
    
    // Store code challenge for later token exchange
    {
        let mut challenge_guard = code_challenge.lock();
        *challenge_guard = Some(code_challenge_str.clone());
    }

    // Clone for navigation listener
    let auth_code_for_nav = auth_code.clone();
    let auth_code_for_poll = auth_code.clone();
    
    // Listen for credentials captured event - stores all TikTok data
    let _listener_id = login_window.listen("credentials-captured", move |event| {
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
            println!("[TikTok Login] Cookies: {}", data.get("cookies").map(|c| c.as_object().map(|o| o.len()).unwrap_or(0)).unwrap_or(0));
            println!("[TikTok Login] localStorage: {}", data.get("localStorage").map(|l| l.as_object().map(|o| o.len()).unwrap_or(0)).unwrap_or(0));
            println!("[TikTok Login] sessionStorage: {}", data.get("sessionStorage").map(|s| s.as_object().map(|o| o.len()).unwrap_or(0)).unwrap_or(0));
        }
    });
    
    // Listen for tiktok-cookies message (for cached login detection)
    let _listener_id_cache = login_window.listen("tiktok-cookies", move |event| {
        println!("[TikTok Login] TikTok cookies message received");
        
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let cookies = if let Some(cookies_obj) = data.get("cookies") {
                cookies_obj.clone()
            } else if let Some(cookies_obj) = data.as_object() {
                serde_json::json!(cookies_obj)
            } else {
                serde_json::json!({})
            };
            
            if cookies.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                let creds_json = serde_json::json!({
                    "success": true,
                    "data": {
                        "cookies": cookies
                    }
                });
                
                let mut cookies_guard = captured_cookies_for_cache.lock();
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

    // Listen for URL changes to detect login and navigate to Streamlabs auth
    let login_window_clone = login_window.clone();
    let auth_code_clone = auth_code_for_nav.clone();
    let code_challenge_clone = code_challenge_str.clone();
    let captured_cookies_clone = captured_cookies_for_nav.clone();
    
    let _listener_id2 = login_window.listen("navigation", move |event| {
        let url = event.payload();
        println!("[TikTok Login] Navigation detected: {}", url);
        
        // Check if URL contains success code (from Streamlabs redirect)
        // Streamlabs redirect format: https://streamlabs.com/m/login?code=XXX&state=YYY
        if url.contains("streamlabs.com") && url.contains("code=") {
            println!("[TikTok Login] Streamlabs redirect detected!");
            
            // Extract auth code properly - find 'code=' and then find '&' or end of string
            if let Some(code_start) = url.find("code=") {
                let code_section = &url[code_start + 5..];
                let code_end = code_section.find('&').unwrap_or(code_section.len());
                let code = code_section[..code_end].to_string();
                println!("[TikTok Login] Auth code found: {}", code);
                
                let mut auth_code_guard = auth_code_clone.lock();
                *auth_code_guard = Some(code.clone());
                
                // Emit event for frontend
                let _ = login_window_clone.emit("auth-code-received", code);
            }
            return;
        }
        
        // Check if we're already on Streamlabs auth page (already logged in to Streamlabs)
        // This handles the case where Streamlabs is cached and immediately redirects with code
        let is_streamlabs_auth = url.contains("streamlabs.com/m/login") || 
                                   url.contains("streamlabs.com/tiktok/auth");
        if is_streamlabs_auth {
            println!("[TikTok Login] Detected Streamlabs auth page - waiting for redirect...");
            return;
        }
        
        // Check if we're on TikTok main page (logged in) - but not Streamlabs
        // This includes various TikTok logged-in states
        let is_tiktok_logged_in = (url.contains("tiktok.com") && 
                                   !url.contains("/login") && 
                                   !url.contains("webcast") &&
                                   !url.contains("streamlabs")) ||
                                  url.contains("tiktok.com/foryou") ||
                                  url.contains("tiktok.com/discover") ||
                                  url.contains("tiktok.com/") && !url.contains("/login");
        
        if is_tiktok_logged_in {
            println!("[TikTok Login] Detected TikTok main page - user already logged in, redirecting to Streamlabs...");
            
            // If we already have cookies captured, save them
            let cookies_guard = captured_cookies_clone.lock();
            if let Some(creds) = cookies_guard.as_ref() {
                if let Err(e) = fs::write("cookies.json", serde_json::to_string_pretty(creds).unwrap()) {
                    println!("[TikTok Login] Failed to save cookies: {}", e);
                } else {
                    println!("[TikTok Login] Cookies saved to cookies.json");
                }
            }
            
            // Navigate to Streamlabs auth URL
            let auth_url = format!(
                "https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge={}",
                code_challenge_clone
            );
            println!("[TikTok Login] Redirecting to: {}", auth_url);
            
            // Parse URL and navigate
            if let Ok(url) = auth_url.parse() {
                let _ = login_window_clone.clone().navigate(url);
            }
        }
    });

    // Inject the cookie interceptor after a short delay
    {
        let login_window = login_window.clone();
        let code_challenge_nav = code_challenge_str.clone();
        let auth_code_poll = auth_code_for_poll.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let script = get_cookie_interceptor_script();
            let _ = login_window.emit("inject-script", script);
            println!("[TikTok Login] Cookie interceptor injected");
            
            // Start URL polling as fallback (every 500ms)
            let window_clone = login_window.clone();
            let mut last_url = String::new();
            let mut attempts = 0;
            let max_attempts = 240; // 2 minutes
            let code_challenge_poll = code_challenge_nav.clone();
            let auth_code_poll = auth_code_poll.clone();
            let captured_cookies_poll = captured_cookies_for_nav.clone();
            
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    attempts += 1;
                    
                    if attempts >= max_attempts {
                        println!("[TikTok Login] URL polling timeout");
                        break;
                    }
                    
                    // Get current URL from the window (sync method)
                    let current_url = window_clone.url();
                    let url_str = if let Ok(url) = current_url {
                        url.to_string()
                    } else {
                        continue;
                    };
                    
                    if url_str != last_url {
                        println!("[TikTok Login] URL changed to: {}", url_str);
                        last_url = url_str.clone();
                        
                        // Check for Streamlabs redirect with code (FINAL SUCCESS STATE)
                        if url_str.contains("streamlabs.com") && url_str.contains("code=") {
                            println!("[TikTok Login] Streamlabs redirect with code detected!");
                            
                            // Extract auth code
                            if let Some(code_start) = url_str.find("code=") {
                                let code_section = &url_str[code_start + 5..];
                                let code_end = code_section.find('&').unwrap_or(code_section.len());
                                let code = code_section[..code_end].to_string();
                                println!("[TikTok Login] Auth code found: {}", code);
                                
                                // Store auth code in state
                                let mut auth_code_store = auth_code_poll.lock();
                                *auth_code_store = Some(code.clone());
                            }
                            
                            // Emit Tauri event for frontend to handle
                            let _ = window_clone.emit("auth-code-received", url_str.as_str());
                            
                            // Also inject script to postMessage to parent
                            let postmessage_script = format!(r#"
                                (function() {{
                                    window.parent.postMessage({{ type: 'auth-code-received', url: '{}' }}, '*');
                                }})();
                            "#, url_str);
                            let _ = window_clone.clone().eval(&postmessage_script);
                            
                            println!("[TikTok Login] Streamlabs auth complete, stopping poll");
                            break;
                        }
                        
                        // Check if we're on Streamlabs login page (may need to wait for redirect)
                        if url_str.contains("streamlabs.com/m/login") {
                            println!("[TikTok Login] On Streamlabs login page - waiting for auth redirect...");
                            // Continue polling to catch the code redirect
                            continue;
                        }
                        
                        // Check if TikTok main page (already logged in via cache)
                        let is_tiktok_logged_in = (url_str.contains("tiktok.com") && 
                                                   !url_str.contains("/login") && 
                                                   !url_str.contains("streamlabs")) ||
                                                  url_str.contains("tiktok.com/foryou") ||
                                                  url_str.contains("tiktok.com/discover");
                        
                        if is_tiktok_logged_in {
                            println!("[TikTok Login] Detected cached TikTok login - redirecting to Streamlabs...");
                            
                            // Check if we already have cookies from before
                            let cookies_guard = captured_cookies_poll.lock();
                            let has_cookies = cookies_guard.is_some();
                            drop(cookies_guard);
                            
                            if has_cookies {
                                println!("[TikTok Login] Already have TikTok cookies from previous capture");
                            } else {
                                // Try to inject script to get cookies from the logged-in page
                                let cookie_script = r#"
                                    (function() {
                                        const cookies = document.cookie.split(';').reduce((acc, cookie) => {
                                            const [name, value] = cookie.trim().split('=');
                                            if (name && value) acc[name] = value;
                                            return acc;
                                        }, {});
                                        window.parent.postMessage({ type: 'tiktok-cookies', cookies: cookies }, '*');
                                    })();
                                "#;
                                let _ = window_clone.clone().eval(cookie_script);
                            }
                            
                            let auth_url = format!(
                                "https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge={}",
                                code_challenge_poll
                            );
                            println!("[TikTok Login] Redirecting to: {}", auth_url);
                            
                            if let Ok(parsed_url) = auth_url.parse() {
                                let _ = window_clone.navigate(parsed_url);
                            }
                            
                            continue;
                        }
                        
                        // Check for TikTok login page - user needs to login
                        if url_str.contains("tiktok.com/login") {
                            println!("[TikTok Login] On login page - waiting for user to login...");
                            continue;
                        }
                    }
                }
            });
        });
    }

    Ok(serde_json::json!({
        "success": true,
        "message": "Login window opened",
        "window_label": "tiktok-login"
    }))
}

#[tauri::command]
async fn inject_cookie_interceptor(window: tauri::Window) -> Result<serde_json::Value, String> {
    let script = get_cookie_interceptor_script();
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

        window.__tiktok_credentials = {
            cookies: {},
            localStorage: {},
            sessionStorage: {},
            headers: {},
            fetch_intercepted: [],
            xhr_intercepted: [],
            login_detected: false,
            login_time: null,
            streamlabs_code: null
        };

        let lastCookieCount = 0;
        let loginCheckCount = 0;
        
        function captureCookies() {
            const cookies = document.cookie.split(';');
            const cookieObj = {};
            let cookieCount = 0;
            cookies.forEach(cookie => {
                const trimmed = cookie.trim();
                if (trimmed) {
                    cookieCount++;
                    const eqIndex = trimmed.indexOf('=');
                    const name = eqIndex > 0 ? trimmed.substring(0, eqIndex) : trimmed;
                    const value = eqIndex > 0 ? trimmed.substring(eqIndex + 1) : '';
                    cookieObj[name] = value;
                }
            });
            window.__tiktok_credentials.cookies = cookieObj;
            console.log('[Cookie Interceptor] Cookies captured:', cookieCount);
            return cookieObj;
        }

        function captureLocalStorage() {
            try {
                const ls = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    ls[key] = localStorage.getItem(key);
                }
                window.__tiktok_credentials.localStorage = ls;
                console.log('[Cookie Interceptor] localStorage captured:', Object.keys(ls).length);
                
                // Check for TikTok-specific storage keys that indicate login
                const loginKeys = Object.keys(ls).filter(k => 
                    k.toLowerCase().includes('user') || 
                    k.toLowerCase().includes('session') ||
                    k.toLowerCase().includes('auth') ||
                    k.toLowerCase().includes('token') ||
                    k.startsWith('tt')
                );
                if (loginKeys.length > 0) {
                    console.log('[Cookie Interceptor] Found potential login keys:', loginKeys);
                }
                return ls;
            } catch (e) {
                console.log('[Cookie Interceptor] localStorage capture failed:', e);
                return {};
            }
        }

        function captureSessionStorage() {
            try {
                const ss = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const key = sessionStorage.key(i);
                    ss[key] = sessionStorage.getItem(key);
                }
                window.__tiktok_credentials.sessionStorage = ss;
                console.log('[Cookie Interceptor] sessionStorage captured:', Object.keys(ss).length);
                return ss;
            } catch (e) {
                console.log('[Cookie Interceptor] sessionStorage capture failed:', e);
                return {};
            }
        }

        function checkTikTokLoginState() {
            const url = window.location.href;
            const pathname = window.location.pathname;
            const hostname = window.location.hostname;
            
            console.log('[Cookie Interceptor] Checking login state:', hostname, pathname);
            
            // Check if we're on TikTok domain
            const isTikTok = hostname.includes('tiktok.com');
            
            if (isTikTok) {
                // Check if NOT on login page
                const isLoginPage = url.includes('/login') || url.includes('/register');
                
                if (!isLoginPage) {
                    // Could be logged in - check multiple indicators
                    const indicators = [];
                    
                    // Check URL patterns
                    if (pathname === '/' || pathname.startsWith('/@') || url.includes('/foryou') || url.includes('/discover')) {
                        indicators.push('url_pattern');
                    }
                    
                    // Check for user avatar element
                    const userAvatar = document.querySelector('[data-e2e="user-avatar"], [data-e2e="user-info"], [data-e2e="user-profile"]');
                    if (userAvatar) {
                        indicators.push('user_element');
                    }
                    
                    // Check for logged-in user info in localStorage
                    const ls = window.__tiktok_credentials.localStorage;
                    const hasUserData = Object.keys(ls).some(k => 
                        k.includes('user') || k.includes('profile') || k.includes('session')
                    );
                    if (hasUserData) {
                        indicators.push('localStorage_user_data');
                    }
                    
                    // Check for cookies that indicate login
                    const cookies = window.__tiktok_credentials.cookies;
                    const hasLoginCookies = Object.keys(cookies).some(k => 
                        k.toLowerCase().includes('session') || 
                        k.toLowerCase().includes('auth') ||
                        k.toLowerCase().includes('user')
                    );
                    if (hasLoginCookies) {
                        indicators.push('login_cookies');
                    }
                    
                    console.log('[Cookie Interceptor] Login indicators:', indicators);
                    
                    // Require at least 1 indicator for reliable detection (since user may already be logged in)
                    if (indicators.length >= 1) {
                        window.__tiktok_credentials.login_detected = true;
                        window.__tiktok_credentials.login_time = new Date().toISOString();
                        console.log('[Cookie Interceptor] Login DETECTED on TikTok');
                        return true;
                    }
                }
            }
            
            // Check for Streamlabs auth code
            if (hostname.includes('streamlabs.com') && url.includes('code=')) {
                console.log('[Cookie Interceptor] Streamlabs auth code detected in URL');
                const urlParams = new URLSearchParams(url.split('?')[1]);
                const code = urlParams.get('code');
                if (code) {
                    window.__tiktok_credentials.streamlabs_code = code;
                    console.log('[Cookie Interceptor] Auth code captured:', code.substring(0, 20) + '...');
                    return 'streamlabs_code';
                }
            }
            
            return false;
        }

        // Check for existing saved session (already logged in)
        function checkExistingSession() {
            console.log('[Cookie Interceptor] Checking for existing session...');
            
            const cookies = captureCookies();
            const ls = captureLocalStorage();
            const ss = captureSessionStorage();
            
            // TikTok stores session in cookies and localStorage
            // Common session cookies: tt_webid, tt_webid_v2, sessionid, sessionid_ss, tt_target_id
            const sessionCookieNames = ['tt_webid', 'tt_webid_v2', 'sessionid', 'sessionid_ss', 'tt_target_id', 's_v_web_id', 'ttwid'];
            const hasSessionCookies = sessionCookieNames.some(name => cookies[name]);
            
            // Check localStorage for user data
            const userStorageKeys = Object.keys(ls).filter(k => 
                k.toLowerCase().includes('user') ||
                k.toLowerCase().includes('session') ||
                k.toLowerCase().includes('profile') ||
                k.toLowerCase().includes('device_id') ||
                k.startsWith('tt_login')
            );
            
            console.log('[Cookie Interceptor] Session cookies:', hasSessionCookies);
            console.log('[Cookie Interceptor] User storage keys:', userStorageKeys);
            
            // If we have session cookies OR user storage keys, user is likely logged in
            if (hasSessionCookies || userStorageKeys.length > 0) {
                console.log('[Cookie Interceptor] Existing session detected!');
                window.__tiktok_credentials.login_detected = true;
                window.__tiktok_credentials.login_time = new Date().toISOString();
                
                // Update storage with captured data
                window.__tiktok_credentials.cookies = cookies;
                window.__tiktok_credentials.localStorage = ls;
                window.__tiktok_credentials.sessionStorage = ss;
                
                return true;
            }
            
            return false;
        }

        function captureAllData() {
            const cookies = captureCookies();
            const ls = captureLocalStorage();
            const ss = captureSessionStorage();
            
            // Detect login state
            const loginState = checkTikTokLoginState();
            
            return {
                cookies,
                localStorage: ls,
                sessionStorage: ss,
                loginState
            };
        }

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

        window.__captureTikTokCredentials = function() {
            console.log('[Cookie Interceptor] Manual capture triggered');
            captureAllData();
            sendDataToTauri();
            return window.__tiktok_credentials;
        };

        // Monitor for localStorage changes (important for login detection)
        const lsObserver = new MutationObserver((mutations) => {
            const ls = captureLocalStorage();
            console.log('[Cookie Interceptor] localStorage changed, rechecking login state');
            
            // Check if this change indicates login
            const loginState = checkTikTokLoginState();
            if (loginState === true) {
                console.log('[Cookie Interceptor] Login detected via localStorage change!');
                captureAllData();
                sendDataToTauri();
            }
        });
        
        try {
            lsObserver.observe(localStorage, { attributes: true });
        } catch (e) {
            console.log('[Cookie Interceptor] Cannot observe localStorage directly, using interval');
        }

        // Monitor for URL changes
        let currentUrl = window.location.href;
        const observer = new MutationObserver(() => {
            const newUrl = window.location.href;
            if (newUrl !== currentUrl) {
                console.log('[Cookie Interceptor] URL changed:', currentUrl, '->', newUrl);
                currentUrl = newUrl;
                
                // Emit navigation event for Tauri to handle
                if (window.__TAURI__) {
                    window.__TAURI__.event.emit('navigation', newUrl);
                }
                
                // Capture data on URL change
                captureAllData();
                
                // Check for login or Streamlabs code
                const loginState = checkTikTokLoginState();
                if (loginState === true) {
                    console.log('[Cookie Interceptor] Login detected on URL change!');
                    sendDataToTauri();
                } else if (loginState === 'streamlabs_code') {
                    console.log('[Cookie Interceptor] Streamlabs code received!');
                    sendDataToTauri();
                }
            }
        });

        observer.observe(document.body, { childList: true, subtree: true });

        // Also use popstate for SPA navigation
        window.addEventListener('popstate', () => {
            const newUrl = window.location.href;
            if (newUrl !== currentUrl) {
                console.log('[Cookie Interceptor] popstate URL change:', currentUrl, '->', newUrl);
                currentUrl = newUrl;
                captureAllData();
                const loginState = checkTikTokLoginState();
                if (loginState === true || loginState === 'streamlabs_code') {
                    sendDataToTauri();
                }
            }
        });

        // Initial capture - check for existing session (already logged in)
        captureAllData();
        
        // Check for existing saved session
        const hasExistingSession = checkExistingSession();
        if (hasExistingSession) {
            console.log('[Cookie Interceptor] Existing session detected - sending credentials and triggering redirect!');
            sendDataToTauri();
            
            // Emit navigation event to trigger Rust-side redirect
            if (window.__TAURI__) {
                window.__TAURI__.event.emit('navigation', window.location.href);
            }
        } else {
            sendDataToTauri();
        }

        // Periodic capture - more frequent initially
        let captureInterval = 0;
        const intervalId = setInterval(() => {
            captureInterval++;
            captureAllData();
            
            // Send data if login detected
            if (window.__tiktok_credentials.login_detected) {
                sendDataToTauri();
            }
            
            // Also check for Streamlabs code periodically
            const url = window.location.href;
            if (url.includes('streamlabs.com') && url.includes('code=')) {
                checkTikTokLoginState();
                if (window.__tiktok_credentials.streamlabs_code) {
                    sendDataToTauri();
                }
            }
            
            // Reduce frequency after first 10 seconds
            if (captureInterval > 5 && intervalId) {
                clearInterval(intervalId);
            }
        }, 1000);

        // Listen for messages from parent window
        window.addEventListener('message', function(event) {
            if (event.data && event.data.type === 'capture-credentials') {
                console.log('[Cookie Interceptor] Capture requested via message');
                window.__captureTikTokCredentials();
            }
        });

        console.log('[Cookie Interceptor] Interceptor initialized');
    })();
    "#.to_string()
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
    let auth_code_guard = login_state.auth_code.lock();
    let code_challenge_guard = login_state.code_challenge.lock();
    
    if let Some(creds) = cookies_guard.as_ref() {
        // Save to cookies.json
        let cookies_path = std::path::Path::new("cookies.json");
        std::fs::write(
            cookies_path,
            serde_json::to_string_pretty(creds).map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;
        
        println!("[Credentials] Saved credentials to cookies.json");
        
        // Build complete credentials object
        let mut credentials = serde_json::Map::new();
        credentials.insert("cookies".to_string(), creds.clone());
        
        if let Some(code) = auth_code_guard.as_ref() {
            credentials.insert("auth_code".to_string(), serde_json::json!(code));
        }
        
        if let Some(challenge) = code_challenge_guard.as_ref() {
            credentials.insert("code_challenge".to_string(), serde_json::json!(challenge));
        }
        
        credentials.insert("saved_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
        
        // Save to credentials.json
        let credentials_path = std::path::Path::new("credentials.json");
        std::fs::write(
            credentials_path,
            serde_json::to_string_pretty(&serde_json::json!(credentials)).map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;
        
        println!("[Credentials] Saved complete credentials to credentials.json");
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Credentials saved to cookies.json and credentials.json",
            "cookies_path": cookies_path.to_string_lossy().as_ref(),
            "credentials_path": credentials_path.to_string_lossy().as_ref(),
            "has_auth_code": auth_code_guard.is_some()
        }))
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "No credentials captured yet"
        }))
    }
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
    if let Ok(saved_cookies) = std::fs::read_to_string("cookies.json") {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&saved_cookies) {
            let has_cookies = parsed.get("data")
                .and_then(|d| d.get("cookies"))
                .map(|c| c.is_object())
                .unwrap_or(false);
            
            if has_cookies {
                println!("[TikTok Login] Found saved TikTok cookies in cookies.json");
                
                // Store in memory for later use
                let mut cookies_guard = login_state.captured_cookies.lock();
                *cookies_guard = Some(parsed.clone());
                
                return Ok(serde_json::json!({
                    "success": true,
                    "state": "tiktok_logged_in",
                    "source": "file"
                }));
            }
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
    if let Ok(saved_tokens) = std::fs::read_to_string("tokens.json") {
        if let Ok(parsed) = serde_json::from_str::<auth::AuthData>(&saved_tokens) {
            if parsed.oauth_token.is_some() {
                println!("[TikTok Login] Found existing Streamlabs token - already authenticated");
                return Ok(serde_json::json!({
                    "success": true,
                    "state": "fully_authenticated"
                }));
            }
        }
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
        let client = reqwest::Client::builder()
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
        let oauth_token = token_json.get("oauth_token")
            .or(token_json.get("access_token"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        if let Some(token) = oauth_token {
            // Save tokens
            std::fs::write(
                "tokens.json",
                serde_json::to_string_pretty(&token_json).map_err(|e| e.to_string())?
            ).map_err(|e| e.to_string())?;
            
            println!("[Auth] Tokens saved to tokens.json");
            
            // Also save credentials.json with all info
            let mut credentials = serde_json::Map::new();
            credentials.insert("oauth_token".to_string(), serde_json::json!(token));
            credentials.insert("tokens".to_string(), token_json.clone());
            credentials.insert("saved_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
            
            std::fs::write(
                "credentials.json",
                serde_json::to_string_pretty(&serde_json::json!(credentials)).map_err(|e| e.to_string())?
            ).map_err(|e| e.to_string())?;
            
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

// Placeholder commands for stream operations (implement in api.rs)
#[tauri::command]
async fn stream_search(query: String) -> Result<serde_json::Value, String> {
    // TODO: Implement actual search using TikTok API
    println!("[Stream] Search called with query: {}", query);
    
    // Return mock data for now
    Ok(serde_json::json!([
        { "full_name": "Gaming", "game_mask_id": "gaming", "id": "1" },
        { "full_name": "Music", "game_mask_id": "music", "id": "2" },
        { "full_name": "Just Chatting", "game_mask_id": "chatting", "id": "3" }
    ]))
}

#[tauri::command]
async fn stream_start(title: String, category: String) -> Result<serde_json::Value, String> {
    // TODO: Implement actual stream start using Streamlabs API
    println!("[Stream] Start called with title: {}, category: {}", title, category);
    
    let stream_key = format!("live_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    
    Ok(serde_json::json!({
        "rtmpUrl": "rtmp://live.tiktok.com/live",
        "streamKey": stream_key,
        "id": uuid::Uuid::new_v4().to_string()
    }))
}

#[tauri::command]
async fn stream_end() -> Result<bool, String> {
    // TODO: Implement actual stream end
    println!("[Stream] End called");
    Ok(true)
}

// ===== TEST COMMANDS =====

/// Run unit tests for the data capture module
#[tauri::command]
async fn run_data_capture_tests() -> Result<serde_json::Value, String> {
    println!("[Test] Running data capture unit tests...");
    
    // Run integration tests
    integration_tests::run_integration_tests();
    
    Ok(serde_json::json!({
        "success": true,
        "message": "Data capture tests completed",
        "test_type": "unit_and_integration"
    }))
}

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

/// Get the JavaScript test script for webview testing
#[tauri::command]
fn get_test_script() -> Result<serde_json::Value, String> {
    let script = data_capture::generate_test_script();
    Ok(serde_json::json!({
        "success": true,
        "script": script,
        "type": "test"
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

fn main() {
    let context = tauri::generate_context!();

    tauri::Builder::default()
        .manage(auth::AuthState::new())
        .manage(LoginState::new())
        .invoke_handler(tauri::generate_handler![
            get_auth_url,
            retrieve_credentials,
            check_credentials,
            check_tiktok_login_state,
            complete_authentication,
            open_tiktok_login_window,
            inject_cookie_interceptor,
            get_captured_credentials,
            close_login_window,
            inject_interceptor_to_login_window,
            save_credentials_to_file,
            stream_search,
            stream_start,
            stream_end,
            // Test commands
            run_data_capture_tests,
            get_capture_script,
            get_test_script,
            validate_captured_data
        ])
        .run(context)
        .expect("Error running Tauri application");
}
