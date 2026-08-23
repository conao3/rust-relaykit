use crate::{
    config::TelegramConfig,
    protocol::{ChatKind, ChatRef, MessageRef, RelayEvent, SenderRef},
};
use anyhow::{Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct TelegramClient {
    client: reqwest::Client,
    base_url: Url,
    allowed_chats: Vec<String>,
    poll_timeout_seconds: u64,
}

impl TelegramClient {
    pub fn from_config(config: &TelegramConfig) -> Result<Self> {
        let token = config.resolve_token()?;
        let base_url = Url::parse(&format!("https://api.telegram.org/bot{token}/"))
            .context("failed to construct Telegram API URL")?;
        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            allowed_chats: config.allowed_chats.clone(),
            poll_timeout_seconds: config.poll_timeout_seconds,
        })
    }

    pub fn poll_timeout_seconds(&self) -> u64 {
        self.poll_timeout_seconds
    }

    pub fn is_allowed_chat(&self, chat_id: &str) -> bool {
        self.allowed_chats.is_empty() || self.allowed_chats.iter().any(|allowed| allowed == chat_id)
    }

    pub async fn get_updates(&self, offset: Option<i64>) -> Result<Vec<TelegramUpdate>> {
        let mut payload = json!({
            "timeout": self.poll_timeout_seconds,
            "allowed_updates": ["message"]
        });
        if let Some(offset) = offset {
            payload["offset"] = json!(offset);
        }

        let response: TelegramResponse<Vec<TelegramUpdate>> = self
            .client
            .post(self.method_url("getUpdates")?)
            .json(&payload)
            .send()
            .await
            .context("failed to call Telegram getUpdates")?
            .json()
            .await
            .context("failed to parse Telegram getUpdates response")?;

        response.into_result("getUpdates")
    }

    pub async fn send_message(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        text: &str,
        reply_to_message_id: Option<&str>,
    ) -> Result<()> {
        let mut payload = json!({
            "chat_id": chat_id,
            "text": text
        });

        if let Some(thread_id) = thread_id.and_then(|id| id.parse::<i64>().ok()) {
            payload["message_thread_id"] = json!(thread_id);
        }
        if let Some(reply_to_message_id) = reply_to_message_id.and_then(|id| id.parse::<i64>().ok())
        {
            payload["reply_parameters"] = json!({ "message_id": reply_to_message_id });
        }

        let response: TelegramResponse<Value> = self
            .client
            .post(self.method_url("sendMessage")?)
            .json(&payload)
            .send()
            .await
            .context("failed to call Telegram sendMessage")?
            .json()
            .await
            .context("failed to parse Telegram sendMessage response")?;

        response.into_result("sendMessage").map(|_| ())
    }

    fn method_url(&self, method: &str) -> Result<Url> {
        self.base_url
            .join(method)
            .with_context(|| format!("failed to build Telegram method URL for {method}"))
    }
}

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

impl<T> TelegramResponse<T> {
    fn into_result(self, method: &str) -> Result<T> {
        if self.ok {
            self.result
                .with_context(|| format!("Telegram {method} response did not contain result"))
        } else {
            anyhow::bail!(
                "Telegram {method} failed: {}",
                self.description
                    .unwrap_or_else(|| "unknown error".to_owned())
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

impl TelegramUpdate {
    pub fn into_relay_event(self) -> Option<RelayEvent> {
        let message = self.message?;
        let raw = serde_json::to_value(&message).unwrap_or(Value::Null);
        let chat_id = message.chat.id.to_string();
        let thread_id = message.message_thread_id.map(|id| id.to_string());
        let title = message.chat.title.clone().or(message.chat.username.clone());
        let sender = message.from.as_ref().map(|from| SenderRef {
            id: from.id.to_string(),
            username: from.username.clone(),
            display_name: from.display_name(),
        });
        let text = message.text.clone().or(message.caption.clone());

        Some(RelayEvent {
            id: format!("telegram:{}", message.message_id),
            platform: "telegram".to_owned(),
            chat: ChatRef {
                id: chat_id,
                kind: message.chat.kind(),
                title,
                thread_id,
            },
            sender,
            message: MessageRef {
                id: message.message_id.to_string(),
                text,
                timestamp: Some(message.date),
            },
            raw,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub message_thread_id: Option<i64>,
    pub from: Option<TelegramUser>,
    pub date: i64,
    pub chat: TelegramChat,
    pub text: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub is_bot: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

impl TelegramUser {
    fn display_name(&self) -> Option<String> {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            (Some(first), None) => Some(first.clone()),
            (None, Some(last)) => Some(last.clone()),
            (None, None) => self.username.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub title: Option<String>,
    pub username: Option<String>,
}

impl TelegramChat {
    fn kind(&self) -> ChatKind {
        match self.chat_type.as_str() {
            "private" => ChatKind::Dm,
            "group" => ChatKind::Group,
            "supergroup" => ChatKind::Supergroup,
            "channel" => ChatKind::Channel,
            _ => ChatKind::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_text_update_to_relay_event() {
        let update = TelegramUpdate {
            update_id: 7,
            message: Some(TelegramMessage {
                message_id: 42,
                message_thread_id: Some(56),
                from: Some(TelegramUser {
                    id: 100,
                    is_bot: Some(false),
                    first_name: Some("Ada".to_owned()),
                    last_name: Some("Lovelace".to_owned()),
                    username: Some("ada".to_owned()),
                }),
                date: 1_787_518_050,
                chat: TelegramChat {
                    id: -1001,
                    chat_type: "supergroup".to_owned(),
                    title: Some("Engineering".to_owned()),
                    username: None,
                },
                text: Some("hello".to_owned()),
                caption: None,
            }),
        };

        let event = update.into_relay_event().unwrap();
        assert_eq!(event.id, "telegram:42");
        assert_eq!(event.chat.id, "-1001");
        assert_eq!(event.chat.kind, ChatKind::Supergroup);
        assert_eq!(event.chat.thread_id.as_deref(), Some("56"));
        assert_eq!(
            event.sender.unwrap().display_name.as_deref(),
            Some("Ada Lovelace")
        );
        assert_eq!(event.message.text.as_deref(), Some("hello"));
    }
}
