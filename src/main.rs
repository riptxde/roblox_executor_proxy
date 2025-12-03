/*!
Universal Roblox Executor Proxy Server

HTTP server that broadcasts scripts to all connected WebSocket clients
Receives file path via HTTP POST and forwards to all connected executors via WebSocket

Example curl command (Windows CMD):
  curl -X POST http://localhost:13377/execute_file -d "C:\path\to\script.lua"
*/

mod client_manager;
mod config;
mod handlers;
mod types;

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use warp::ws::Ws;
use warp::Filter;

use client_manager::ClientManager;
use config::{ServerConfig, PING_INTERVAL_SECS, PONG_TIMEOUT_SECS};
use handlers::{handle_execute, handle_status, handle_websocket};

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
    let execute_route = warp::path("execute_file")
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

    // Start ping sender background task
    let client_manager_ping = client_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
        loop {
            interval.tick().await;
            client_manager_ping.send_ping().await;
        }
    });

    // Start timeout checker background task
    let client_manager_timeout = client_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let timed_out = client_manager_timeout
                .check_timeouts(PONG_TIMEOUT_SECS)
                .await;
            if !timed_out.is_empty() {
                client_manager_timeout.disconnect_clients(timed_out).await;
            }
        }
    });

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

    // Print server info
    config.print_info();

    warp::serve(http_routes).run(http_socket_addr).await;

    Ok(())
}
