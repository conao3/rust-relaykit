use crate::{
    config::BackendConfig,
    protocol::{RelayEvent, RelayReplyBatch},
};
use anyhow::{Context, Result};
use reqwest::StatusCode;

#[derive(Debug, Clone)]
pub struct BackendClient {
    client: reqwest::Client,
    config: BackendConfig,
}

impl BackendClient {
    pub fn new(config: BackendConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn dispatch(&self, event: &RelayEvent) -> Result<RelayReplyBatch> {
        let response = self
            .client
            .post(&self.config.url)
            .json(event)
            .send()
            .await
            .with_context(|| format!("failed to post event {} to backend", event.id))?;

        if response.status() == StatusCode::NO_CONTENT {
            return Ok(RelayReplyBatch::default());
        }

        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .context("failed to read backend response")?;
        if !status.is_success() {
            let body = String::from_utf8_lossy(&bytes);
            anyhow::bail!("backend returned {status}: {body}");
        }

        if bytes.is_empty() {
            return Ok(RelayReplyBatch::default());
        }

        serde_json::from_slice(&bytes).context("failed to parse backend reply JSON")
    }
}
