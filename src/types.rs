use serde::{Deserialize, Serialize};

/// Message structure for broadcasting scripts to executor clients
#[derive(Serialize, Deserialize)]
pub struct ExecuteMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub script: String,
    pub filename: String,
    pub timestamp: String,
}

/// Status response structure for the /status endpoint
#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub connected_clients: usize,
    pub timestamp: String,
}
