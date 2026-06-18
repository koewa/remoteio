use crate::services::types::Status;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

const TODO_FILE: &str = "todo_store.json";

pub const MESSAGE_TYPES: &[&str] = &[
    "todo_list", "todo_add", "todo_remove", "todo_reorder", "todo_edit", "todo_move",
    "todo_list_create", "todo_list_delete", "todo_list_rename",
];

pub fn load_todos() -> HashMap<String, Vec<String>> {
    match fs::read_to_string(TODO_FILE) {
        Ok(s) => {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<String>>>(&s) {
                return map;
            }
            // old format: plain array -> migrate to "default" list
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&s) {
                let mut map = HashMap::new();
                map.insert("default".to_string(), arr);
                return map;
            }
            HashMap::new()
        }
        Err(_) => {
            let mut map = HashMap::new();
            map.insert("default".to_string(), Vec::new());
            save_todos(&map);
            map
        }
    }
}

fn save_todos(lists: &HashMap<String, Vec<String>>) {
    if let Ok(json) = serde_json::to_string_pretty(lists) {
        let _ = fs::write(TODO_FILE, &json);
    }
}

fn broadcast_lists(lists: &HashMap<String, Vec<String>>, tx: &tokio::sync::broadcast::Sender<String>) {
    let list_names: Vec<&String> = lists.keys().collect();
    let msg = serde_json::json!({
        "type": "todo_list",
        "lists": lists,
        "listNames": list_names,
    }).to_string();
    let _ = tx.send(msg);
}

fn list_name(json: &Value) -> String {
    json["list"].as_str().map(|s| s.to_string()).unwrap_or_else(|| "default".to_string())
}

