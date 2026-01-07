use crate::{AuthField, Connection, Protocol};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use super::ConnectionEvent;

#[derive(Clone, Debug)]
pub struct MockConnection {
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    event_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<ConnectionEvent>>>>,
}

impl MockConnection {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        MockConnection {
            event_tx,
            event_rx: Arc::new(Mutex::new(Some(event_rx))),
        }
    }
}

unsafe impl Send for MockConnection {}
unsafe impl Sync for MockConnection {}

#[async_trait]
impl Connection for MockConnection {
    fn set_auth(&mut self, _auth: Vec<AuthField>) -> Result<(), String> {
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String> {
        self.event_tx.send(event).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn subscribe(&mut self) -> mpsc::UnboundedReceiver<ConnectionEvent> {
        self.event_rx
            .try_lock()
            .ok()
            .and_then(|mut guard| guard.take())
            .expect("subscribe can only be called once")
    }

    fn protocol_spec(&self) -> Protocol {
        Protocol {
            name: "Mock".to_string(),
            auth: None,
        }
    }
}
