use crate::{AuthField, Channel, Message};
use async_trait::async_trait;
use tokio::sync::broadcast;

#[derive(Clone)]
pub enum ChatEvent {
    New {
        channel_id: String,
        message: Message,
    },
    Update {
        channel_id: String,
        message_id: String,
        new_message: Message,
    },
    Remove {
        channel_id: String,
        message_id: String,
    },
}

#[derive(Clone)]
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
}

#[derive(Clone)]
pub enum ConnectionEvent {
    Chat { event: ChatEvent },
    Channel { event: ChannelEvent },
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn connect(&mut self, auth: Vec<AuthField>) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String>;
    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent>;
    fn protocol_name() -> String;
}

pub mod mock;
pub use mock::MockConnection;
