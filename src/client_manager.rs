use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Local;
use tokio::sync::Mutex;
use warp::ws::Message;

/// Manages WebSocket client connections and message broadcasting
pub struct ClientManager {
    clients: Arc<Mutex<HashSet<usize>>>,
    next_id: Arc<Mutex<usize>>,
    senders: Arc<Mutex<HashMap<usize, tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

impl ClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashSet::new())),
            next_id: Arc::new(Mutex::new(0)),
            senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new client and return its ID
    pub async fn register(&self, sender: tokio::sync::mpsc::UnboundedSender<Message>) -> usize {
        let mut id_guard = self.next_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let mut clients = self.clients.lock().await;
        clients.insert(id);
        let count = clients.len();
        drop(clients);

        let mut senders = self.senders.lock().await;
        senders.insert(id, sender);
        drop(senders);

        log(&format!("Client connected. Total clients: {}", count));
        id
    }

    /// Unregister a client by ID
    pub async fn unregister(&self, id: usize) {
        let mut clients = self.clients.lock().await;
        clients.remove(&id);
        let count = clients.len();
        drop(clients);

        let mut senders = self.senders.lock().await;
        senders.remove(&id);
        drop(senders);

        log(&format!("Client disconnected. Total clients: {}", count));
    }

    /// Broadcast a message to all connected clients
    /// Returns (successful_count, total_count)
    pub async fn broadcast(&self, message: &str) -> (usize, usize) {
        let senders = self.senders.lock().await;
        let total = senders.len();

        if total == 0 {
            return (0, 0);
        }

        let mut successful = 0;
        let mut failed_ids = Vec::new();

        for (id, sender) in senders.iter() {
            if sender.send(Message::text(message.to_string())).is_ok() {
                successful += 1;
            } else {
                log(&format!("Failed to send to client {}", id));
                failed_ids.push(*id);
            }
        }
        drop(senders);

        // Remove failed clients
        if !failed_ids.is_empty() {
            let mut clients = self.clients.lock().await;
            let mut senders = self.senders.lock().await;
            for id in failed_ids {
                clients.remove(&id);
                senders.remove(&id);
            }
        }

        (successful, total)
    }

    /// Get the current number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Log a message with timestamp
pub fn log(message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] {}", timestamp, message);
}
