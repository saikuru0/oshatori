use std::collections::HashMap;

use super::state::ConnectionState;

pub trait StateStorage: Send + Sync {
    fn get(&self, connection_id: &str) -> Option<ConnectionState>;
    fn get_mut(&mut self, connection_id: &str) -> Option<&mut ConnectionState>;
    fn insert(&mut self, connection_id: String, state: ConnectionState);
    fn remove(&mut self, connection_id: &str) -> Option<ConnectionState>;
    fn list_connections(&self) -> Vec<String>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryStorage {
    connections: HashMap<String, ConnectionState>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            connections: HashMap::new(),
        }
    }
}

impl StateStorage for InMemoryStorage {
    fn get(&self, connection_id: &str) -> Option<ConnectionState> {
        self.connections.get(connection_id).cloned()
    }

    fn get_mut(&mut self, connection_id: &str) -> Option<&mut ConnectionState> {
        self.connections.get_mut(connection_id)
    }

    fn insert(&mut self, connection_id: String, state: ConnectionState) {
        self.connections.insert(connection_id, state);
    }

    fn remove(&mut self, connection_id: &str) -> Option<ConnectionState> {
        self.connections.remove(connection_id)
    }

    fn list_connections(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }
}
