// src-tauri/src/api.rs
#![allow(dead_code)]
use reqwest;
use serde::{Deserialize, Serialize};
use std::fmt;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub rtmp_url: String,
    pub stream_key: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCategory {
    pub id: String,
    pub full_name: String,
    pub game_mask_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIError {
    HttpError(String),
    ParseError(String),
    MissingToken,
    MissingData,
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            APIError::HttpError(e) => write!(f, "HTTP Error: {}", e),
            APIError::ParseError(e) => write!(f, "Parse Error: {}", e),
            APIError::MissingToken => write!(f, "Missing authentication token"),
            APIError::MissingData => write!(f, "Missing data in response"),
        }
    }
}

impl std::error::Error for APIError {}

#[derive(Clone)]
pub struct StreamAPI {
    client: reqwest::Client,
    current_stream_id: Option<String>,
    base_url: String,
}

impl StreamAPI {
    pub fn new(token: &str) -> Self {
        let client = reqwest::Client::builder()
            .default_headers(
                reqwest::header::HeaderMap::from_iter([
                    (
                        reqwest::header::USER_AGENT,
                        reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) StreamlabsDesktop/1.17.0 Chrome/122.0.6261.156 Safari/537.36"),
                    ),
                    (
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                    ),
                ])
            )
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            current_stream_id: None,
            base_url: "https://streamlabs.com/api/v5/slobs/tiktok".to_string(),
        }
    }

    fn get_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn search(&self, game: &str) -> Result<Vec<StreamCategory>, APIError> {
        if game.trim().is_empty() {
            return self.get_initial_categories().await;
        }

        let query = game.trim();
        let truncated_game = if query.len() > 25 {
            &query[..25]
        } else {
            query
        };

        println!("[StreamAPI] Searching for category: \"{}\"", truncated_game);

        // Use percent encoding for the category parameter
        let encoded_category: String = utf8_percent_encode(truncated_game, NON_ALPHANUMERIC).collect();
        let url = format!("/info?category={}", encoded_category);
        
        let response = self
            .client
            .get(&self.get_url(&url))
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            let results = json.get("categories").and_then(|v| v.as_array()).map(|v| v.to_vec()).unwrap_or_default();
            let categories: Vec<StreamCategory> = serde_json::from_value(serde_json::Value::Array(results))
                .unwrap_or_default();
            println!("[StreamAPI] Found {} matches for \"{}\"", categories.len(), truncated_game);
            Ok(categories)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_initial_categories(&self) -> Result<Vec<StreamCategory>, APIError> {
        let response = self
            .client
            .get(&self.get_url("/info?category=gaming"))
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            let results = json.get("categories").and_then(|v| v.as_array()).map(|v| v.to_vec()).unwrap_or_default();
            let mut categories: Vec<StreamCategory> = serde_json::from_value(serde_json::Value::Array(results))
                .unwrap_or_default();
            categories.truncate(20);
            Ok(categories)
        } else {
            Ok(vec![StreamCategory {
                id: "other".to_string(),
                full_name: "Other".to_string(),
                game_mask_id: "".to_string(),
            }])
        }
    }

    pub async fn start(&mut self, title: &str, category: &str) -> Result<Option<StreamInfo>, APIError> {
        let title_owned = title.to_string();
        let category_owned = category.to_string();
        
        let form = reqwest::multipart::Form::new()
            .text("title", title_owned)
            .text("device_platform", "win32")
            .text("category", category_owned)
            .text("audience_type", "0");

        let response = self
            .client
            .post(&self.get_url("/stream/start"))
            .multipart(form)
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            
            if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                self.current_stream_id = Some(id.to_string());
                Ok(Some(StreamInfo {
                    rtmp_url: json.get("rtmp").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    stream_key: json.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    id: id.to_string(),
                }))
            } else {
                println!("Error starting stream, unexpected response: {:?}", json);
                Ok(None)
            }
        } else {
            println!("Error starting stream: {:?}", response.text().await);
            Ok(None)
        }
    }

    pub async fn end(&mut self) -> Result<bool, APIError> {
        let id = match &self.current_stream_id {
            Some(id) => id.clone(),
            None => return Ok(false),
        };

        let response = self
            .client
            .post(&self.get_url(&format!("/stream/{}/end", id)))
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            let success = json.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            if success {
                self.current_stream_id = None;
            }
            Ok(success)
        } else {
            Ok(false)
        }
    }

    pub async fn get_info(&self) -> Result<serde_json::Value, APIError> {
        let response = self
            .client
            .get(&self.get_url("/info"))
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            println!("[StreamAPI] Info response: {}", json);
            Ok(json)
        } else {
            Err(APIError::HttpError(format!("Status: {}", response.status())))
        }
    }

    pub async fn get_user_profile(&self) -> Result<Option<UserProfile>, APIError> {
        let data = self.get_info().await?;
        let user = data.get("user").and_then(|v| v.as_object()).map(|v| {
            UserProfile {
                username: v.get("username").and_then(|s| s.as_str()).map(|s| s.to_string()),
                display_name: v.get("display_name").and_then(|s| s.as_str()).map(|s| s.to_string()),
                avatar_url: v.get("avatar_url").and_then(|s| s.as_str()).map(|s| s.to_string()),
                avatar_thumb: v.get("avatar_thumb").and_then(|s| s.as_str()).map(|s| s.to_string()),
            }
        });
        Ok(user)
    }

    pub async fn get_current_stream(&self) -> Result<Option<serde_json::Value>, APIError> {
        let response = self
            .client
            .get(&self.get_url("/stream/current"))
            .send()
            .await
            .map_err(|e| APIError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await
                .map_err(|e| APIError::ParseError(e.to_string()))?;
            Ok(Some(json))
        } else {
            Ok(None)
        }
    }
}
