use chrono::prelude::*;

pub mod connection;

pub use connection::{Connection, MockConnection};

#[derive(Clone, Debug)]
pub struct Account {
    pub auth: Vec<AuthField>,
    pub protocol_name: String,
    pub private_profile: Option<Profile>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub sender: Option<Profile>,
    pub content: Vec<MessageFragment>,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub status: MessageStatus,
}

#[derive(Clone, Debug)]
pub enum MessageStatus {
    Sent,
    Delivered,
    Edited,
    Deleted,
    Failed,
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
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
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
    pub name: String,
    pub auth: Option<AuthField>,
}

#[derive(Clone, Debug)]
pub struct AuthField {
    pub name: String,
    pub value: FieldValue,
    pub required: bool,
}

#[derive(Clone, Debug)]
pub enum FieldValue {
    Text(String),
    Password(String),
    Group(Vec<AuthField>),
}
