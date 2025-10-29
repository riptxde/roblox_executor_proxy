use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::Local;
use tokio::sync::Mutex;
use warp::ws::Message;

/// Manages WebSocket client connections and message broadcasting
pub struct ClientManager {
    clients: Arc<Mutex<HashSet<usize>>>,
    next_id: Arc<Mutex<usize>>,
    senders: Arc<Mutex<HashMap<usize, tokio::sync::mpsc::UnboundedSender<Message>>>>,
    last_pong: Arc<Mutex<HashMap<usize, Instant>>>,
}

impl ClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashSet::new())),
            next_id: Arc::new(Mutex::new(0)),
            senders: Arc::new(Mutex::new(HashMap::new())),
            last_pong: Arc::new(Mutex::new(HashMap::new())),
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

        let mut last_pong = self.last_pong.lock().await;
        last_pong.insert(id, Instant::now());
        drop(last_pong);

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

        let mut last_pong = self.last_pong.lock().await;
        last_pong.remove(&id);
        drop(last_pong);

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

    /// Update the last pong time for a client
    pub async fn update_pong(&self, id: usize) {
        let mut last_pong = self.last_pong.lock().await;
        last_pong.insert(id, Instant::now());
    }

    /// Send ping message to all clients
    pub async fn send_ping(&self) -> usize {
        let senders = self.senders.lock().await;
        let total = senders.len();

        if total == 0 {
            return 0;
        }

        let ping_message = r#"{"type":"ping"}"#;
        let mut successful = 0;

        for (id, sender) in senders.iter() {
            if sender.send(Message::text(ping_message)).is_ok() {
                successful += 1;
            } else {
                log(&format!("Failed to send ping to client {}", id));
            }
        }

        log(&format!("Sent ping to {}/{} clients", successful, total));
        successful
    }

    /// Check for clients that haven't responded to pings within the timeout
    /// Returns a list of timed-out client IDs
    pub async fn check_timeouts(&self, timeout_secs: u64) -> Vec<usize> {
        let last_pong = self.last_pong.lock().await;
        let now = Instant::now();
        let mut timed_out = Vec::new();

        for (id, last_time) in last_pong.iter() {
            if now.duration_since(*last_time).as_secs() > timeout_secs {
                timed_out.push(*id);
            }
        }

        timed_out
    }

    /// Disconnect clients by their IDs
    pub async fn disconnect_clients(&self, client_ids: Vec<usize>) {
        if client_ids.is_empty() {
            return;
        }

        let mut clients = self.clients.lock().await;
        let mut senders = self.senders.lock().await;
        let mut last_pong = self.last_pong.lock().await;

        for id in client_ids {
            clients.remove(&id);
            senders.remove(&id);
            last_pong.remove(&id);
            log(&format!("Client {} timed out and was disconnected", id));
        }

        let count = clients.len();
        log(&format!("Remaining clients: {}", count));
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
