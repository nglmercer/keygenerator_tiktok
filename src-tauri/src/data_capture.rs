//! Data Capture Module - Modular and testable TikTok credential capture
//!
//! This module provides an agnostic data capture mechanism that can work
//! on any webpage, independent of the specific website being accessed.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;

/// Represents captured data from a web page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapturedData {
    pub cookies: serde_json::Value,
    pub local_storage: serde_json::Value,
    pub session_storage: serde_json::Value,
    pub headers: serde_json::Value,
    pub url: String,
    pub timestamp: String,
    pub login_detected: bool,
    pub streamlabs_code: Option<String>,
}

impl CapturedData {
    /// Create a new empty CapturedData instance
    pub fn new() -> Self {
        Self {
            cookies: serde_json::json!({}),
            local_storage: serde_json::json!({}),
            session_storage: serde_json::json!({}),
            headers: serde_json::json!({}),
            url: String::new(),
            timestamp: String::new(),
            login_detected: false,
            streamlabs_code: None,
        }
    }

    /// Check if any data was captured
    pub fn has_data(&self) -> bool {
        !self.cookies.as_object().map(|o| o.is_empty()).unwrap_or(true) ||
        !self.local_storage.as_object().map(|o| o.is_empty()).unwrap_or(true) ||
        !self.session_storage.as_object().map(|o| o.is_empty()).unwrap_or(true)
    }

    /// Get total count of captured items
    pub fn total_items(&self) -> usize {
        self.cookies.as_object().map(|o| o.len()).unwrap_or(0) +
        self.local_storage.as_object().map(|o| o.len()).unwrap_or(0) +
        self.session_storage.as_object().map(|o| o.len()).unwrap_or(0)
    }

    /// Check if TikTok-specific data was captured
    pub fn has_tiktok_data(&self) -> bool {
        // Check for common TikTok cookie names
        let tiktok_cookies = ["tt_webid", "tt_webid_v2", "sessionid", "sessionid_ss", "ttwid", "s_v_web_id"];
        let has_tiktok_cookies = if let Some(cookies) = self.cookies.as_object() {
            tiktok_cookies.iter().any(|name| cookies.contains_key(*name))
        } else {
            false
        };

        // Check for TikTok localStorage keys
        let tiktok_ls_keys = ["ttwid", "user", "session", "device_id"];
        let has_tiktok_ls = if let Some(ls) = self.local_storage.as_object() {
            tiktok_ls_keys.iter().any(|key| {
                ls.keys().any(|k| k.to_lowercase().contains(key))
            })
        } else {
            false
        };

        has_tiktok_cookies || has_tiktok_ls
    }
}

/// State for tracking captured data
#[derive(Debug)]
pub struct CaptureState {
    pub captured_data: Arc<Mutex<Option<CapturedData>>>,
    pub capture_complete: Arc<Mutex<bool>>,
    pub capture_errors: Arc<Mutex<Vec<String>>>,
}

impl CaptureState {
    /// Create a new CaptureState
    pub fn new() -> Self {
        Self {
            captured_data: Arc::new(Mutex::new(None)),
            capture_complete: Arc::new(Mutex::new(false)),
            capture_errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Reset the state for a new capture session
    pub fn reset(&self) {
        let mut data = self.captured_data.lock();
        let mut complete = self.capture_complete.lock();
        let mut errors = self.capture_errors.lock();
        
        *data = None;
        *complete = false;
        errors.clear();
    }

    /// Store captured data
    pub fn store_data(&self, data: CapturedData) {
        let mut guard = self.captured_data.lock();
        *guard = Some(data);
    }

    /// Mark capture as complete
    pub fn set_complete(&self) {
        let mut guard = self.capture_complete.lock();
        *guard = true;
    }

    /// Add an error
    pub fn add_error(&self, error: &str) {
        let mut guard = self.capture_errors.lock();
        guard.push(error.to_string());
    }

    /// Get captured data if available
    pub fn get_data(&self) -> Option<CapturedData> {
        let guard = self.captured_data.lock();
        guard.clone()
    }

    /// Check if capture is complete
    pub fn is_complete(&self) -> bool {
        let guard = self.capture_complete.lock();
        *guard
    }

    /// Get all errors
    pub fn get_errors(&self) -> Vec<String> {
        let guard = self.capture_errors.lock();
        guard.clone()
    }
}

/// Configuration for the data capture module
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub capture_cookies: bool,
    pub capture_local_storage: bool,
    pub capture_session_storage: bool,
    pub capture_headers: bool,
    pub capture_interval_ms: u64,
    pub max_capture_duration_ms: u64,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_cookies: true,
            capture_local_storage: true,
            capture_session_storage: true,
            capture_headers: true,
            capture_interval_ms: 1000,
            max_capture_duration_ms: 30000, // 30 seconds default timeout
        }
    }
}

