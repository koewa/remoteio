use crate::services::types::Status;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use std::time::Duration;

pub const MESSAGE_TYPES: &[&str] = &["shutdown"];

pub async fn handle_message(json: &Value, state: &mut Status, socket: &mut WebSocket) -> bool {
    match json["type"].as_str() {
        Some("shutdown") => {
            let _ = socket
                .send(Message::Text("Server shutting down...".into()))
                .await;
            let _ = state.tx.send("Server shutting down...".to_string());
            state.shutdown.notify_one();
            tokio::time::sleep(Duration::from_millis(500)).await;
            true
        }
        _ => false,
    }
}
