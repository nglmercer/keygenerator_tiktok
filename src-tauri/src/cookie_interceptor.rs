//! Cookie interceptor module for capturing TikTok authentication data
//!
//! This module provides JavaScript injection scripts and utilities for
//! capturing cookies, localStorage, and sessionStorage from the TikTok
//! login window.

/// Returns the JavaScript code for the cookie interceptor.
///
/// This script is injected into the TikTok login window to capture
/// authentication data including cookies, localStorage, and sessionStorage.
/// It monitors for URL changes and login state changes to automatically
/// capture credentials when the user logs in.
///
/// # Returns
/// A string containing the JavaScript code to inject.
pub fn get_cookie_interceptor_script() -> String {
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

            // Try to emit via Tauri event system
            if (window.__TAURI__) {
                window.__TAURI__.event.emit('credentials-captured', JSON.stringify(data));
                console.log('[Cookie Interceptor] Data sent via Tauri event');
            } else {
                // Fallback: try to use the window's event emitter if available
                try {
                    if (window.__tauri__) {
                        window.__tauri__.emit('credentials-captured', JSON.stringify(data));
                        console.log('[Cookie Interceptor] Data sent via __tauri__ event');
                    } else {
                        console.log('[Cookie Interceptor] Tauri API not available, data captured but not sent');
                    }
                } catch (e) {
                    console.log('[Cookie Interceptor] Failed to send data:', e);
                }
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
                } else if (window.__tauri__) {
                    window.__tauri__.emit('navigation', newUrl);
                }
                
                // Capture data on URL change
                captureAllData();
                
                // Check for login or Streamlabs code
                const loginState = checkTikTokLoginState();
                if (loginState === true) {
                    console.log('[Cookie Interceptor] Login detected on URL change!');
                    sendDataToTauri();
                    // DO NOT close window - let Tauri handle redirect to Streamlabs
                } else if (loginState === 'streamlabs_code') {
                    console.log('[Cookie Interceptor] Streamlabs code received!');
                    sendDataToTauri();
                    // Emit navigation event so Rust can also process it
                    if (window.__TAURI__) {
                        window.__TAURI__.event.emit('navigation', newUrl);
                    } else if (window.__tauri__) {
                        window.__tauri__.emit('navigation', newUrl);
                    }
                    // Close window after capturing Streamlabs code
                    setTimeout(() => {
                        if (window.__TAURI__) {
                            window.__TAURI__.window.getCurrent().close();
                        } else {
                            window.close();
                        }
                    }, 1000);
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
                    
                    if (loginState === 'streamlabs_code') {
                        // Emit navigation event so Rust can also process it
                        if (window.__TAURI__) {
                            window.__TAURI__.event.emit('navigation', newUrl);
                        } else if (window.__tauri__) {
                            window.__tauri__.emit('navigation', newUrl);
                        }
                        // Close window after capturing Streamlabs code
                        setTimeout(() => {
                            if (window.__TAURI__) {
                                window.__TAURI__.window.getCurrent().close();
                            } else {
                                window.close();
                            }
                        }, 1000);
                    }
                }
            }
        });

        // Initial capture - check for existing session (already logged in)
        captureAllData();

        // Check for existing saved session
        const hasExistingSession = checkExistingSession();
        if (hasExistingSession) {
            console.log('[Cookie Interceptor] Existing session detected - sending credentials!');
            sendDataToTauri();
            // DO NOT close window - will be redirected to Streamlabs by Tauri handler
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
                    // Emit navigation event for Tauri to handle token exchange
                    if (window.__TAURI__) {
                        window.__TAURI__.event.emit('navigation', url);
                    } else if (window.__tauri__) {
                        window.__tauri__.emit('navigation', url);
                    }
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

/// Returns a JavaScript script to extract cookies from the current page.
///
/// This is a simpler script that can be used to extract cookies from
/// a page that is already logged in.
///
/// # Returns
/// A string containing the JavaScript code to inject.
#[allow(dead_code)]
pub fn get_cookie_extraction_script() -> String {
    r#"
        (function() {
            const cookies = document.cookie.split(';').reduce((acc, cookie) => {
                const [name, value] = cookie.trim().split('=');
                if (name && value) acc[name] = value;
                return acc;
            }, {});
            window.parent.postMessage({ type: 'tiktok-cookies', cookies: cookies }, '*');
        })();
    "#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cookie_interceptor_script() {
        let script = get_cookie_interceptor_script();
        assert!(script.contains("Cookie Interceptor"));
        assert!(script.contains("captureCookies"));
        assert!(script.contains("captureLocalStorage"));
        assert!(script.contains("captureSessionStorage"));
    }

    #[test]
    fn test_get_cookie_extraction_script() {
        let script = get_cookie_extraction_script();
        assert!(script.contains("tiktok-cookies"));
        assert!(script.contains("document.cookie"));
    }
}