use chrono::prelude::*;
pub mod connection;
pub mod utils;
pub use connection::Connection;
use serde::{Deserialize, Serialize};
pub use utils::assets;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub auth: Vec<AuthField>,
    pub protocol_name: String,
    pub private_profile: Option<Profile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub color: Option<[u8; 4]>,
    pub picture: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            id: None,
            username: None,
            display_name: None,
            color: None,
            picture: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub sender_id: Option<String>,
    pub content: Vec<MessageFragment>,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub status: MessageStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageStatus {
    Sent,
    Delivered,
    Edited,
    Deleted,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageType {
    CurrentUser,
    Normal,
    Server,
    Meta,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MessageFragment {
    Text(String),
    Image { url: String, mime: String },
    Video { url: String, mime: String },
    Audio { url: String, mime: String },
    Url(String),
    AssetId(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Asset {
    Emote {
        id: Option<String>,
        pattern: String,
        src: String,
        source: AssetSource,
    },
    Sticker {
        id: Option<String>,
        pattern: String,
        src: String,
        source: AssetSource,
    },
    Audio {
        id: Option<String>,
        pattern: String,
        src: String,
        source: AssetSource,
    },
    Command {
        id: Option<String>,
        pattern: String,
        args: Vec<MessageFragment>,
        source: AssetSource,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssetSource {
    User,
    Meta,
    Server,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub channel_type: ChannelType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChannelType {
    Group,
    Direct,
    Broadcast,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Protocol {
    pub name: String,
    pub auth: Option<Vec<AuthField>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthField {
    pub name: String,
    pub display: Option<String>,
    pub value: FieldValue,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FieldValue {
    Text(Option<String>),
    Password(Option<String>),
    Group(Vec<AuthField>),
}
