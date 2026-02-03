use tokio::sync::broadcast;

use super::message::WsMessage;

#[derive(Debug, Clone)]
pub struct Hub {
    sender: broadcast::Sender<WsMessage>,
}

impl Hub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn broadcast(&self, message: WsMessage) {
        // Ignore error when no receivers are connected
        let _ = self.sender.send(message);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.sender.subscribe()
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new(256)
    }
}
