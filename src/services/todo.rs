use crate::services::types::Status;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use std::fs;
use tokio::sync::broadcast::Sender;

const TODO_FILE: &str = "todo_store.json";

pub const MESSAGE_TYPES: &[&str] = &["todo_list", "todo_add", "todo_remove", "todo_reorder", "todo_edit"];

pub fn load_todos() -> Vec<String> {
    load_todos_from(TODO_FILE)
}

fn load_todos_from(path: &str) -> Vec<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_todos(todos: &[String]) {
    save_todos_to(TODO_FILE, todos);
}

fn save_todos_to(path: &str, todos: &[String]) {
    if let Ok(json) = serde_json::to_string_pretty(todos) {
        let _ = fs::write(path, &json);
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

fn handle_reorder(todos: &mut Vec<String>, from: usize, to: usize, tx: &Sender<String>) {
    if from < todos.len() && to < todos.len() {
        let item = todos.remove(from);
        todos.insert(to, item);
        save_todos(todos);
        broadcast_list(todos, tx);
    }
}

fn handle_edit(todos: &mut Vec<String>, id: usize, text: &str, tx: &Sender<String>) {
    if id < todos.len() {
        todos[id] = text.to_string();
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
        Some("todo_reorder") => {
            if let (Some(from), Some(to)) = (json["from"].as_u64(), json["to"].as_u64()) {
                handle_reorder(&mut state.todos, from as usize, to as usize, &state.tx);
            }
            false
        }
        Some("todo_edit") => {
            if let (Some(id), Some(text)) = (json["id"].as_u64(), json["text"].as_str()) {
                handle_edit(&mut state.todos, id as usize, text, &state.tx);
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::channel;

    #[test]
    fn test_get_list_msg() {
        let todos = vec!["alpha".to_string(), "beta".to_string()];
        let msg = get_list_msg(&todos);
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["items"][0], "alpha");
        assert_eq!(v["items"][1], "beta");
    }

    #[test]
    fn test_handle_add() {
        let mut todos = vec![];
        let (tx, mut rx) = channel::<String>(16);
        handle_add(&mut todos, "hello", &tx);
        assert_eq!(todos, vec!["hello"]);
        let broadcast = rx.try_recv().unwrap();
        assert!(broadcast.contains(r#""type":"todo_list""#));
    }

    #[test]
    fn test_handle_remove() {
        let mut todos = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_remove(&mut todos, 1, &tx);
        assert_eq!(todos, vec!["a", "c"]);
    }

    #[test]
    fn test_handle_remove_out_of_bounds() {
        let mut todos = vec!["a".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_remove(&mut todos, 5, &tx);
        assert_eq!(todos.len(), 1);
    }

    #[test]
    fn test_handle_reorder() {
        let mut todos = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_reorder(&mut todos, 0, 2, &tx);
        assert_eq!(todos, vec!["b", "c", "a"]);
    }

    #[test]
    fn test_handle_reorder_same_index() {
        let mut todos = vec!["a".to_string(), "b".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_reorder(&mut todos, 0, 0, &tx);
        assert_eq!(todos, vec!["a", "b"]);
    }

    #[test]
    fn test_handle_edit() {
        let mut todos = vec!["a".to_string(), "b".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_edit(&mut todos, 0, "edited", &tx);
        assert_eq!(todos, vec!["edited", "b"]);
    }

    #[test]
    fn test_handle_edit_out_of_bounds() {
        let mut todos = vec!["a".to_string()];
        let (tx, _) = channel::<String>(16);
        handle_edit(&mut todos, 5, "x", &tx);
        assert_eq!(todos, vec!["a"]);
    }

    #[test]
    fn test_broadcast_list() {
        let todos = vec!["hello".to_string()];
        let (tx, mut rx) = channel::<String>(16);
        broadcast_list(&todos, &tx);
        let msg = rx.try_recv().unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["items"][0], "hello");
    }

    #[test]
    fn test_save_and_load() {
        let tmp = std::env::temp_dir().join("test_remoteio_todos.json");
        let todos = vec!["one".to_string(), "two".to_string()];
        save_todos_to(tmp.to_str().unwrap(), &todos);
        let loaded = load_todos_from(tmp.to_str().unwrap());
        assert_eq!(loaded, todos);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_load_missing_file() {
        let tmp = std::env::temp_dir().join("test_remoteio_nonexistent.json");
        let loaded = load_todos_from(tmp.to_str().unwrap());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_corrupt_file() {
        let tmp = std::env::temp_dir().join("test_remoteio_corrupt.json");
        std::fs::write(&tmp, "not json").unwrap();
        let loaded = load_todos_from(tmp.to_str().unwrap());
        assert!(loaded.is_empty());
        let _ = std::fs::remove_file(&tmp);
    }
}
