use std::fs;
use std::path::Path;
use std::sync::Arc;

use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use warp::http::StatusCode;
use warp::ws::WebSocket;

use crate::client_manager::{log, ClientManager};
use crate::config::ALLOWED_EXTENSIONS;
use crate::types::{ExecuteMessage, ExecuteResponse, SimpleMessage, StatusResponse};

/// Handle WebSocket connections from executor clients
pub async fn handle_websocket(ws: WebSocket, client_manager: Arc<ClientManager>) {
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

    // Handle incoming messages from client
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        // Try to parse as SimpleMessage to check for pong
                        if let Ok(parsed) = serde_json::from_str::<SimpleMessage>(text) {
                            if parsed.msg_type == "pong" {
                                // Update pong time silently (no log)
                                client_manager.update_pong(client_id).await;
                            } else {
                                // Log other message types
                                log(&format!(
                                    "Received message from client {}: {}",
                                    client_id, text
                                ));
                            }
                        } else {
                            // If parsing fails, just log it
                            log(&format!(
                                "Received message from client {}: {}",
                                client_id, text
                            ));
                        }
                    }
                } else if msg.is_binary() {
                    log(&format!(
                        "Received binary message from client {}",
                        client_id
                    ));
                }
            }
            Err(_) => break,
        }
    }

    // Cleanup
    send_task.abort();
    client_manager.unregister(client_id).await;
}

/// Handle /execute endpoint - receives file path and broadcasts script to all clients
pub async fn handle_execute(
    body: String,
    client_manager: Arc<ClientManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let file_path_str = body.trim();

    // Validate file path provided
    if file_path_str.is_empty() {
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some("No file path provided".to_string()),
            clients_reached: None,
            total_clients: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    let file_path = Path::new(file_path_str);

    // Validate file exists
    if !file_path.exists() {
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some(format!("File '{}' does not exist", file_path_str)),
            clients_reached: None,
            total_clients: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Validate it's a file
    if !file_path.is_file() {
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some(format!("'{}' is not a file", file_path_str)),
            clients_reached: None,
            total_clients: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
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
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some(format!(
                "File must be one of {:?}, got '{}'",
                ALLOWED_EXTENSIONS, extension
            )),
            clients_reached: None,
            total_clients: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Read file contents
    let code = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            let response = ExecuteResponse {
                success: false,
                message: None,
                error: Some(format!("Error reading file: {}", e)),
                clients_reached: None,
                total_clients: None,
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
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
            let response = ExecuteResponse {
                success: false,
                message: None,
                error: Some(format!("Error serializing message: {}", e)),
                clients_reached: None,
                total_clients: None,
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Broadcast to all clients
    let (successful, total) = client_manager.broadcast(&message_json).await;

    if total == 0 {
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some("No clients connected".to_string()),
            clients_reached: Some(0),
            total_clients: Some(0),
        };
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    } else if successful == total {
        let response = ExecuteResponse {
            success: true,
            message: Some(format!(
                "Script '{}' sent to all connected clients",
                filename
            )),
            error: None,
            clients_reached: Some(successful),
            total_clients: Some(total),
        };
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::OK,
        ))
    } else {
        let response = ExecuteResponse {
            success: false,
            message: None,
            error: Some(format!(
                "Script '{}' only reached {}/{} clients",
                filename, successful, total
            )),
            clients_reached: Some(successful),
            total_clients: Some(total),
        };
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::MULTI_STATUS,
        ))
    }
}

/// Handle /status endpoint - returns server status and client count
pub async fn handle_status(
    client_manager: Arc<ClientManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let status = StatusResponse {
        status: "running".to_string(),
        connected_clients: client_manager.client_count().await,
        timestamp: Local::now().to_rfc3339(),
    };

    Ok(warp::reply::json(&status))
}
