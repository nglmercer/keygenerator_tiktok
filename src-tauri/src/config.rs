// src-tauri/src/config.rs
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub token: Option<String>,
    pub title: Option<String>,
    pub game: Option<String>,
    pub audience_type: Option<String>,
    pub suppress_donation_reminder: Option<bool>,
}

#[derive(Clone)]
pub struct ConfigManager {
    config_path: String,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = "config.json".to_string();
        let config = AppConfig {
            audience_type: Some("0".to_string()),
            suppress_donation_reminder: Some(false),
            ..Default::default()
        };

        Self { config_path, config }
    }

    pub fn load(&mut self) -> &AppConfig {
        if let Ok(data) = fs::read_to_string(&self.config_path) {
            if let Ok(parsed) = serde_json::from_str::<AppConfig>(&data) {
                self.config = AppConfig {
                    token: self.config.token.clone().or(parsed.token),
                    title: self.config.title.clone().or(parsed.title),
                    game: self.config.game.clone().or(parsed.game),
                    audience_type: self.config.audience_type.clone().or(parsed.audience_type),
                    suppress_donation_reminder: self.config.suppress_donation_reminder.or(parsed.suppress_donation_reminder),
                };
            }
        }
        &self.config
    }

    pub fn save(&self, new_config: &AppConfig) -> Result<(), std::io::Error> {
        let config_to_save = AppConfig {
            token: self.config.token.clone().or(new_config.token.clone()),
            title: self.config.title.clone().or(new_config.title.clone()),
            game: self.config.game.clone().or(new_config.game.clone()),
            audience_type: self.config.audience_type.clone().or(new_config.audience_type.clone()),
            suppress_donation_reminder: self.config.suppress_donation_reminder.or(new_config.suppress_donation_reminder),
        };

        let json = serde_json::to_string_pretty(&config_to_save)?;
        fs::write(&self.config_path, json)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "token" => self.config.token.clone(),
            "title" => self.config.title.clone(),
            "game" => self.config.game.clone(),
            "audience_type" => self.config.audience_type.clone(),
            _ => None,
        }
    }
}
