use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub enum ChatRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}
