use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub enum ChatRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub system: Option<String>,
}

impl From<ChatRequest> for genai::chat::ChatRequest {
    fn from(req: ChatRequest) -> Self {
        let mut chat_req = Self::default();

        if let Some(system) = req.system {
            chat_req = chat_req.with_system(&system);
        }

        for message in req.messages {
            match message.role {
                ChatRole::User => {
                    chat_req = chat_req.append_message(genai::chat::ChatMessage::user(message.content));
                }
                ChatRole::Assistant => {
                    chat_req = chat_req.append_message(genai::chat::ChatMessage::assistant(message.content));
                }
                ChatRole::System => {
                    chat_req = chat_req.append_message(genai::chat::ChatMessage::system(message.content));
                }
            }
        }

        chat_req
    }
}
