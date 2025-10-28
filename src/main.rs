/*!
Universal Roblox Script Proxy Server

HTTP server that broadcasts scripts to all connected WebSocket clients
Receives file path via HTTP POST and forwards to all connected executors via WebSocket

Example curl command (Windows CMD):
  curl -X POST http://localhost:13377/execute -d "C:\path\to\script.lua"
*/

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

// Constants
const DEFAULT_HTTP_PORT: u16 = 13377;
const DEFAULT_WS_PORT: u16 = 13378;
const DEFAULT_HOST: &str = "localhost";
const ALLOWED_EXTENSIONS: &[&str] = &[".lua", ".luau", ".txt"];

// Message structure for broadcasting to clients
#[derive(Serialize, Deserialize)]
struct ExecuteMessage {
    #[serde(rename = "type")]
    msg_type: String,
    script: String,
    filename: String,
    timestamp: String,
}

// Status response structure
#[derive(Serialize)]
struct StatusResponse {
    status: String,
    connected_clients: usize,
    timestamp: String,
}

// Client manager to track WebSocket connections
struct ClientManager {
    clients: Arc<Mutex<HashSet<usize>>>,
    next_id: Arc<Mutex<usize>>,
    senders:
        Arc<Mutex<std::collections::HashMap<usize, tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

impl ClientManager {
    fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashSet::new())),
            next_id: Arc::new(Mutex::new(0)),
            senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn register(&self, sender: tokio::sync::mpsc::UnboundedSender<Message>) -> usize {
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

    async fn unregister(&self, id: usize) {
        let mut clients = self.clients.lock().await;
        clients.remove(&id);
        let count = clients.len();
        drop(clients);

        let mut senders = self.senders.lock().await;
        senders.remove(&id);
        drop(senders);

        log(&format!("Client disconnected. Total clients: {}", count));
    }

    async fn broadcast(&self, message: &str) -> (usize, usize) {
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

    async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }
}

// Logging function with timestamp
fn log(message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] {}", timestamp, message);
}

// Handle WebSocket connections
async fn handle_websocket(ws: WebSocket, client_manager: Arc<ClientManager>) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Register client
    let client_id = client_manager.register(tx).await;

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client (log them but don't act on them)
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        log(&format!("Received message from client: {}", text));
                    }
                } else if msg.is_binary() {
                    log("Received binary message from client");
                }
            }
            Err(_) => break,
        }
    }

    // Cleanup
    send_task.abort();
    client_manager.unregister(client_id).await;
}

