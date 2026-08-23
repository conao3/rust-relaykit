use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatKind {
    Dm,
    Group,
    Supergroup,
    Channel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatRef {
    pub id: String,
    pub kind: ChatKind,
    pub title: Option<String>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SenderRef {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageRef {
    pub id: String,
    pub text: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayEvent {
    pub id: String,
    pub platform: String,
    pub chat: ChatRef,
    pub sender: Option<SenderRef>,
    pub message: MessageRef,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RelayReplyBatch {
    #[serde(default)]
    pub messages: Vec<RelayReply>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayReply {
    pub text: String,
    pub chat_id: Option<String>,
    pub thread_id: Option<String>,
    pub reply_to_message_id: Option<String>,
}