pub async fn handle_message(json: &Value, state: &mut Status, socket: &mut WebSocket) -> bool {
    match json["type"].as_str() {
        Some("todo_list") => {
            let list_names: Vec<&String> = state.todo_lists.keys().collect();
            let msg = serde_json::json!({
                "type": "todo_list",
                "lists": &state.todo_lists,
                "listNames": list_names,
            }).to_string();
            if socket.send(Message::Text(msg.into())).await.is_err() {
                return true;
            }
            false
        }
        Some("todo_add") => {
            if let Some(text) = json["text"].as_str() {
                let list = list_name(json);
                state.todo_lists.entry(list).or_default().push(text.to_string());
                save_todos(&state.todo_lists);
                broadcast_lists(&state.todo_lists, &state.tx);
            }
            false
        }
        Some("todo_remove") => {
            if let Some(id) = json["id"].as_u64() {
                let list = list_name(json);
                if let Some(items) = state.todo_lists.get_mut(&list) {
                    if (id as usize) < items.len() {
                        items.remove(id as usize);
                        save_todos(&state.todo_lists);
                        broadcast_lists(&state.todo_lists, &state.tx);
                    }
                }
            }
            false
        }
        Some("todo_reorder") => {
            if let (Some(from), Some(to)) = (json["from"].as_u64(), json["to"].as_u64()) {
                let list = list_name(json);
                if let Some(items) = state.todo_lists.get_mut(&list) {
                    if from < items.len() as u64 && to < items.len() as u64 {
                        let item = items.remove(from as usize);
                        items.insert(to as usize, item);
                        save_todos(&state.todo_lists);
                        broadcast_lists(&state.todo_lists, &state.tx);
                    }
                }
            }
            false
        }
        Some("todo_edit") => {
            if let (Some(id), Some(text)) = (json["id"].as_u64(), json["text"].as_str()) {
                let list = list_name(json);
                if let Some(items) = state.todo_lists.get_mut(&list) {
                    if (id as usize) < items.len() {
                        items[id as usize] = text.to_string();
                        save_todos(&state.todo_lists);
                        broadcast_lists(&state.todo_lists, &state.tx);
                    }
                }
            }
            false
        }
        Some("todo_list_create") => {
            if let Some(name) = json["name"].as_str() {
                if !state.todo_lists.contains_key(name) {
                    state.todo_lists.insert(name.to_string(), Vec::new());
                    save_todos(&state.todo_lists);
                    broadcast_lists(&state.todo_lists, &state.tx);
                }
            }
            false
        }
        Some("todo_list_delete") => {
            if let Some(name) = json["name"].as_str() {
                if state.todo_lists.len() > 1 {
                    state.todo_lists.remove(name);
                    save_todos(&state.todo_lists);
                    broadcast_lists(&state.todo_lists, &state.tx);
                }
            }
            false
        }
        Some("todo_list_rename") => {
            if let (Some(from), Some(to)) = (json["old"].as_str(), json["name"].as_str()) {
                if let Some(items) = state.todo_lists.remove(from) {
                    state.todo_lists.insert(to.to_string(), items);
                    save_todos(&state.todo_lists);
                    broadcast_lists(&state.todo_lists, &state.tx);
                }
            }
            false
        }
        Some("todo_move") => {
            if let Some(id) = json["id"].as_u64() {
                let from = list_name(json);
                if let Some(to) = json["to"].as_str() {
                    if to != from && state.todo_lists.contains_key(to) {
                        if let Some(items) = state.todo_lists.get_mut(&from) {
                            if (id as usize) < items.len() {
                                let item = items.remove(id as usize);
                                state.todo_lists.get_mut(to).unwrap().push(item);
                                save_todos(&state.todo_lists);
                                broadcast_lists(&state.todo_lists, &state.tx);
                            }
                        }
                    }
                }
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
    use crate::services::types::ServerState;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::Notify;

    fn make_state() -> Status {
        let mut todo_lists = HashMap::new();
        todo_lists.insert("default".to_string(), vec!["alpha".to_string(), "beta".to_string()]);
        Status {
            state: ServerState::Disconnected,
            tx: channel::<String>(16).0,
            shutdown: Arc::new(Notify::new()),
            todo_lists,
            syncing: HashSet::new(),
        }
    }

    #[test]
    fn test_get_list_msg() {
        let state = make_state();
        let items = state.todo_lists.get("default").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "alpha");
        assert_eq!(items[1], "beta");
    }

    #[test]
    fn test_handle_add() {
        let mut state = make_state();
        let (tx, mut rx) = channel::<String>(16);
        state.tx = tx;
        state.todo_lists.entry("default".to_string()).or_default().push("hello".to_string());
        save_todos(&state.todo_lists);
        broadcast_lists(&state.todo_lists, &state.tx);
        assert_eq!(state.todo_lists["default"].len(), 3);
        let broadcast = rx.try_recv().unwrap();
        assert!(broadcast.contains(r#""type":"todo_list""#));
        assert!(broadcast.contains(r#""lists""#));
    }

    #[test]
    fn test_handle_add_to_named_list() {
        let mut state = make_state();
        let (tx, mut rx) = channel::<String>(16);
        state.tx = tx;
        state.todo_lists.insert("work".to_string(), vec![]);
        state.todo_lists.get_mut("work").unwrap().push("task".to_string());
        save_todos(&state.todo_lists);
        broadcast_lists(&state.todo_lists, &state.tx);
        assert_eq!(state.todo_lists["work"].len(), 1);
        let broadcast = rx.try_recv().unwrap();
        assert!(broadcast.contains(r#""listNames""#));
    }

    #[test]
    fn test_handle_remove() {
        let mut state = make_state();
        let items = state.todo_lists.get_mut("default").unwrap();
        assert_eq!(items.len(), 2);
        items.remove(1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "alpha");
    }

    #[test]
    fn test_handle_remove_out_of_bounds() {
        let mut state = make_state();
        let json = serde_json::json!({"type":"todo_remove","id":5});
        let list = list_name(&json);
        if let Some(items) = state.todo_lists.get_mut(&list) {
            if (json["id"].as_u64().unwrap() as usize) < items.len() {
                items.remove(json["id"].as_u64().unwrap() as usize);
            }
        }
        assert_eq!(state.todo_lists["default"].len(), 2);
    }

    #[test]
    fn test_handle_reorder() {
        let mut state = make_state();
        let items = state.todo_lists.get_mut("default").unwrap();
        assert_eq!(items.len(), 2);
        let item = items.remove(0);
        items.insert(1, item);
        assert_eq!(items[0], "beta");
        assert_eq!(items[1], "alpha");
    }

    #[test]
    fn test_handle_edit() {
        let mut state = make_state();
        let items = state.todo_lists.get_mut("default").unwrap();
        items[0] = "edited".to_string();
        assert_eq!(items[0], "edited");
    }

    #[test]
    fn test_load_save() {
        let tmp = std::env::temp_dir().join("test_todo_lists.json");
        let mut lists = HashMap::new();
        lists.insert("default".to_string(), vec!["one".to_string(), "two".to_string()]);
        if let Ok(json) = serde_json::to_string_pretty(&lists) {
            let _ = std::fs::write(&tmp, &json);
        }
        let loaded: HashMap<String, Vec<String>> = std::fs::read_to_string(&tmp)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded["default"][0], "one");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_load_missing_file() {
        // clean up any leftover file from other tests
        let _ = std::fs::remove_file(TODO_FILE);
        let lists = load_todos();
        assert!(lists.contains_key("default"));
        assert!(lists["default"].is_empty());
        let _ = std::fs::remove_file(TODO_FILE);
    }

    #[test]
    fn test_create_list() {
        let mut state = make_state();
        let name = "projects".to_string();
        state.todo_lists.insert(name.clone(), Vec::new());
        assert!(state.todo_lists.contains_key("projects"));
    }

    #[test]
    fn test_delete_list() {
        let mut state = make_state();
        state.todo_lists.insert("temp".to_string(), vec![]);
        state.todo_lists.remove("temp");
        assert!(!state.todo_lists.contains_key("temp"));
    }

    #[test]
    fn test_rename_list() {
        let mut state = make_state();
        let items = state.todo_lists.remove("default").unwrap();
        state.todo_lists.insert("renamed".to_string(), items);
        assert!(!state.todo_lists.contains_key("default"));
        assert!(state.todo_lists.contains_key("renamed"));
        assert_eq!(state.todo_lists["renamed"].len(), 2);
    }
}
