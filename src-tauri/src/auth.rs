#![allow(dead_code)]
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::fs;
use std::net::TcpListener;
use std::io::{Read, Write};
use parking_lot::Mutex;
use tokio::sync::mpsc;

const CLIENT_KEY: &str = "awdjaq9ide8frtz";
const REDIRECT_URI: &str = "https://streamlabs.com/tiktok/auth";
const AUTH_DATA_URL: &str = "https://streamlabs.com/api/v5/slobs/auth/data";

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthData {
    pub success: bool,
    pub data: Option<TokenData>,
    pub oauth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenData {
    pub oauth_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Clone)]
pub struct AuthManager {
    pub code_verifier: String,
    pub code_challenge: String,
    pub cookies_path: String,
    pub tokens_path: String,
    pub port: u16,
}

impl AuthManager {
    pub fn new() -> Self {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        let cookies_path = "cookies.json".to_string();
        let tokens_path = "tokens.json".to_string();
        let port = 1421; // Callback port

        Self {
            code_verifier,
            code_challenge,
            cookies_path,
            tokens_path,
            port,
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

    pub fn get_auth_url(&self) -> String {
        format!(
            "https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge={}",
            self.code_challenge
        )
    }

    // Static async function for use in Tauri commands without holding lock
    pub async fn async_retrieve_credentials(
        code_verifier: String,
        code_challenge: String,
        tokens_path: String,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Try to load existing token first
        if let Ok(saved) = fs::read_to_string(&tokens_path) {
            if let Ok(parsed) = serde_json::from_str::<AuthData>(&saved) {
                if let Some(token) = parsed.oauth_token {
                    println!("[AuthManager] Using saved token from tokens.json");
                    return Ok(serde_json::json!({
                        "success": true,
                        "oauth_token": token,
                        "source": "cache"
                    }));
                }
            }
        }

        let auth_url = format!(
            "https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge={}",
            code_challenge
        );
        println!("[AuthManager] Auth URL: {}", auth_url);

        // Use reqwest to get cookies first (simulating browser behavior)
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        // Navigate to auth URL to get initial cookies
        println!("[AuthManager] Fetching initial page to get cookies...");
        let _response = client.get(&auth_url).send().await?;

        // Wait for callback with OAuth code
        println!("[AuthManager] Waiting for OAuth callback...");
        let code = Self::wait_for_callback().await?;

        // Exchange code for token
        println!("[AuthManager] Exchanging code for token...");
        let token_response = Self::exchange_code_for_token(&client, &code, &code_verifier, &tokens_path).await?;

        // Extract cookies from the response
        let cookies = Self::extract_cookies(&client).await?;

        Ok(serde_json::json!({
            "success": true,
            "oauth_token": token_response.get("oauth_token").unwrap_or(&serde_json::json!("")),
            "cookies": cookies,
            "source": "login"
        }))
    }

    async fn wait_for_callback() -> Result<String, Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::channel(1);

        // Clone necessary data for the thread
        let tx_clone = tx.clone();

        // Start callback server in a separate thread
        std::thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:1421").expect("Failed to bind callback server");
            println!("[AuthManager] Callback server listening on port 1421");

            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buffer = [0u8; 1024];
                    if let Ok(bytes_read) = stream.read(&mut buffer) {
                        if bytes_read == 0 {
                            continue;
                        }
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                        println!("[AuthManager] Received callback request");

                        if let Some(code_start) = request.find("code=") {
                            let code_end = request[code_start..].find(' ').unwrap_or(request[code_start..].len());
                            // Clone the code to a new String with 'static lifetime
                            let code = request[code_start + 5..code_start + code_end].to_string();
                            println!("[AuthManager] Found code: {}", code);

                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<!DOCTYPE html><html><head><title>Auth Complete</title></head><body><h1>Authentication Complete!</h1><p>You can close this window.</p><script>window.close();</script></body></html>"
                            );
                            let _ = stream.write(response.as_bytes());

                            let _ = tx_clone.blocking_send(code);
                            break;
                        }
                    }
                }
            }
        });

        // Wait for the code with timeout
        if let Some(code) = rx.recv().await {
            Ok(code)
        } else {
            Err("Timeout waiting for OAuth callback".into())
        }
    }

    async fn exchange_code_for_token(
        client: &reqwest::Client,
        code: &str,
        code_verifier: &str,
        tokens_path: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let token_url = "https://streamlabs.com/api/v5/slobs/auth/token";
        let params = [
            ("code", code),
            ("code_verifier", code_verifier),
            ("grant_type", "authorization_code"),
            ("client_key", CLIENT_KEY),
            ("redirect_uri", REDIRECT_URI),
        ];

        let response = client
            .post(token_url)
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            println!("[AuthManager] Token response: {:?}", data);

            // Save token to file
            if let Some(oauth_token) = data.get("oauth_token")
                .or(data.get("data").and_then(|d| d.get("oauth_token"))) 
            {
                let token_data = AuthData {
                    success: true,
                    data: None,
                    oauth_token: Some(oauth_token.to_string()),
                };
                let _ = fs::write(tokens_path, serde_json::to_string_pretty(&token_data)?);
            }

            Ok(data)
        } else {
            Err(format!("Failed to exchange code: {:?}", response.text().await).into())
        }
    }

    async fn extract_cookies(_client: &reqwest::Client) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        println!("[AuthManager] Cookie extraction via HTTP client is limited");
        println!("[AuthManager] For full cookie extraction, navigate webview to Streamlabs URL");

        Ok(serde_json::json!({
            "note": "Use webview navigation for complete cookie extraction",
            "recommended": "Navigate webview to https://streamlabs.com/tiktok/auth"
        }))
    }
}

// AuthState for Tauri state management
pub struct AuthState {
    pub auth_manager: Mutex<AuthManager>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            auth_manager: Mutex::new(AuthManager::new()),
        }
    }
}
