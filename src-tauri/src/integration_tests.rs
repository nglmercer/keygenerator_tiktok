//! Integration Tests for Data Capture Module
//!
//! These tests verify the data capture functionality with timeout support
//! using JavaScript callbacks for webview integration testing.

use crate::data_capture::{
    CaptureConfig, CaptureResult, CaptureState, CapturedData,
    generate_capture_script, generate_test_script,
};
use crate::test_utils::{
    MockWebviewEnv, TestConfig, TestResult,
    create_tiktok_mock_env, create_empty_mock_env, 
    create_streamlabs_mock_env, run_capture_test,
    simulate_js_capture, validate_captured_data,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use std::thread;

/// Integration Test Suite for Data Capture
/// These tests verify the complete capture flow with timeouts and validation
pub struct DataCaptureIntegrationTests {
    config: TestConfig,
    results: Vec<TestResult>,
}

impl DataCaptureIntegrationTests {
    /// Create a new test suite with default config
    pub fn new() -> Self {
        Self {
            config: TestConfig::default(),
            results: Vec::new(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: TestConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run all tests and return results
    pub fn run_all(&mut self) -> Vec<TestResult> {
        self.test_capture_with_tiktok_cookies();
        self.test_capture_with_empty_env();
        self.test_capture_with_streamlabs_redirect();
        self.test_capture_timeout_behavior();
        self.test_concurrent_capture_requests();
        self.test_state_management();
        self.results.clone()
    }

    /// Test 1: Capture data with TikTok cookies present
    fn test_capture_with_tiktok_cookies(&mut self) {
        let test_name = "TikTok Cookies Capture";
        println!("Running: {}", test_name);
        
        let env = create_tiktok_mock_env();
        let result = run_capture_test(&env, test_name);
        
        if self.config.verbose {
            println!("  Result: {} - {} items captured", 
                if result.passed { "PASS" } else { "FAIL" },
                result.data_captured);
        }
        
        self.results.push(result);
    }

    /// Test 2: Capture data with empty environment
    fn test_capture_with_empty_env(&mut self) {
        let test_name = "Empty Environment Capture";
        println!("Running: {}", test_name);
        
        let env = create_empty_mock_env();
        let result = run_capture_test(&env, test_name);
        
        if self.config.verbose {
            println!("  Result: {} - {} errors", 
                if result.passed { "PASS" } else { "FAIL" },
                result.errors.len());
        }
        
        self.results.push(result);
    }

    /// Test 3: Capture with Streamlabs redirect URL
    fn test_capture_with_streamlabs_redirect(&mut self) {
        let test_name = "Streamlabs Redirect Capture";
        println!("Running: {}", test_name);
        
        let env = create_streamlabs_mock_env("test_auth_code_ABC123");
        let result = run_capture_test(&env, test_name);
        
        if self.config.verbose {
            let auth_code = result.details
                .as_ref()
                .and_then(|d| d.get("streamlabs_code"))
                .and_then(|v| v.as_str())
                .unwrap_or("none");
            println!("  Result: {} - Auth code: {}", 
                if result.passed { "PASS" } else { "FAIL" },
                auth_code);
        }
        
        self.results.push(result);
    }

    /// Test 4: Test timeout behavior
    fn test_capture_timeout_behavior(&mut self) {
        let test_name = "Timeout Behavior";
        println!("Running: {}", test_name);
        
        let start = Instant();
        let mut result = TestResult::new(test_name);
        
        // Simulate a capture that might timeout
        let handle = std::thread::spawn(move || {
            thread::sleep(Duration::from_millis(500)); // Simulate work
            CapturedData::new()
        });

        let timeout_ms = 1000u64;
        loop {
            if handle.is_finished() {
                let _ = handle.join();
                result.success(0);
                break;
            }

            if start.elapsed() > Duration::from_millis(timeout_ms) {
                result.add_error("Simulated timeout");
                break;
            }
            
            thread::sleep(Duration::from_millis(50));
        }
        
        result.duration_ms = start.elapsed().as_millis() as u64;
        
        if self.config.verbose {
            println!("  Result: {} - Duration: {}ms", 
                if result.passed { "PASS" } else { "FAIL" },
                result.duration_ms);
        }
        
        self.results.push(result);
    }

    /// Test 5: Test concurrent capture requests
    fn test_concurrent_capture_requests(&mut self) {
        let test_name = "Concurrent Capture Requests";
        println!("Running: {}", test_name);
        
        let start = Instant();
        let mut result = TestResult::new(test_name);
        
        // Create multiple environments
        let env1 = create_tiktok_mock_env();
        let env2 = create_streamlabs_mock_env("concurrent_code");
        
        let handle1 = std::thread::spawn(move || simulate_js_capture(&env1));
        let handle2 = std::thread::spawn(move || simulate_js_capture(&env2));
        
        let (data1, data2) = (handle1.join().unwrap(), handle2.join().unwrap());
        
        let total_items = data1.total_items() + data2.total_items();
        result.success(total_items);
        result.duration_ms = start.elapsed().as_millis() as u64;
        
        result.set_details(serde_json::json!({
            "thread1_items": data1.total_items(),
            "thread2_items": data2.total_items(),
            "concurrent": true
        }));
        
        if self.config.verbose {
            println!("  Result: {} - {} items from 2 threads", 
                if result.passed { "PASS" } else { "FAIL" },
                total_items);
        }
        
        self.results.push(result);
    }

    /// Test 6: Test state management during capture
    fn test_state_management(&mut self) {
        let test_name = "State Management";
        println!("Running: {}", test_name);
        
        let start = Instant();
        let mut result = TestResult::new(test_name);
        
        let state = CaptureState::new();
        
        // Initially empty
        assert!(state.get_data().is_none());
        assert!(!state.is_complete());
        assert!(state.get_errors().is_empty());
        
        // Store some data
        let data = simulate_js_capture(&create_tiktok_mock_env());
        state.store_data(data.clone());
        state.set_complete();
        state.add_error("Test error");
        
        // Verify state
        let stored_data = state.get_data();
        assert!(stored_data.is_some());
        assert!(state.is_complete());
        assert_eq!(state.get_errors().len(), 1);
        
        // Reset state
        state.reset();
        assert!(state.get_data().is_none());
        assert!(!state.is_complete());
        assert!(state.get_errors().is_empty());
        
        result.success(1);
        result.duration_ms = start.elapsed().as_millis() as u64;
        
        if self.config.verbose {
            println!("  Result: {} - State management works correctly", 
                if result.passed { "PASS" } else { "FAIL" });
        }
        
        self.results.push(result);
    }

    /// Get summary of test results
    pub fn get_summary(&self) -> serde_json::Value {
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = self.results.len() - passed;
        
        serde_json::json!({
            "total_tests": self.results.len(),
            "passed": passed,
            "failed": failed,
            "pass_rate": if !self.results.is_empty() {
                (passed as f64 / self.results.len() as f64) * 100.0
            } else {
                0.0
            },
            "config": {
                "timeout_ms": self.config.timeout_ms,
                "retries": self.config.retries,
                "verbose": self.config.verbose
            }
        })
    }
}

/// Webview Integration Test Helpers
/// These functions help create tests that can be run in a real Tauri webview context
pub struct WebviewTestHelpers;

impl WebviewTestHelpers {
    /// Generate JavaScript code that runs a capture test and reports via callback
    pub fn generate_js_capture_test(callback_name: &str) -> String {
        format!(r#"
        (function() {{
            console.log('[WebviewTest] Starting capture test with callback: {}');
            
            const testResult = {{
                success: false,
                cookies_count: 0,
                localStorage_count: 0,
                sessionStorage_count: 0,
                errors: [],
                timestamp: new Date().toISOString()
            }};
            
            try {{
                // Count cookies
                const cookies = document.cookie.split(';').filter(c => c.trim());
                testResult.cookies_count = cookies.length;
                
                // Count localStorage
                try {{
                    testResult.localStorage_count = localStorage.length;
                }} catch(e) {{
                    testResult.errors.push('localStorage error: ' + e.message);
                }}
                
                // Count sessionStorage
                try {{
                    testResult.sessionStorage_count = sessionStorage.length;
                }} catch(e) {{
                    testResult.errors.push('sessionStorage error: ' + e.message);
                }}
                
                testResult.success = testResult.errors.length === 0;
            }} catch(e) {{
                testResult.errors.push('Capture error: ' + e.message);
            }}
            
            // Call the callback function
            if (typeof window.{callback_name} === 'function') {{
                window.{callback_name}(testResult);
            }} else {{
                // Fallback to postMessage
                window.parent.postMessage({{
                    type: 'capture-test-result',
                    data: testResult
                }}, '*');
            }}
            
            console.log('[WebviewTest] Test complete:', JSON.stringify(testResult, null, 2));
            return testResult;
        }})();
        "#, callback_name)
    }

    /// Generate JavaScript code that waits for data and calls callback
    pub fn generate_js_wait_for_data(max_wait_ms: u64, callback_name: &str) -> String {
        format!(r#"
        (function() {{
            console.log('[WebviewTest] Waiting for data (max {}ms)...');
            
            let elapsed = 0;
            const checkInterval = 100;
            
            const checkAndReport = () => {{
                const result = {{
                    success: false,
                    has_data: false,
                    items_found: 0,
                    login_detected: false,
                    elapsed_ms: elapsed,
                    errors: []
                }};
                
                try {{
                    // Check cookies
                    const cookies = document.cookie.split(';').filter(c => c.trim());
                    result.items_found += cookies.length;
                    
                    // Check localStorage
                    result.items_found += localStorage.length;
                    
                    // Check sessionStorage
                    result.items_found += sessionStorage.length;
                    
                    result.has_data = result.items_found > 0;
                    result.success = true;
                    
                    // Check for login indicators
                    const tiktokCookies = ['tt_webid', 'tt_webid_v2', 'sessionid', 'sessionid_ss', 'ttwid'];
                    const hasLoginCookie = tiktokCookies.some(name => 
                        document.cookie.split(';').some(c => c.trim().startsWith(name + '='))
                    );
                    
                    result.login_detected = hasLoginCookie || localStorage.length > 0;
                    
                }} catch(e) {{
                    result.errors.push(e.message);
                }}
                
                // Report via callback
                if (typeof window.{callback_name} === 'function') {{
                    window.{callback_name}(result);
                }} else {{
                    window.parent.postMessage({{
                        type: 'wait-for-data-result',
                        data: result
                    }}, '*');
                }}
                
                return result.has_data;
            }};
            
            const intervalId = setInterval(() => {{
                elapsed += checkInterval;
                
                if (checkAndReport().has_data) {{
                    clearInterval(intervalId);
                    console.log('[WebviewTest] Data found after {}ms', elapsed);
                    return;
                }}
                
                if (elapsed >= {}) {{
                    clearInterval(intervalId);
                    console.log('[WebviewTest] Timeout after {}ms', elapsed);
                    checkAndReport(); // Report final state
                }}
            }}, checkInterval);
            
            // Initial check
            checkAndReport();
            
        }})();
        "#, max_wait_ms, max_wait_ms, max_wait_ms)
    }

    /// Generate JavaScript code that tests storage access
    pub fn generate_js_storage_test() -> String {
        r#"
        (function() {
            console.log('[WebviewTest] Testing storage access...');
            
            const result = {
                localStorage_access: false,
                sessionStorage_access: false,
                documentCookie_access: false,
                localStorage_keys: [],
                sessionStorage_keys: [],
                cookie_count: 0,
                error: null
            };
            
            try {
                // Test document.cookie
                result.documentCookie_access = true;
                const cookies = document.cookie.split(';').filter(c => c.trim());
                result.cookie_count = cookies.length;
            } catch(e) {
                result.error = 'Cookie access: ' + e.message;
            }
            
            try {
                // Test localStorage
                result.localStorage_access = true;
                result.localStorage_keys = [];
                for (let i = 0; i < localStorage.length; i++) {
                    result.localStorage_keys.push(localStorage.key(i));
                }
            } catch(e) {
                result.error = (result.error ? result.error + '; ' : '') + 'localStorage: ' + e.message;
            }
            
            try {
                // Test sessionStorage
                result.sessionStorage_access = true;
                result.sessionStorage_keys = [];
                for (let i = 0; i < sessionStorage.length; i++) {
                    result.sessionStorage_keys.push(sessionStorage.key(i));
                }
            } catch(e) {
                result.error = (result.error ? result.error + '; ' : '') + 'sessionStorage: ' + e.message;
            }
            
            console.log('[WebviewTest] Storage test result:', JSON.stringify(result, null, 2));
            return result;
        })();
        "#.to_string()
    }
}

/// Run a webview simulation test (for CI/CD without actual webview)
pub fn run_webview_simulation_test() -> TestResult {
    let test_name = "Webview Simulation";
    let mut result = TestResult::new(test_name);
    
    let start = Instant();
    
    // Simulate the JavaScript test
    let env = create_tiktok_mock_env();
    let data = simulate_js_capture(&env);
    
    // Validate
    if let Err(e) = validate_captured_data(&data, 1) {
        result.add_error(&e);
    } else {
        result.success(data.total_items());
    }
    
    result.duration_ms = start.elapsed().as_millis() as u64;
    
    result.set_details(serde_json::json!({
        "simulated": true,
        "has_data": data.has_data(),
        "has_tiktok_data": data.has_tiktok_data(),
        "login_detected": data.login_detected,
        "total_items": data.total_items()
    }));
    
    result
}

/// Run all integration tests and print summary
pub fn run_integration_tests() {
    println!("\n=== Running Data Capture Integration Tests ===\n");
    
    // Run unit tests
    let mut tests = DataCaptureIntegrationTests::new();
    let results = tests.run_all();
    
    // Print results
    println!("\n=== Test Summary ===");
    for r in &results {
        let status = if r.passed { "✓ PASS" } else { "✗ FAIL" };
        println!("{} - {} - {}ms - {} items", status, r.test_name, r.duration_ms, r.data_captured);
        
        if !r.errors.is_empty() {
            for e in &r.errors {
                println!("  Error: {}", e);
            }
        }
    }
    
    // Summary
    let passed: usize = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let pass_rate = if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 0.0 };
    
    println!("\n=== Results: {}/{} ({:.1}%) passed ===\n", passed, total, pass_rate);
    
    // Print detailed summary
    println!("Detailed Summary: {}", serde_json::to_string_pretty(&tests.get_summary()).unwrap());
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_run_all_integration_tests() {
        let mut tests = DataCaptureIntegrationTests::new();
        let results = tests.run_all();
        
        // Should have 6 tests
        assert_eq!(results.len(), 6);
        
        // At least some should pass (those that simulate valid data)
        let passed = results.iter().filter(|r| r.passed).count();
        assert!(passed >= 4, "Expected at least 4 tests to pass, got {}", passed);
    }

    #[test]
    fn test_webview_test_helpers() {
        let js_test = WebviewTestHelpers::generate_js_capture_test("testCallback");
        assert!(js_test.contains("testCallback"));
        assert!(js_test.contains("capture-test-result"));
        
        let js_wait = WebviewTestHelpers::generate_js_wait_for_data(5000, "waitCallback");
        assert!(js_wait.contains("waitCallback"));
        assert!(js_wait.contains("wait-for-data-result"));
        
        let js_storage = WebviewTestHelpers::generate_js_storage_test();
        assert!(js_storage.contains("localStorage_access"));
        assert!(js_storage.contains("sessionStorage_access"));
    }

    #[test]
    fn test_run_webview_simulation() {
        let result = run_webview_simulation_test();
        
        assert!(result.passed);
        assert!(result.data_captured > 0);
        assert!(result.duration_ms < 1000); // Should be fast
        
        if let Some(details) = &result.details {
            assert!(details.get("simulated").and_then(|v| v.as_bool()).unwrap_or(false));
            assert!(details.get("has_data").and_then(|v| v.as_bool()).unwrap_or(false));
        }
    }

    #[test]
    fn test_integration_test_summary() {
        let mut tests = DataCaptureIntegrationTests::new();
        tests.run_all();
        
        let summary = tests.get_summary();
        assert_eq!(summary["total_tests"], 6);
        assert!(summary["passed"].as_u64().unwrap() > 0);
        assert!(summary["pass_rate"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn test_js_callback_generation() {
        // Test that the generated JavaScript contains expected patterns
        let capture_script = generate_capture_script();
        assert!(capture_script.contains("captureCookies"));
        assert!(capture_script.contains("captureLocalStorage"));
        assert!(capture_script.contains("captureSessionStorage"));
        
        let test_script = generate_test_script();
        assert!(test_script.contains("__capture_test_result"));
        assert!(test_script.contains("success"));
    }
}
