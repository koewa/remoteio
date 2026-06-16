use std::fs;
use tokio::sync::broadcast::Sender;

const TODO_FILE: &str = "todo_store.json";

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

pub fn handle_add(todos: &mut Vec<String>, text: &str, tx: &Sender<String>) {
    todos.push(text.to_string());
    save_todos(todos);
    broadcast_list(todos, tx);
}

pub fn handle_remove(todos: &mut Vec<String>, id: usize, tx: &Sender<String>) {
    if id < todos.len() {
        todos.remove(id);
        save_todos(todos);
        broadcast_list(todos, tx);
    }
}

pub fn get_list_msg(todos: &[String]) -> String {
    serde_json::json!({"type":"todo_list","items": todos}).to_string()
}
