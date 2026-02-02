//! Test Utilities and Mocks for Data Capture Module
//!
//! This module provides utilities for testing the data capture functionality
//! including mock webview environments, test helpers, and integration test support.

use crate::data_capture::{CaptureState, CapturedData, CaptureConfig, CaptureResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use std::thread;

/// Mock Webview Environment for Testing
/// This simulates a webview environment for unit tests without needing a real Tauri webview
#[derive(Debug, Clone)]
pub struct MockWebviewEnv {
    pub cookies: Arc<Mutex<Vec<(String, String)>>>,
    pub local_storage: Arc<Mutex<Vec<(String, String)>>>,
    pub session_storage: Arc<Mutex<Vec<(String, String)>>>,
    pub url: Arc<Mutex<String>>,
    pub is_tiktok_domain: bool,
}

impl Default for MockWebviewEnv {
    fn default() -> Self {
        Self::new(false)
    }
}

impl MockWebviewEnv {
    /// Create a new mock environment
    pub fn new(is_tiktok_domain: bool) -> Self {
        Self {
            cookies: Arc::new(Mutex::new(Vec::new())),
            local_storage: Arc::new(Mutex::new(Vec::new())),
            session_storage: Arc::new(Mutex::new(Vec::new())),
            url: Arc::new(Mutex::new(String::new())),
            is_tiktok_domain,
        }
    }

    /// Add a cookie
    pub fn add_cookie(&self, name: &str, value: &str) {
        let mut guard = self.cookies.lock();
        guard.push((name.to_string(), value.to_string()));
    }

    /// Add a localStorage item
    pub fn add_local_storage(&self, key: &str, value: &str) {
        let mut guard = self.local_storage.lock();
        guard.push((key.to_string(), value.to_string()));
    }

    /// Add a sessionStorage item
    pub fn add_session_storage(&self, key: &str, value: &str) {
        let mut guard = self.session_storage.lock();
        guard.push((key.to_string(), value.to_string()));
    }

    /// Set the current URL
    pub fn set_url(&self, url: &str) {
        let mut guard = self.url.lock();
        *guard = url.to_string();
    }

    /// Simulate cookie access (like JavaScript document.cookie)
    pub fn get_cookies_string(&self) -> String {
        let guard = self.cookies.lock();
        guard.iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Simulate localStorage access
    pub fn get_local_storage(&self) -> Vec<(String, String)> {
        let guard = self.local_storage.lock();
        guard.clone()
    }

    /// Simulate sessionStorage access
    pub fn get_session_storage(&self) -> Vec<(String, String)> {
        let guard = self.session_storage.lock();
        guard.clone()
    }
}

/// Test Result for integration tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub data_captured: usize,
    pub errors: Vec<String>,
    pub details: Option<serde_json::Value>,
}

impl TestResult {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            passed: false,
            duration_ms: 0,
            data_captured: 0,
            errors: Vec::new(),
            details: None,
        }
    }

    pub fn success(&mut self, data_captured: usize) {
        self.passed = true;
        self.data_captured = data_captured;
    }

    pub fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }

    pub fn set_details(&mut self, details: serde_json::Value) {
        self.details = Some(details);
    }
}

/// Test Configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout_ms: u64,
    pub retries: u32,
    pub verbose: bool,
    pub capture_on_tiktok: bool,
    pub capture_on_streamlabs: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30000,
            retries: 3,
            verbose: true,
            capture_on_tiktok: true,
            capture_on_streamlabs: true,
        }
    }
}

