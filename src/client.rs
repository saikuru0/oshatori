use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::{
    spawn,
    sync::{broadcast, Mutex, RwLock},
};
use uuid::Uuid;

use crate::connection::{
    AssetEvent, ChannelEvent, ChatEvent, Connection, ConnectionEvent, UserEvent,
};

pub type AccountId = Uuid;
pub type ConnectionId = Uuid;

#[derive(Clone, Debug)]
pub struct SelectedContext {
    pub account: AccountId,
    pub connection: ConnectionId,
    pub channel_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct ChannelState {
    pub chat_history: VecDeque<ChatEvent>,
    pub user_list: Vec<UserEvent>,
    pub asset_list: Vec<AssetEvent>,
}

struct AccountClient {
    pub connections: HashMap<ConnectionId, Arc<Mutex<dyn Connection + Send + Sync>>>,
    pub channel_states: HashMap<ConnectionId, HashMap<Option<String>, ChannelState>>,
    inbound_rxs: HashMap<ConnectionId, broadcast::Receiver<ConnectionEvent>>,
    pub selected_connection: Option<ConnectionId>,
}

pub struct StateClient {
    accounts: Arc<RwLock<HashMap<AccountId, AccountClient>>>,
    pub selected: Arc<RwLock<Option<SelectedContext>>>,
}

impl StateClient {
    pub fn new() -> Self {
        StateClient {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            selected: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn send(&self, event: ConnectionEvent) -> Result<(), String> {
        let sel = self
            .selected
            .read()
            .await
            .clone()
            .ok_or("no active context")?;
        let accounts = self.accounts.read().await;
        let ac = accounts.get(&sel.account).ok_or("account not found")?;
        let conn = ac
            .connections
            .get(&sel.connection)
            .ok_or("connection not found")?;
        let mut guard = conn.lock().await;
        guard.send(event).await
    }

    pub async fn add_account(&self) -> AccountId {
        let id = Uuid::new_v4();
        let ac = AccountClient {
            connections: HashMap::new(),
            channel_states: HashMap::new(),
            inbound_rxs: HashMap::new(),
            selected_connection: None,
        };
        self.accounts.write().await.insert(id, ac);
        id
    }

    pub async fn remove_account(&self, acc_id: &AccountId) {
        if let Some(ac) = self.accounts.write().await.remove(acc_id) {
            for (_, conn) in ac.connections {
                let mut c = conn.lock().await;
                let _ = c.disconnect().await;
            }
        }
    }

    pub async fn add_connection(
        &self,
        acc_id: &AccountId,
        conn: impl Connection + Send + Sync + 'static,
    ) -> Option<ConnectionId> {
        let mut accounts = self.accounts.write().await;
        let ac = accounts.get_mut(acc_id)?;
        let cid = Uuid::new_v4();
        let arc_conn = Arc::new(Mutex::new(conn));
        let mut rx = arc_conn.lock().await.subscribe();
        ac.channel_states.insert(cid, HashMap::new());
        ac.inbound_rxs.insert(cid, rx.resubscribe());
        ac.connections.insert(cid, arc_conn.clone());

        let accounts_arc = Arc::clone(&self.accounts);
        let acc_clone = *acc_id;
        spawn(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let key: Option<String> = match &event {
                            ConnectionEvent::Chat { event: e } => e.channel_id().clone(),
                            ConnectionEvent::User { event: e } => e.channel_id().clone(),
                            ConnectionEvent::Asset { event: e } => e.channel_id().clone(),
                            ConnectionEvent::Channel { event: e } => match e {
                                ChannelEvent::New { channel } => Some(channel.id.clone()),
                                ChannelEvent::Update { channel_id, .. }
                                | ChannelEvent::Remove { channel_id }
                                | ChannelEvent::Join { channel_id }
                                | ChannelEvent::Leave { channel_id }
                                | ChannelEvent::Switch { channel_id } => Some(channel_id.clone()),
                                ChannelEvent::Kick { channel_id, .. }
                                | ChannelEvent::Wipe { channel_id } => channel_id.clone(),
                                ChannelEvent::ClearList => None,
                            },
                            ConnectionEvent::Status { .. } => None,
                        };

                        let mut accounts = accounts_arc.write().await;
                        if let Some(ac) = accounts.get_mut(&acc_clone) {
                            let state_map = ac.channel_states.get_mut(&cid).unwrap();
                            let ch_state = state_map.entry(key.clone()).or_default();
                            match event {
                                ConnectionEvent::Chat { event } => {
                                    ch_state.chat_history.push_back(event)
                                }
                                ConnectionEvent::User { event } => ch_state.user_list.push(event),
                                ConnectionEvent::Asset { event } => {
                                    ch_state.asset_list.push(event);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
        ac.selected_connection = Some(cid);
        Some(cid)
    }

    pub async fn select(&self, ctx: SelectedContext) {
        *self.selected.write().await = Some(ctx);
    }

    pub async fn current_state(&self) -> Option<ChannelState> {
        let sel = self.selected.read().await.clone()?;
        let accounts = self.accounts.read().await;
        let ac = accounts.get(&sel.account)?;
        let map = ac.channel_states.get(&sel.connection)?;
        map.get(&sel.channel_id).cloned()
    }

    pub async fn list_accounts(&self) -> Vec<AccountId> {
        self.accounts.read().await.keys().cloned().collect()
    }

    pub async fn list_connections(&self, acc_id: &AccountId) -> Option<Vec<ConnectionId>> {
        Some(
            self.accounts
                .read()
                .await
                .get(acc_id)?
                .connections
                .keys()
                .cloned()
                .collect(),
        )
    }

    pub async fn list_channels(
        &self,
        acc_id: &AccountId,
        conn_id: &ConnectionId,
    ) -> Option<Vec<Option<String>>> {
        Some(
            self.accounts
                .read()
                .await
                .get(acc_id)?
                .channel_states
                .get(conn_id)?
                .keys()
                .cloned()
                .collect(),
        )
    }
}

trait HasChannelId {
    fn channel_id(&self) -> &Option<String>;
}

impl HasChannelId for ChatEvent {
    fn channel_id(&self) -> &Option<String> {
        match self {
            ChatEvent::New { channel_id, .. }
            | ChatEvent::Update { channel_id, .. }
            | ChatEvent::Remove { channel_id, .. } => channel_id,
        }
    }
}

impl HasChannelId for UserEvent {
    fn channel_id(&self) -> &Option<String> {
        match self {
            UserEvent::New { channel_id, .. }
            | UserEvent::Update { channel_id, .. }
            | UserEvent::Remove { channel_id, .. }
            | UserEvent::ClearList { channel_id } => channel_id,
        }
    }
}

impl HasChannelId for AssetEvent {
    fn channel_id(&self) -> &Option<String> {
        match self {
            AssetEvent::New { channel_id, .. }
            | AssetEvent::Update { channel_id, .. }
            | AssetEvent::Remove { channel_id, .. }
            | AssetEvent::ClearList { channel_id } => channel_id,
        }
    }
}
