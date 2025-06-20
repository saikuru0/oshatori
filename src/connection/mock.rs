use crate::{AuthField, Connection, Protocol};
use async_trait::async_trait;
use tokio::sync::broadcast;

use super::ConnectionEvent;

#[derive(Clone, Debug)]
pub struct MockConnection {
    event_tx: broadcast::Sender<ConnectionEvent>,
}

impl MockConnection {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(127);
        MockConnection { event_tx }
    }
}

unsafe impl Send for MockConnection {}
unsafe impl Sync for MockConnection {}

#[async_trait]
impl Connection for MockConnection {
    async fn connect(&mut self, _auth: Vec<AuthField>) -> Result<(), String> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String> {
        self.event_tx.send(event).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent> {
        self.event_tx.subscribe()
    }

    fn protocol_spec(&self) -> Protocol {
        Protocol {
            name: "Mock".to_string(),
            auth: None,
        }
    }
}
