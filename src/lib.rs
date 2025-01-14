use chrono::prelude::*;
use serde_json::Value;

pub mod connection;

pub use connection::{Connection, MockConnection};

#[derive(Clone, Debug)]
pub struct Account {
    pub auth: Value,
    pub protocol_name: String,
    pub private_profile: Profile,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub color: Option<[u8; 3]>,
    pub picture: Option<String>,
    pub metadata: Value,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            username: Some("[no username]".to_string()),
            display_name: Some("[no display_name]".to_string()),
            color: Some([255, 255, 255]),
            picture: None,
            metadata: Value::Null,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Option<Profile>,
    pub content: Vec<MessageFragment>,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub enum MessageType {
    CurrentUser,
    Normal,
    Server,
    Meta,
}

#[derive(Clone, Debug)]
pub enum MessageFragment {
    Text(String),
    Image { url: String, mime: String },
    Video { url: String, mime: String },
    Audio { url: String, mime: String },
    Url(String),
    Custom(String, Value),
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub messages: Vec<Message>,
    pub participants: Vec<Profile>,
    pub channel_type: ChannelType,
}

#[derive(Clone, Debug)]
pub enum ChannelType {
    Group,
    Direct,
    Broadcast,
}

#[derive(Clone, Debug)]
pub struct Protocol {
    pub name: &'static str,
    pub auth_fields: Vec<AuthField>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AuthField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
}

#[derive(Clone, Debug)]
pub enum FieldType {
    Text,
    Password,
    Group(Vec<AuthField>),
}

#[derive(Debug)]
pub struct Session<C: Connection> {
    pub id: String,
    pub protocol_name: String,
    pub connection: C,
    pub channels: Vec<Channel>,
    pub metadata: Value,
}