/// Helper to run tests with timeout
pub fn run_test_with_timeout<F, T>(test_name: &str, timeout_ms: u64, test_fn: F) -> TestResult
where
    F: FnOnce() -> T,
    T: Into<serde_json::Value>,
{
    let start = Instant::now();
    let mut result = TestResult::new(test_name);
    
    let handle = std::thread::spawn(move || {
        test_fn().into()
    });

    loop {
        if handle.is_finished() {
            match handle.join() {
                Ok(test_output) => {
                    result.passed = true;
                    result.duration_ms = start.elapsed().as_millis() as u64;
                    result.set_details(test_output);
                }
                Err(_) => {
                    result.add_error("Test panicked");
                    result.duration_ms = start.elapsed().as_millis() as u64;
                }
            }
            return result;
        }

        if start.elapsed() > Duration::from_millis(timeout_ms) {
            result.add_error(&format!("Test timeout after {}ms", timeout_ms));
            result.duration_ms = timeout_ms;
            return result;
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Simulate JavaScript capture execution in mock environment
pub fn simulate_js_capture(env: &MockWebviewEnv) -> CapturedData {
    let mut data = CapturedData::new();
    
    // Capture cookies
    let cookies_str = env.get_cookies_string();
    if !cookies_str.is_empty() {
        let cookie_obj: serde_json::Map<String, serde_json::Value> = cookies_str
            .split(";")
            .filter(|c| !c.trim().is_empty())
            .filter_map(|cookie| {
                let parts: Vec<&str> = cookie.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((
                        parts[0].trim().to_string(),
                        serde_json::json!(parts[1].trim()),
                    ))
                } else {
                    None
                }
            })
            .collect();
        data.cookies = serde_json::json!(cookie_obj);
    }

    // Capture localStorage
    let ls_items = env.get_local_storage();
    if !ls_items.is_empty() {
        let ls_obj: serde_json::Map<String, serde_json::Value> = ls_items
            .into_iter()
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect();
        data.local_storage = serde_json::json!(ls_obj);
    }

    // Capture sessionStorage
    let ss_items = env.get_session_storage();
    if !ss_items.is_empty() {
        let ss_obj: serde_json::Map<String, serde_json::Value> = ss_items
            .into_iter()
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect();
        data.session_storage = serde_json::json!(ss_obj);
    }

    // Set URL
    let url_guard = env.url.lock();
    data.url = url_guard.clone();

    // Check for TikTok login indicators
    let tiktok_cookies = ["tt_webid", "tt_webid_v2", "sessionid", "sessionid_ss", "ttwid"];
    let has_tiktok_cookie = if let Some(cookies) = data.cookies.as_object() {
        tiktok_cookies.iter().any(|name| cookies.contains_key(*name))
    } else {
        false
    };

    let tiktok_ls_keys = ["ttwid", "user", "session", "device_id"];
    let has_tiktok_ls = if let Some(ls) = data.local_storage.as_object() {
        tiktok_ls_keys.iter().any(|key| {
            ls.keys().any(|k| k.to_lowercase().contains(key))
        })
    } else {
        false
    };

    data.login_detected = has_tiktok_cookie || has_tiktok_ls;

    // Check for Streamlabs code
    if data.url.contains("streamlabs.com") && data.url.contains("code=") {
        if let Some(code_start) = data.url.find("code=") {
            let code_section = &data.url[code_start + 5..];
            let code_end = code_section.find('&').unwrap_or(code_section.len());
            data.streamlabs_code = Some(code_section[..code_end].to_string());
        }
    }

    data.timestamp = chrono::Utc::now().to_rfc3339();

    data
}

/// Create a mock TikTok environment with typical TikTok cookies/storage
pub fn create_tiktok_mock_env() -> MockWebviewEnv {
    let env = MockWebviewEnv::new(true);
    
    // Add typical TikTok cookies
    env.add_cookie("tt_webid", "7201234567890123456");
    env.add_cookie("tt_webid_v2", "7201234567890123456");
    env.add_cookie("ttwid", "abc123-def456");
    env.add_cookie("s_v_web_id", "verify_abc123");
    env.add_cookie("sessionid", "xyz789");
    env.add_cookie("sessionid_ss", "xyz789_ss");
    
    // Add TikTok localStorage items
    env.add_local_storage("ttwid", "abc123-def456");
    env.add_local_storage("device_id", "device_123456");
    env.add_local_storage("user_info", "{\"userId\":\"12345\"}");
    env.add_local_storage("session_data", "{\"token\":\"test_token\"}");
    
    env.set_url("https://www.tiktok.com/foryou");
    
    env
}

/// Create a mock environment with no data (fresh browser)
pub fn create_empty_mock_env() -> MockWebviewEnv {
    let env = MockWebviewEnv::new(false);
    env.set_url("https://www.tiktok.com/login");
    env
}

/// Create a mock environment simulating Streamlabs redirect
pub fn create_streamlabs_mock_env(auth_code: &str) -> MockWebviewEnv {
    let env = MockWebviewEnv::new(false);
    env.set_url(&format!(
        "https://streamlabs.com/dashboard?success=true&code={}",
        auth_code
    ));
    env
}

/// Run a capture test with the given environment
pub fn run_capture_test(env: &MockWebviewEnv, test_name: &str) -> TestResult {
    let start = Instant::now();
    let mut result = TestResult::new(test_name);
    
    // Simulate capture
    let data = simulate_js_capture(env);
    
    // Validate results
    if data.has_data() {
        result.success(data.total_items());
        
        // Check if we captured expected data
        let details = serde_json::json!({
            "cookies_count": data.cookies.as_object().map(|o| o.len()).unwrap_or(0),
            "localStorage_count": data.local_storage.as_object().map(|o| o.len()).unwrap_or(0),
            "sessionStorage_count": data.session_storage.as_object().map(|o| o.len()).unwrap_or(0),
            "login_detected": data.login_detected,
            "has_tiktok_data": data.has_tiktok_data(),
            "url": data.url
        });
        result.set_details(details);
    } else {
        result.add_error("No data captured");
    }
    
    result.duration_ms = start.elapsed().as_millis() as u64;
    result
}

/// Test capture with timeout
pub fn run_capture_test_with_timeout(
    env: &MockWebviewEnv,
    test_name: &str,
    timeout_ms: u64,
) -> TestResult {
    let start = Instant();
    let mut result = TestResult::new(test_name);
    
    let handle = std::thread::spawn(move || {
        simulate_js_capture(env)
    });

    loop {
        if handle.is_finished() {
            match handle.join() {
                Ok(data) => {
                    if data.has_data() {
                        result.success(data.total_items());
                        let details = serde_json::json!({
                            "total_items": data.total_items(),
                            "login_detected": data.login_detected,
                            "has_tiktok_data": data.has_tiktok_data()
                        });
                        result.set_details(details);
                    } else {
                        result.add_error("No data captured");
                    }
                }
                Err(_) => {
                    result.add_error("Capture thread panicked");
                }
            }
            result.duration_ms = start.elapsed().as_millis() as u64;
            return result;
        }

        if start.elapsed() > Duration::from_millis(timeout_ms) {
            result.add_error(&format!("Capture timeout after {}ms", timeout_ms));
            result.duration_ms = timeout_ms;
            return result;
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Validate that captured data meets minimum requirements
pub fn validate_captured_data(data: &CapturedData, min_items: usize) -> Result<(), String> {
    let total = data.total_items();
    if total < min_items {
        return Err(format!(
            "Insufficient data captured: {} items (minimum: {})",
            total, min_items
        ));
    }
    
    if data.cookies.is_object() && data.cookies.as_object().unwrap().is_empty() {
        return Err("No cookies captured".to_string());
    }
    
    Ok(())
}

/// Print test results in a formatted way
pub fn print_test_results(results: &[TestResult]) {
    println!("\n=== Test Results ===");
    for result in results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!(
            "[{}] {} - {}ms - {} items captured",
            status,
            result.test_name,
            result.duration_ms,
            result.data_captured
        );
        
        if !result.errors.is_empty() {
            println!("  Errors:");
            for error in &result.errors {
                println!("    - {}", error);
            }
        }
        
        if let Some(ref details) = result.details {
            println!("  Details: {}", serde_json::to_string_pretty(details).unwrap());
        }
    }
    
    let passed: usize = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    println!("\n=== Summary: {}/{} tests passed ===\n", passed, total);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_webview_env_new() {
        let env = MockWebviewEnv::new(true);
        assert!(env.get_cookies_string().is_empty());
        assert_eq!(env.get_local_storage().len(), 0);
        assert!(env.is_tiktok_domain);
    }

    #[test]
    fn test_mock_webview_env_add_items() {
        let env = MockWebviewEnv::default();
        env.add_cookie("test", "value");
        env.add_local_storage("key", "val");
        env.add_session_storage("ss_key", "ss_val");
        env.set_url("https://example.com");
        
        assert!(env.get_cookies_string().contains("test=value"));
        assert_eq!(env.get_local_storage().len(), 1);
        assert_eq!(env.get_session_storage().len(), 1);
        
        let url_guard = env.url.lock();
        assert_eq!(*url_guard, "https://example.com");
    }

    #[test]
    fn test_simulate_js_capture_empty() {
        let env = MockWebviewEnv::default();
        let data = simulate_js_capture(&env);
        
        assert!(!data.has_data());
        assert_eq!(data.total_items(), 0);
    }

    #[test]
    fn test_simulate_js_capture_with_data() {
        let env = create_tiktok_mock_env();
        let data = simulate_js_capture(&env);
        
        assert!(data.has_data());
        assert!(data.has_tiktok_data());
        assert!(data.login_detected);
        assert!(data.total_items() > 0);
    }

    #[test]
    fn test_simulate_js_capture_streamlabs() {
        let env = create_streamlabs_mock_env("test_auth_code_123");
        let data = simulate_js_capture(&env);
        
        assert!(data.url.contains("streamlabs.com"));
        assert!(data.streamlabs_code.is_some());
        assert_eq!(data.streamlabs_code, Some("test_auth_code_123".to_string()));
    }

    #[test]
    fn test_run_capture_test() {
        let env = create_tiktok_mock_env();
        let result = run_capture_test(&env, "TikTok Data Capture");
        
        assert!(result.passed);
        assert!(result.data_captured > 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_run_capture_test_empty() {
        let env = create_empty_mock_env();
        let result = run_capture_test(&env, "Empty Environment");
        
        assert!(!result.passed);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_captured_data_success() {
        let env = create_tiktok_mock_env();
        let data = simulate_js_capture(&env);
        
        let result = validate_captured_data(&data, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_captured_data_failure() {
        let env = create_empty_mock_env();
        let data = simulate_js_capture(&env);
        
        let result = validate_captured_data(&data, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_test_result() {
        let mut result = TestResult::new("Test 1");
        assert!(!result.passed);
        
        result.success(5);
        assert!(result.passed);
        assert_eq!(result.data_captured, 5);
        
        result.add_error("Error 1");
        assert_eq!(result.errors.len(), 1);
        
        result.set_details(serde_json::json!({"key": "value"}));
        assert!(result.details.is_some());
    }

    #[test]
    fn test_test_config_defaults() {
        let config = TestConfig::default();
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.retries, 3);
        assert!(config.verbose);
    }
}
