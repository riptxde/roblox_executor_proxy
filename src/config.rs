use std::env;

use anyhow::{Context, Result};

// Default server settings
const DEFAULT_HTTP_PORT: u16 = 13377;
const DEFAULT_WS_PORT: u16 = 13378;
const DEFAULT_HOST: &str = "localhost";

/// Allowed file extensions for script execution
pub const ALLOWED_EXTENSIONS: &[&str] = &[".lua", ".luau", ".txt"];

/// Interval between ping messages sent to clients
pub const PING_INTERVAL_SECS: u64 = 30;

/// Timeout duration - clients that don't respond within this time are disconnected
pub const PONG_TIMEOUT_SECS: u64 = 90;

/// Server configuration
pub struct ServerConfig {
    pub http_host: String,
    pub http_port: u16,
    pub ws_host: String,
    pub ws_port: u16,
}

impl ServerConfig {
    /// Parse configuration from command-line arguments
    pub fn from_args() -> Result<Self> {
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
                    anyhow::bail!(
                        "Unknown argument: {}\nUsage: {} [--http-port PORT] [--ws-port PORT] [--host HOST]",
                        args[i],
                        args[0]
                    );
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

    /// Print server information to console
    pub fn print_info(&self) {
        println!("\nUniversal Roblox Script Proxy Server\n");
        println!("HTTP Server: http://{}:{}", self.http_host, self.http_port);
        println!("WebSocket Server: ws://{}:{}", self.ws_host, self.ws_port);
        println!("\nWaiting for executor clients to connect...");
        println!("\nExample usage (Windows CMD):");
        println!(
            r#"  curl -X POST http://{}:{}/execute -d "C:\path\to\script.lua""#,
            self.http_host, self.http_port
        );
        println!("\nCheck status:");
        println!("  curl http://{}:{}/status", self.http_host, self.http_port);
        println!("\nPress Ctrl+C to stop\n");
    }
}
