use std::sync::Arc;

use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::{
    connection::{AssetEvent, ChannelEvent, ChatEvent, ConnectionEvent, StatusEvent, UserEvent},
    Asset, Message, Profile,
};

use super::{
    state::{ChannelState, ConnectionState, ConnectionStatus},
    storage::{InMemoryStorage, StateStorage},
};

pub struct StateClient<S: StateStorage = InMemoryStorage> {
    storage: Arc<RwLock<S>>,
}

impl StateClient<InMemoryStorage> {
    pub fn new() -> Self {
        StateClient {
            storage: Arc::new(RwLock::new(InMemoryStorage::new())),
        }
    }
}

impl<S: StateStorage + 'static> StateClient<S> {
    pub fn with_storage(storage: S) -> Self {
        StateClient {
            storage: Arc::new(RwLock::new(storage)),
        }
    }

    pub async fn track(&self, protocol_name: &str) -> String {
        let connection_id = Uuid::new_v4().to_string();
        let state = ConnectionState::new(connection_id.clone(), protocol_name.to_string());
        self.storage
            .write()
            .await
            .insert(connection_id.clone(), state);
        connection_id
    }

    pub async fn untrack(&self, connection_id: &str) {
        self.storage.write().await.remove(connection_id);
    }

    pub async fn process(&self, connection_id: &str, event: ConnectionEvent) {
        let mut storage = self.storage.write().await;
        let Some(state) = storage.get_mut(connection_id) else {
            return;
        };

        match event {
            ConnectionEvent::Status { event } => {
                self.process_status(state, event);
            }
            ConnectionEvent::Channel { event } => {
                self.process_channel(state, event);
            }
            ConnectionEvent::User { event } => {
                self.process_user(state, event);
            }
            ConnectionEvent::Chat { event } => {
                self.process_chat(state, event);
            }
            ConnectionEvent::Asset { event } => {
                self.process_asset(state, event);
            }
        }
    }

    fn process_status(&self, state: &mut ConnectionState, event: StatusEvent) {
        match event {
            StatusEvent::Connected { .. } => {
                state.status = ConnectionStatus::Connected;
            }
            StatusEvent::Disconnected { .. } => {
                state.status = ConnectionStatus::Disconnected;
            }
            StatusEvent::Ping { .. } => {}
        }
    }

    fn process_channel(&self, state: &mut ConnectionState, event: ChannelEvent) {
        match event {
            ChannelEvent::New { channel } => {
                state
                    .channels
                    .entry(channel.id.clone())
                    .or_insert_with(|| ChannelState::new(channel));
            }
            ChannelEvent::Update {
                channel_id,
                new_channel,
            } => {
                if let Some(channel_state) = state.channels.get_mut(&channel_id) {
                    channel_state.channel = new_channel;
                }
            }
            ChannelEvent::Remove { channel_id } => {
                state.channels.remove(&channel_id);
            }
            ChannelEvent::Join { channel_id } => {
                state.get_or_create_channel(&channel_id);
            }
            ChannelEvent::Leave { channel_id } => {
                if state.current_channel.as_ref() == Some(&channel_id) {
                    state.current_channel = None;
                }
            }
            ChannelEvent::Switch { channel_id } => {
                state.current_channel = Some(channel_id);
            }
            ChannelEvent::Kick { .. } => {
                state.current_channel = None;
            }
            ChannelEvent::Wipe { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(channel_state) = state.channels.get_mut(&cid) {
                        channel_state.messages.clear();
                    }
                }
            }
            ChannelEvent::ClearList => {
                state.channels.clear();
            }
        }
    }

    fn process_user(&self, state: &mut ConnectionState, event: UserEvent) {
        match event {
            UserEvent::New { channel_id, user } => {
                let user_id = user.id.clone().unwrap_or_default();
                if let Some(cid) = channel_id {
                    let channel = state.get_or_create_channel(&cid);
                    channel.users.insert(user_id, user);
                } else {
                    state.global_users.insert(user_id, user);
                }
            }
            UserEvent::Update {
                channel_id,
                user_id,
                new_user,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.users.insert(user_id, new_user);
                    }
                } else {
                    state.global_users.insert(user_id, new_user);
                }
            }
            UserEvent::Remove {
                channel_id,
                user_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.users.remove(&user_id);
                    }
                } else {
                    state.global_users.remove(&user_id);
                }
            }
            UserEvent::ClearList { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.users.clear();
                    }
                } else {
                    state.global_users.clear();
                }
            }
            UserEvent::Identify { user_id } => {
                state.current_user_id = Some(user_id);
            }
        }
    }

    fn process_chat(&self, state: &mut ConnectionState, event: ChatEvent) {
        match event {
            ChatEvent::New {
                channel_id,
                message,
            } => {
                if let Some(cid) = channel_id {
                    let channel = state.get_or_create_channel(&cid);
                    channel.messages.push(message);
                }
            }
            ChatEvent::Update {
                channel_id,
                message_id,
                new_message,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        if let Some(msg) = channel
                            .messages
                            .iter_mut()
                            .find(|m| m.id.as_ref() == Some(&message_id))
                        {
                            *msg = new_message;
                        }
                    }
                }
            }
            ChatEvent::Remove {
                channel_id,
                message_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel
                            .messages
                            .retain(|m| m.id.as_ref() != Some(&message_id));
                    }
                }
            }
        }
    }

    fn process_asset(&self, state: &mut ConnectionState, event: AssetEvent) {
        match event {
            AssetEvent::New { channel_id, asset } => {
                let asset_id = get_asset_id(&asset).unwrap_or_default();
                if let Some(cid) = channel_id {
                    let channel = state.get_or_create_channel(&cid);
                    channel.assets.insert(asset_id, asset);
                } else {
                    state.global_assets.insert(asset_id, asset);
                }
            }
            AssetEvent::Update {
                channel_id,
                asset_id,
                new_asset,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.assets.insert(asset_id, new_asset);
                    }
                } else {
                    state.global_assets.insert(asset_id, new_asset);
                }
            }
            AssetEvent::Remove {
                channel_id,
                asset_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.assets.remove(&asset_id);
                    }
                } else {
                    state.global_assets.remove(&asset_id);
                }
            }
            AssetEvent::ClearList { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(channel) = state.channels.get_mut(&cid) {
                        channel.assets.clear();
                    }
                } else {
                    state.global_assets.clear();
                }
            }
        }
    }

    pub fn spawn_processor(
        &self,
        connection_id: String,
        mut rx: mpsc::UnboundedReceiver<ConnectionEvent>,
    ) -> JoinHandle<()> {
        let storage = self.storage.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut storage = storage.write().await;
                if let Some(state) = storage.get_mut(&connection_id) {
                    process_event(state, event);
                }
            }
        })
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectionState> {
        self.storage.read().await.get(connection_id)
    }

    pub async fn get_channel(&self, connection_id: &str, channel_id: &str) -> Option<ChannelState> {
        let storage = self.storage.read().await;
        let state = storage.get(connection_id)?;
        state.channels.get(channel_id).cloned()
    }

    pub async fn get_user(&self, connection_id: &str, user_id: &str) -> Option<Profile> {
        let storage = self.storage.read().await;
        let state = storage.get(connection_id)?;

        if let Some(user) = state.global_users.get(user_id) {
            return Some(user.clone());
        }

        for channel in state.channels.values() {
            if let Some(user) = channel.users.get(user_id) {
                return Some(user.clone());
            }
        }

        None
    }

    pub async fn get_messages(&self, connection_id: &str, channel_id: &str) -> Vec<Message> {
        let storage = self.storage.read().await;
        let Some(state) = storage.get(connection_id) else {
            return Vec::new();
        };
        state
            .channels
            .get(channel_id)
            .map(|c| c.messages.clone())
            .unwrap_or_default()
    }

    pub async fn get_assets(&self, connection_id: &str, channel_id: Option<&str>) -> Vec<Asset> {
        let storage = self.storage.read().await;
        let Some(state) = storage.get(connection_id) else {
            return Vec::new();
        };

        match channel_id {
            Some(cid) => state
                .channels
                .get(cid)
                .map(|c| c.assets.values().cloned().collect())
                .unwrap_or_default(),
            None => state.global_assets.values().cloned().collect(),
        }
    }

    pub async fn list_connections(&self) -> Vec<String> {
        self.storage.read().await.list_connections()
    }
}

