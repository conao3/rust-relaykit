use crate::{
    config::BackendConfig,
    protocol::{RelayEvent, RelayReplyBatch},
};
use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[derive(Debug, Clone)]
pub struct BackendClient {
    client: reqwest::Client,
    target: BackendTarget,
}

#[derive(Debug, Clone)]
enum BackendTarget {
    Http { url: String },
    Jsonl { command: Vec<String> },
}

impl BackendClient {
    pub fn new(config: BackendConfig) -> Result<Self> {
        let target = match (config.url, config.command) {
            (Some(url), None) => BackendTarget::Http { url },
            (None, Some(command)) if !command.is_empty() => BackendTarget::Jsonl { command },
            (Some(_), Some(_)) => bail!("backend.url and backend.command are mutually exclusive"),
            _ => bail!("backend.url or backend.command is required"),
        };

        Ok(Self {
            client: reqwest::Client::new(),
            target,
        })
    }

    pub async fn dispatch(&self, event: &RelayEvent) -> Result<RelayReplyBatch> {
        match &self.target {
            BackendTarget::Http { url } => self.dispatch_http(url, event).await,
            BackendTarget::Jsonl { command } => dispatch_jsonl(command, event).await,
        }
    }

    async fn dispatch_http(&self, url: &str, event: &RelayEvent) -> Result<RelayReplyBatch> {
        let response = self
            .client
            .post(url)
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
            bail!("backend returned {status}: {body}");
        }

        if bytes.is_empty() {
            return Ok(RelayReplyBatch::default());
        }

        serde_json::from_slice(&bytes).context("failed to parse backend reply JSON")
    }
}

async fn dispatch_jsonl(command: &[String], event: &RelayEvent) -> Result<RelayReplyBatch> {
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to start JSONL backend command {}", command[0]))?;

    let mut stdin = child
        .stdin
        .take()
        .context("JSONL backend stdin was not available")?;
    let stdout = child
        .stdout
        .take()
        .context("JSONL backend stdout was not available")?;

    let mut input = serde_json::to_vec(event).context("failed to serialize relay event")?;
    input.push(b'\n');
    stdin
        .write_all(&input)
        .await
        .context("failed to write relay event JSONL to backend")?;
    stdin
        .shutdown()
        .await
        .context("failed to close JSONL backend stdin")?;

    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .await
        .context("failed to read relay reply JSONL from backend")?;

    let status = child
        .wait()
        .await
        .context("failed to wait for JSONL backend command")?;
    if !status.success() {
        bail!("JSONL backend command exited with {status}");
    }

    if line.trim().is_empty() {
        Ok(RelayReplyBatch::default())
    } else {
        serde_json::from_str(&line).context("failed to parse JSONL backend reply")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ChatKind, ChatRef, MessageRef, RelayEvent};
    use serde_json::json;

    #[tokio::test]
    async fn dispatches_jsonl_backend() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let client = BackendClient::new(BackendConfig {
            url: None,
            command: Some(vec![
                shell,
                "-c".to_owned(),
                "read line; printf '%s\n' '{\"messages\":[{\"text\":\"ok\",\"chat_id\":null,\"thread_id\":null,\"reply_to_message_id\":null}]}'".to_owned(),
            ]),
        })
        .unwrap();

        let replies = client
            .dispatch(&RelayEvent {
                id: "telegram:1".to_owned(),
                platform: "telegram".to_owned(),
                chat: ChatRef {
                    id: "1".to_owned(),
                    kind: ChatKind::Dm,
                    title: None,
                    thread_id: None,
                },
                sender: None,
                message: MessageRef {
                    id: "1".to_owned(),
                    text: Some("hello".to_owned()),
                    timestamp: Some(1),
                },
                raw: json!({}),
            })
            .await
            .unwrap();

        assert_eq!(replies.messages[0].text, "ok");
    }

    #[test]
    fn rejects_missing_backend_target() {
        assert!(
            BackendClient::new(BackendConfig {
                url: None,
                command: None,
            })
            .is_err()
        );
    }

    #[test]
    fn rejects_multiple_backend_targets() {
        assert!(
            BackendClient::new(BackendConfig {
                url: Some("http://127.0.0.1:8080".to_owned()),
                command: Some(vec!["cat".to_owned()]),
            })
            .is_err()
        );
    }
}
