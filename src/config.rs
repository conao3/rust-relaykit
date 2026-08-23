use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{env, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub backend: BackendConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: Option<String>,
    pub token_env: Option<String>,
    #[serde(default)]
    pub allowed_chats: Vec<String>,
    #[serde(default = "default_poll_timeout_seconds")]
    pub poll_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendConfig {
    pub url: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse TOML config {}", path.display()))
    }
}

impl TelegramConfig {
    pub fn resolve_token(&self) -> Result<String> {
        if let Some(token) = self
            .token
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(token.to_owned());
        }

        let Some(name) = self
            .token_env
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            bail!("telegram token is missing; set telegram.token or telegram.token_env");
        };

        env::var(name).with_context(|| format!("environment variable {name} is not set"))
    }

    pub fn chat_allowed(&self, chat_id: &str) -> bool {
        self.allowed_chats.is_empty() || self.allowed_chats.iter().any(|allowed| allowed == chat_id)
    }
}

fn default_poll_timeout_seconds() -> u64 {
    30
}