impl Default for StateClient<InMemoryStorage> {
    fn default() -> Self {
        Self::new()
    }
}

fn get_asset_id(asset: &Asset) -> Option<String> {
    match asset {
        Asset::Emote { id, .. } => id.clone(),
        Asset::Sticker { id, .. } => id.clone(),
        Asset::Audio { id, .. } => id.clone(),
        Asset::Command { id, .. } => id.clone(),
    }
}

fn process_event(state: &mut ConnectionState, event: ConnectionEvent) {
    match event {
        ConnectionEvent::Status { event } => match event {
            StatusEvent::Connected { .. } => state.status = ConnectionStatus::Connected,
            StatusEvent::Disconnected { .. } => state.status = ConnectionStatus::Disconnected,
            StatusEvent::Ping { .. } => {}
        },
        ConnectionEvent::Channel { event } => match event {
            ChannelEvent::New { channel } => {
                state
                    .channels
                    .entry(channel.id.clone())
                    .or_insert_with(|| ChannelState::new(channel));
            }
            ChannelEvent::Update {
                channel_id,
                new_channel,
            } => {
                if let Some(cs) = state.channels.get_mut(&channel_id) {
                    cs.channel = new_channel;
                }
            }
            ChannelEvent::Remove { channel_id } => {
                state.channels.remove(&channel_id);
            }
            ChannelEvent::Join { channel_id } => {
                state.get_or_create_channel(&channel_id);
            }
            ChannelEvent::Leave { channel_id } => {
                if state.current_channel.as_ref() == Some(&channel_id) {
                    state.current_channel = None;
                }
            }
            ChannelEvent::Switch { channel_id } => {
                state.current_channel = Some(channel_id);
            }
            ChannelEvent::Kick { .. } => {
                state.current_channel = None;
            }
            ChannelEvent::Wipe { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.messages.clear();
                    }
                }
            }
            ChannelEvent::ClearList => {
                state.channels.clear();
            }
        },
        ConnectionEvent::User { event } => match event {
            UserEvent::New { channel_id, user } => {
                let uid = user.id.clone().unwrap_or_default();
                if let Some(cid) = channel_id {
                    state.get_or_create_channel(&cid).users.insert(uid, user);
                } else {
                    state.global_users.insert(uid, user);
                }
            }
            UserEvent::Update {
                channel_id,
                user_id,
                new_user,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.users.insert(user_id, new_user);
                    }
                } else {
                    state.global_users.insert(user_id, new_user);
                }
            }
            UserEvent::Remove {
                channel_id,
                user_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.users.remove(&user_id);
                    }
                } else {
                    state.global_users.remove(&user_id);
                }
            }
            UserEvent::ClearList { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.users.clear();
                    }
                } else {
                    state.global_users.clear();
                }
            }
            UserEvent::Identify { user_id } => {
                state.current_user_id = Some(user_id);
            }
        },
        ConnectionEvent::Chat { event } => match event {
            ChatEvent::New {
                channel_id,
                message,
            } => {
                if let Some(cid) = channel_id {
                    state.get_or_create_channel(&cid).messages.push(message);
                }
            }
            ChatEvent::Update {
                channel_id,
                message_id,
                new_message,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        if let Some(m) = cs
                            .messages
                            .iter_mut()
                            .find(|m| m.id.as_ref() == Some(&message_id))
                        {
                            *m = new_message;
                        }
                    }
                }
            }
            ChatEvent::Remove {
                channel_id,
                message_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.messages.retain(|m| m.id.as_ref() != Some(&message_id));
                    }
                }
            }
        },
        ConnectionEvent::Asset { event } => match event {
            AssetEvent::New { channel_id, asset } => {
                let aid = get_asset_id(&asset).unwrap_or_default();
                if let Some(cid) = channel_id {
                    state.get_or_create_channel(&cid).assets.insert(aid, asset);
                } else {
                    state.global_assets.insert(aid, asset);
                }
            }
            AssetEvent::Update {
                channel_id,
                asset_id,
                new_asset,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.assets.insert(asset_id, new_asset);
                    }
                } else {
                    state.global_assets.insert(asset_id, new_asset);
                }
            }
            AssetEvent::Remove {
                channel_id,
                asset_id,
            } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.assets.remove(&asset_id);
                    }
                } else {
                    state.global_assets.remove(&asset_id);
                }
            }
            AssetEvent::ClearList { channel_id } => {
                if let Some(cid) = channel_id {
                    if let Some(cs) = state.channels.get_mut(&cid) {
                        cs.assets.clear();
                    }
                } else {
                    state.global_assets.clear();
                }
            }
        },
    }
}
