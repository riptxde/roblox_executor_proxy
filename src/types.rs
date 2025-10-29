use serde::{Deserialize, Serialize};

/// Generic message type for ping/pong and other simple messages
#[derive(Serialize, Deserialize)]
pub struct SimpleMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
}

/// Message structure for broadcasting scripts to executor clients
#[derive(Serialize, Deserialize)]
pub struct ExecuteMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub script: String,
    pub filename: String,
    pub timestamp: String,
}

/// Execute response structure for the /execute endpoint
#[derive(Serialize)]
pub struct ExecuteResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clients_reached: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_clients: Option<usize>,
}

/// Status response structure for the /status endpoint
#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub connected_clients: usize,
    pub timestamp: String,
}
