pub mod mock;

use crate::{Channel, Message};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Connection: Send + Sync {
    async fn connect(&mut self, auth: &Value) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    async fn send(&self, channel_id: &str, message: &Message) -> Result<(), String>;
    async fn receive(&self, channel_id: &str) -> Result<Message, String>;
    async fn add_channel(&mut self, channel: Channel) -> Result<(), String>;
    async fn list_channels(&self) -> Result<Vec<Channel>, String>;
    async fn metadata(&self) -> &Value;
    async fn start_background_tasks(&mut self) -> Result<(), String>;
    async fn stop_background_tasks(&mut self) -> Result<(), String>;
}

pub use mock::MockConnection;