// Handle /execute endpoint
async fn handle_execute(
    body: String,
    client_manager: Arc<ClientManager>,
) -> Result<warp::reply::WithStatus<String>, warp::Rejection> {
    let file_path_str = body.trim();

    // Validate file path provided
    if file_path_str.is_empty() {
        return Ok(warp::reply::with_status(
            "Error: No file path provided".to_string(),
            StatusCode::BAD_REQUEST,
        ));
    }

    let file_path = Path::new(file_path_str);

    // Validate file exists
    if !file_path.exists() {
        return Ok(warp::reply::with_status(
            format!("Error: File '{}' does not exist", file_path_str),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Validate it's a file
    if !file_path.is_file() {
        return Ok(warp::reply::with_status(
            format!("Error: '{}' is not a file", file_path_str),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Validate extension
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(warp::reply::with_status(
            format!(
                "Error: File must be one of {:?}, got '{}'",
                ALLOWED_EXTENSIONS, extension
            ),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Read file contents
    let code = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return Ok(warp::reply::with_status(
                format!("Error reading file: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Create message
    let message = ExecuteMessage {
        msg_type: "execute".to_string(),
        script: code,
        filename: filename.to_string(),
        timestamp: Local::now().to_rfc3339(),
    };

    let message_json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(e) => {
            return Ok(warp::reply::with_status(
                format!("Error serializing message: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Broadcast to all clients
    let (successful, total) = client_manager.broadcast(&message_json).await;

    if total == 0 {
        Ok(warp::reply::with_status(
            "[WARNING] No clients connected. Script not sent to any executor.".to_string(),
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    } else if successful == total {
        Ok(warp::reply::with_status(
            format!("[SUCCESS] {} sent to {} client(s)", filename, successful),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            format!(
                "[PARTIAL] {} sent to {}/{} client(s)",
                filename, successful, total
            ),
            StatusCode::MULTI_STATUS,
        ))
    }
}

// Handle /status endpoint
async fn handle_status(
    client_manager: Arc<ClientManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let status = StatusResponse {
        status: "running".to_string(),
        connected_clients: client_manager.client_count().await,
        timestamp: Local::now().to_rfc3339(),
    };

    Ok(warp::reply::json(&status))
}

// Configuration
struct ServerConfig {
    http_host: String,
    http_port: u16,
    ws_host: String,
    ws_port: u16,
}

impl ServerConfig {
    fn from_args() -> Result<Self> {
        let args: Vec<String> = env::args().collect();
        let mut http_host = DEFAULT_HOST.to_string();
        let mut http_port = DEFAULT_HTTP_PORT;
        let mut ws_host = DEFAULT_HOST.to_string();
        let mut ws_port = DEFAULT_WS_PORT;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--http-port" => {
                    if i + 1 < args.len() {
                        http_port = args[i + 1].parse().context("Invalid HTTP port")?;
                        i += 2;
                    } else {
                        anyhow::bail!("--http-port requires a value");
                    }
                }
                "--ws-port" => {
                    if i + 1 < args.len() {
                        ws_port = args[i + 1].parse().context("Invalid WebSocket port")?;
                        i += 2;
                    } else {
                        anyhow::bail!("--ws-port requires a value");
                    }
                }
                "--host" => {
                    if i + 1 < args.len() {
                        http_host = args[i + 1].clone();
                        ws_host = args[i + 1].clone();
                        i += 2;
                    } else {
                        anyhow::bail!("--host requires a value");
                    }
                }
                _ => {
                    anyhow::bail!("Unknown argument: {}\nUsage: {} [--http-port PORT] [--ws-port PORT] [--host HOST]", args[i], args[0]);
                }
            }
        }

        Ok(Self {
            http_host,
            http_port,
            ws_host,
            ws_port,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = ServerConfig::from_args()?;

    // Create client manager
    let client_manager = Arc::new(ClientManager::new());

    // WebSocket route
    let client_manager_ws = client_manager.clone();
    let ws_route = warp::path::end().and(warp::ws()).map(move |ws: Ws| {
        let client_manager = client_manager_ws.clone();
        ws.on_upgrade(move |socket| handle_websocket(socket, client_manager))
    });

    // HTTP routes
    let client_manager_execute = client_manager.clone();
    let execute_route = warp::path("execute")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(move |body: Bytes| {
            let client_manager = client_manager_execute.clone();
            let body_str = String::from_utf8_lossy(&body).to_string();
            handle_execute(body_str, client_manager)
        });

    let client_manager_status = client_manager.clone();
    let status_route = warp::path("status").and(warp::get()).and_then(move || {
        let client_manager = client_manager_status.clone();
        handle_status(client_manager)
    });

    let http_routes = execute_route.or(status_route);

    // Start WebSocket server
    let ws_addr = format!("{}:{}", config.ws_host, config.ws_port);
    let ws_socket_addr = ws_addr
        .to_socket_addrs()
        .context("Failed to resolve WebSocket host:port")?
        .next()
        .context("No addresses resolved for WebSocket host")?;

    tokio::spawn(async move {
        warp::serve(ws_route).run(ws_socket_addr).await;
    });

    // Start HTTP server
    let http_addr = format!("{}:{}", config.http_host, config.http_port);
    let http_socket_addr = http_addr
        .to_socket_addrs()
        .context("Failed to resolve HTTP host:port")?
        .next()
        .context("No addresses resolved for HTTP host")?;

    println!("\nUniversal Roblox Script Proxy Server\n");
    println!(
        "HTTP Server: http://{}:{}",
        config.http_host, config.http_port
    );
    println!(
        "WebSocket Server: ws://{}:{}",
        config.ws_host, config.ws_port
    );
    println!("\nWaiting for executor clients to connect...");
    println!("\nExample usage (Windows CMD):");
    println!(
        r#"  curl -X POST http://{}:{}/execute -d "C:\path\to\script.lua""#,
        config.http_host, config.http_port
    );
    println!("\nCheck status:");
    println!(
        "  curl http://{}:{}/status",
        config.http_host, config.http_port
    );
    println!("\nPress Ctrl+C to stop\n");

    warp::serve(http_routes).run(http_socket_addr).await;

    Ok(())
}
