use crate::services::types::Status;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use std::fs;
use tokio::sync::broadcast::Sender;

const TODO_FILE: &str = "todo_store.json";

pub const MESSAGE_TYPES: &[&str] = &["todo_list", "todo_add", "todo_remove"];

pub fn load_todos() -> Vec<String> {
    fs::read_to_string(TODO_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_todos(todos: &[String]) {
    if let Ok(json) = serde_json::to_string(todos) {
        let _ = fs::write(TODO_FILE, &json);
    }
}

fn broadcast_list(todos: &[String], tx: &Sender<String>) {
    let list = serde_json::json!({"type":"todo_list","items": todos}).to_string();
    let _ = tx.send(list);
}

fn handle_add(todos: &mut Vec<String>, text: &str, tx: &Sender<String>) {
    todos.push(text.to_string());
    save_todos(todos);
    broadcast_list(todos, tx);
}

fn handle_remove(todos: &mut Vec<String>, id: usize, tx: &Sender<String>) {
    if id < todos.len() {
        todos.remove(id);
        save_todos(todos);
        broadcast_list(todos, tx);
    }
}

fn get_list_msg(todos: &[String]) -> String {
    serde_json::json!({"type":"todo_list","items": todos}).to_string()
}

pub async fn handle_message(json: &Value, state: &mut Status, socket: &mut WebSocket) -> bool {
    match json["type"].as_str() {
        Some("todo_list") => {
            let msg = get_list_msg(&state.todos);
            socket.send(Message::Text(msg.into())).await.is_err()
        }
        Some("todo_add") => {
            if let Some(text) = json["text"].as_str() {
                handle_add(&mut state.todos, text, &state.tx);
            }
            false
        }
        Some("todo_remove") => {
            if let Some(id) = json["id"].as_u64() {
                handle_remove(&mut state.todos, id as usize, &state.tx);
            }
            false
        }
        _ => false,
    }
}
