use crate::{Channel, Connection, Message, MessageFragment, Profile};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct MockConnection {
    connected: Arc<Mutex<bool>>,
    channels: Arc<Mutex<Vec<Channel>>>,
}

impl MockConnection {
    pub fn new() -> Self {
        MockConnection {
            connected: Arc::new(Mutex::new(false)),
            channels: Arc::new(Mutex::new(vec![Channel {
                id: "general".to_string(),
                name: Some("General".to_string()),
                messages: vec![],
                participants: vec![],
                channel_type: crate::ChannelType::Group,
            }])),
        }
    }
}

#[async_trait]
impl Connection for MockConnection {
    async fn connect(&mut self, _auth: &Value) -> Result<(), String> {
        let mut connected = self.connected.lock().unwrap();
        *connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        let mut connected = self.connected.lock().unwrap();
        *connected = false;
        Ok(())
    }

    async fn send(&self, channel_id: &str, message: &Message) -> Result<(), String> {
        let mut channels = self.channels.lock().unwrap();
        if let Some(channel) = channels.iter_mut().find(|c| c.id == channel_id) {
            channel.messages.push(message.clone());
            Ok(())
        } else {
            Err("Channel not found".to_string())
        }
    }

    async fn receive(&self, _channel_id: &str) -> Result<Message, String> {
        Ok(Message {
            sender: Some(Profile::default()),
            content: vec![MessageFragment::Text("Hello from Mock!".to_string())],
            timestamp: chrono::Utc::now(),
            message_type: crate::MessageType::Normal,
            metadata: Value::Null,
        })
    }

    async fn add_channel(&mut self, channel: Channel) -> Result<(), String> {
        self.channels.lock().unwrap().push(channel);
        Ok(())
    }

    async fn list_channels(&self) -> Result<Vec<Channel>, String> {
        Ok(self.channels.lock().unwrap().clone())
    }

    async fn metadata(&self) -> &Value {
        &Value::Null
    }
    async fn start_background_tasks(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn stop_background_tasks(&mut self) -> Result<(), String> {
        Ok(())
    }
}