/// Result of a capture operation
#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureResult {
    pub success: bool,
    pub data: Option<CapturedData>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
    pub items_captured: usize,
}

impl CaptureResult {
    /// Create a successful result
    pub fn success(data: CapturedData, duration_ms: u64) -> Self {
        let items = data.total_items();
        Self {
            success: true,
            data: Some(data),
            errors: Vec::new(),
            duration_ms,
            items_captured: items,
        }
    }

    /// Create a failed result
    pub fn failed(errors: Vec<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            errors,
            duration_ms,
            items_captured: 0,
        }
    }

    /// Create a timeout result
    pub fn timeout(duration_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            errors: vec!["Capture timeout".to_string()],
            duration_ms,
            items_captured: 0,
        }
    }
}

/// Generate JavaScript code for data capture
/// This JavaScript is designed to be injected into any webview and capture
/// data regardless of the website's security settings
pub fn generate_capture_script() -> String {
    r#"
    (function() {
        console.log('[DataCapture] Starting capture...');
        
        const captureData = {
            cookies: {},
            localStorage: {},
            sessionStorage: {},
            headers: {},
            url: window.location.href,
            timestamp: new Date().toISOString(),
            login_detected: false,
            streamlabs_code: null
        };

        // Capture cookies
        function captureCookies() {
            try {
                const cookies = document.cookie.split(';');
                const cookieObj = {};
                cookies.forEach(cookie => {
                    const trimmed = cookie.trim();
                    if (trimmed) {
                        const eqIndex = trimmed.indexOf('=');
                        const name = eqIndex > 0 ? trimmed.substring(0, eqIndex) : trimmed;
                        const value = eqIndex > 0 ? trimmed.substring(eqIndex + 1) : '';
                        cookieObj[name] = value;
                    }
                });
                captureData.cookies = cookieObj;
                console.log('[DataCapture] Cookies captured:', Object.keys(cookieObj).length);
                return cookieObj;
            } catch (e) {
                console.log('[DataCapture] Cookie capture failed:', e);
                return {};
            }
        }

        // Capture localStorage
        function captureLocalStorage() {
            try {
                const ls = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    ls[key] = localStorage.getItem(key);
                }
                captureData.localStorage = ls;
                console.log('[DataCapture] localStorage captured:', Object.keys(ls).length);
                return ls;
            } catch (e) {
                console.log('[DataCapture] localStorage capture failed:', e);
                // Cross-origin access might be blocked
                return {};
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
                captureData.sessionStorage = ss;
                console.log('[DataCapture] sessionStorage captured:', Object.keys(ss).length);
                return ss;
            } catch (e) {
                console.log('[DataCapture] sessionStorage capture failed:', e);
                return {};
            }
        }

        // Capture all data
        function captureAll() {
            captureCookies();
            captureLocalStorage();
            captureSessionStorage();
            
            // Check for TikTok login indicators
            const tiktokCookies = ['tt_webid', 'tt_webid_v2', 'sessionid', 'sessionid_ss', 'ttwid', 's_v_web_id'];
            const hasTiktokCookie = tiktokCookies.some(name => captureData.cookies[name]);
            
            const tiktokLsKeys = ['ttwid', 'user', 'session', 'device_id'];
            const hasTiktokLs = tiktokLsKeys.some(key => {
                return Object.keys(captureData.localStorage).some(k => 
                    k.toLowerCase().includes(key)
                );
            });
            
            captureData.login_detected = hasTiktokCookie || hasTiktokLs;
            
            // Check for Streamlabs code in URL
            if (window.location.href.includes('streamlabs.com') && window.location.href.includes('code=')) {
                const urlParams = new URLSearchParams(window.location.href.split('?')[1]);
                const code = urlParams.get('code');
                if (code) {
                    captureData.streamlabs_code = code;
                }
            }
            
            return captureData;
        }

        // Expose capture function
        window.__capture_data = captureAll;

        // Initial capture
        const data = captureAll();
        
        console.log('[DataCapture] Initial capture complete:', {
            cookies: Object.keys(data.cookies).length,
            localStorage: Object.keys(data.localStorage).length,
            sessionStorage: Object.keys(data.sessionStorage).length,
            login_detected: data.login_detected
        });

        // Return the captured data for testing
        return data;
    })();
    "#.to_string()
}
