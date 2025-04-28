use crate::{AuthField, Channel, Message, Profile, Protocol};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChatEvent {
    New {
        channel_id: Option<String>,
        message: Message,
    },
    Update {
        channel_id: Option<String>,
        message_id: String,
        new_message: Message,
    },
    Remove {
        channel_id: Option<String>,
        message_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChannelEvent {
    New {
        channel: Channel,
    },
    Update {
        channel_id: String,
        new_channel: Channel,
    },
    Remove {
        channel_id: String,
    },
    Join {
        channel_id: String,
    },
    Leave {
        channel_id: String,
    },
    Switch {
        channel_id: String,
    },
    Kick {
        channel_id: Option<String>,
        reason: Option<String>,
        ban: bool,
    },
    Wipe {
        channel_id: Option<String>,
    },
    ClearList,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UserEvent {
    New {
        channel_id: Option<String>,
        user: Profile,
    },
    Update {
        channel_id: Option<String>,
        user_id: String,
        new_user: Profile,
    },
    Remove {
        channel_id: Option<String>,
        user_id: String,
    },
    ClearList {
        channel_id: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StatusEvent {
    Ping { artifact: Option<String> },
    Connected { artifact: Option<String> },
    Disconnected { artifact: Option<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConnectionEvent {
    Chat { event: ChatEvent },
    User { event: UserEvent },
    Channel { event: ChannelEvent },
    Status { event: StatusEvent },
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn connect(&mut self, auth: Vec<AuthField>) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String>;
    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent>;
    fn protocol_spec() -> Protocol;
}

#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "mock")]
pub use mock::MockConnection;

#[cfg(feature = "sockchat")]
pub mod sockchat;
#[cfg(feature = "sockchat")]
pub use sockchat::SockchatConnection;
