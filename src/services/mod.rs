pub mod types;
pub mod process;
pub mod server;
pub mod todo;

pub use process::setup_process_monitor;
pub use todo::load_todos;

use types::Status;
use axum::extract::ws::WebSocket;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn dispatch(
    type_str: &str,
    json: &Value,
    state: &Arc<Mutex<Status>>,
    socket: &mut WebSocket,
) -> bool {
    if server::MESSAGE_TYPES.contains(&type_str) {
        let mut s = state.lock().await;
        return server::handle_message(json, &mut s, socket).await;
    }
    if todo::MESSAGE_TYPES.contains(&type_str) {
        let mut s = state.lock().await;
        return todo::handle_message(json, &mut s, socket).await;
    }
    false
}
