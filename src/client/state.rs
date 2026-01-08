use std::collections::HashMap;

use crate::{Asset, Channel, Message, Profile};

#[derive(Clone, Debug, Default)]
pub struct ChannelState {
    pub channel: Channel,
    pub users: HashMap<String, Profile>,
    pub messages: Vec<Message>,
    pub assets: HashMap<String, Asset>,
}

impl ChannelState {
    pub fn new(channel: Channel) -> Self {
        ChannelState {
            channel,
            users: HashMap::new(),
            messages: Vec::new(),
            assets: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionState {
    pub connection_id: String,
    pub protocol_name: String,
    pub status: ConnectionStatus,
    pub channels: HashMap<String, ChannelState>,
    pub current_channel: Option<String>,
    pub global_users: HashMap<String, Profile>,
    pub global_assets: HashMap<String, Asset>,
    pub current_user_id: Option<String>,
}

impl ConnectionState {
    pub fn new(connection_id: String, protocol_name: String) -> Self {
        ConnectionState {
            connection_id,
            protocol_name,
            status: ConnectionStatus::Disconnected,
            channels: HashMap::new(),
            current_channel: None,
            global_users: HashMap::new(),
            global_assets: HashMap::new(),
            current_user_id: None,
        }
    }

    pub fn get_or_create_channel(&mut self, channel_id: &str) -> &mut ChannelState {
        self.channels.entry(channel_id.to_string()).or_insert_with(|| {
            ChannelState::new(Channel {
                id: channel_id.to_string(),
                name: None,
                channel_type: crate::ChannelType::Group,
            })
        })
    }
}
