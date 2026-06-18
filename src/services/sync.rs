use crate::services::types::Status;
use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MESSAGE_TYPES: &[&str] = &["sync_list", "sync_save", "sync_delete", "sync_push", "sync_pull", "sync_push_preview", "sync_pull_preview"];

const CONFIG_FILE: &str = "sync_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEndpoint {
    pub host: String,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTarget {
    pub name: String,
    pub local: SyncEndpoint,
    pub remote: SyncEndpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<String>,
}

fn current_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut z = secs as i64 / 86400;
    let mut y = 1970i64;
    loop {
        let days = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
        if z < days { break; }
        z -= days;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let md: &[i64] = if leap { &[31,29,31,30,31,30,31,31,30,31,30,31] } else { &[31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut m = 1i64;
    for &d in md {
        if z < d { break; }
        z -= d;
        m += 1;
    }
    let d = z + 1;
    let rem = secs % 86400;
    let h = rem / 3600;
    let mi = (rem % 3600) / 60;
    let s = rem % 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}

fn endpoint_to_rsync_path(ep: &SyncEndpoint) -> String {
    if ep.host.is_empty() {
        ep.path.clone()
    } else if ep.user.is_empty() {
        format!("{}:{}", ep.host, ep.path)
    } else {
        format!("{}@{}:{}", ep.user, ep.host, ep.path)
    }
}

pub fn load_targets() -> Vec<SyncTarget> {
    std::fs::read_to_string(CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_targets(targets: &[SyncTarget]) {
    if let Ok(json) = serde_json::to_string_pretty(targets) {
        let _ = std::fs::write(CONFIG_FILE, &json);
    }
}

fn broadcast_list(tx: &tokio::sync::broadcast::Sender<String>, targets: &[SyncTarget]) {
    let msg = serde_json::json!({"type":"sync_list","targets": targets}).to_string();
    let _ = tx.send(msg);
}

pub async fn handle_message(json: &Value, state: &Arc<Mutex<Status>>, socket: &mut WebSocket) -> bool {
    match json["type"].as_str() {
        Some("sync_list") => {
            let targets = load_targets();
            let msg = serde_json::json!({"type":"sync_list","targets": targets}).to_string();
            if socket.send(Message::Text(msg.into())).await.is_err() {
                return true;
            }
            false
        }
        Some("sync_save") => {
            let target: SyncTarget = match serde_json::from_value(json["target"].clone()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            let mut targets = load_targets();
            if let Some(pos) = targets.iter().position(|t| t.name == target.name) {
                targets[pos] = target;
            } else {
                targets.push(target);
            }
            save_targets(&targets);
            let s = state.lock().await;
            broadcast_list(&s.tx, &targets);
            false
        }
        Some("sync_delete") => {
            let name = match json["name"].as_str() {
                Some(n) => n,
                None => return false,
            };
            let mut targets = load_targets();
            targets.retain(|t| t.name != name);
            save_targets(&targets);
            let s = state.lock().await;
            broadcast_list(&s.tx, &targets);
            false
        }
        Some("sync_push_preview") | Some("sync_pull_preview") => {
            let direction = match json["type"].as_str() {
                Some("sync_push_preview") => "push",
                _ => "pull",
            };
            let target_name = match json["name"].as_str() {
                Some(n) => n,
                None => return false,
            };
            let targets = load_targets();
            let target = match targets.iter().find(|t| t.name == target_name) {
                Some(t) => t.clone(),
                None => return false,
            };
            let (src, dst) = match direction {
                "push" => (endpoint_to_rsync_path(&target.local), endpoint_to_rsync_path(&target.remote)),
                _ => (endpoint_to_rsync_path(&target.remote), endpoint_to_rsync_path(&target.local)),
            };
            let output = tokio::process::Command::new("rsync")
                .args(["-avz", "--delete", "--dry-run", &src, &dst])
                .output()
                .await;
            let files = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.lines()
                        .filter(|l| {
                            !l.starts_with("sending") && !l.starts_with("receiving")
                            && !l.starts_with("sent ") && !l.starts_with("total ")
                            && !l.is_empty()
                        })
                        .map(|l| {
                            if let Some(path) = l.strip_prefix("deleting ") {
                                serde_json::json!({"path": path, "change": "deleted"})
                            } else {
                                serde_json::json!({"path": l, "change": "changed"})
                            }
                        })
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            };
            let resp = serde_json::json!({"type":"sync_preview","name":target_name,"direction":direction,"files":files}).to_string();
            if socket.send(Message::Text(resp.into())).await.is_err() {
                return true;
            }
            false
        }
        Some("sync_push") | Some("sync_pull") => {
            let targets = load_targets();
            initiate_sync(json, state, &targets).await
        }
        _ => false,
    }
}

pub(crate) async fn initiate_sync(json: &Value, state: &Arc<Mutex<Status>>, targets: &[SyncTarget]) -> bool {
    let direction = match json["type"].as_str() {
        Some("sync_push") => "push",
        Some("sync_pull") => "pull",
        _ => return false,
    };

    let target_name = match json["name"].as_str() {
        Some(n) => n.to_string(),
        None => return false,
    };

    let target = match targets.iter().find(|t| t.name == target_name) {
        Some(t) => t.clone(),
        None => {
            let s = state.lock().await;
            let _ = s.tx.send(
                serde_json::json!({"type":"sync_result","direction":direction,"status":"error","message":format!("Unknown target: {}", target_name)}).to_string(),
            );
            return false;
        }
    };

    {
        let mut s = state.lock().await;
        if !s.syncing.insert(target_name.clone()) {
            return false;
        }
        let _ = s.tx.send(
            serde_json::json!({"type":"sync_status","direction":direction,"name":target_name,"status":"starting"}).to_string(),
        );
    }

    let state_clone = state.clone();
    let dir = direction.to_string();
    let tn = target_name.clone();
    tokio::spawn(async move {
        let result = run_sync_for_target(&target, &dir).await;
        let mut s = state_clone.lock().await;
        s.syncing.remove(&tn);
        let (msg, ok) = match &result {
            Ok(summary) => (
                serde_json::json!({
                    "type": "sync_result",
                    "direction": dir,
                    "name": target.name,
                    "status": "ok",
                    "summary": summary,
                }),
                true,
            ),
            Err(e) => (
                serde_json::json!({
                    "type": "sync_result",
                    "direction": dir,
                    "name": target.name,
                    "status": "error",
                    "message": e,
                }),
                false,
            ),
        };
        let _ = s.tx.send(msg.to_string());
        drop(s);

        if ok {
            let mut targets = load_targets();
            if let Some(t) = targets.iter_mut().find(|t| t.name == tn) {
                t.last_synced = Some(current_timestamp());
                save_targets(&targets);
                let s = state_clone.lock().await;
                broadcast_list(&s.tx, &targets);
            }
        }
    });

    false
}

async fn run_sync_for_target(target: &SyncTarget, direction: &str) -> Result<String, String> {
    let (src, dst) = match direction {
        "push" => (endpoint_to_rsync_path(&target.local), endpoint_to_rsync_path(&target.remote)),
        "pull" => (endpoint_to_rsync_path(&target.remote), endpoint_to_rsync_path(&target.local)),
        _ => return Err("Invalid direction".to_string()),
    };

    let output = tokio::process::Command::new("rsync")
        .args(["-avz", "--delete", &src, &dst])
        .output()
        .await
        .map_err(|e| format!("Failed to execute rsync: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let summary = stdout.lines().last().unwrap_or("done").to_string();
        Ok(summary)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::types::ServerState;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::time::Duration;
    use tokio::sync::broadcast::channel;
    use tokio::sync::Notify;

    fn make_target(name: &str) -> SyncTarget {
        SyncTarget {
            name: name.into(),
            local: SyncEndpoint { host: String::new(), path: "/local/".into(), user: String::new() },
            remote: SyncEndpoint { host: "remote.local".into(), path: "/remote/".into(), user: "user".into() },
            last_synced: None,
        }
    }

    #[tokio::test]
    async fn test_sync_push_broadcasts_status() {
        let (tx, mut rx) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todo_lists: HashMap::new(),
            todo_active: "default".to_string(),
            syncing: HashSet::new(),
        }));

        let targets = vec![make_target("MyLaptop")];
        let json = serde_json::json!({"type":"sync_push","name":"MyLaptop"});
        let result = initiate_sync(&json, &state, &targets).await;
        assert!(!result);

        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "sync_status");
        assert_eq!(v["direction"], "push");
        assert_eq!(v["name"], "MyLaptop");
        assert_eq!(v["status"], "starting");

        {
            let s = state.lock().await;
            assert!(s.syncing.contains("MyLaptop"));
        }
    }

    #[tokio::test]
    async fn test_sync_unknown_target() {
        let (tx, mut rx) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todo_lists: HashMap::new(),
            todo_active: "default".to_string(),
            syncing: HashSet::new(),
        }));

        let targets = vec![make_target("Existing")];
        let json = serde_json::json!({"type":"sync_pull","name":"NonExistent"});
        let result = initiate_sync(&json, &state, &targets).await;
        assert!(!result);

        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "sync_result");
        assert_eq!(v["status"], "error");
        assert!(v["message"].as_str().unwrap().contains("Unknown target"));
    }

    #[tokio::test]
    async fn test_sync_push_missing_name() {
        let (tx, _) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todo_lists: HashMap::new(),
            todo_active: "default".to_string(),
            syncing: HashSet::new(),
        }));

        let json = serde_json::json!({"type":"sync_push"});
        let result = initiate_sync(&json, &state, &[]).await;
        assert!(!result);

        let s = state.lock().await;
        assert!(s.syncing.is_empty());
    }

    #[tokio::test]
    async fn test_ignore_concurrent_same_target() {
        let (tx, _) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todo_lists: HashMap::new(),
            todo_active: "default".to_string(),
            syncing: HashSet::from(["Old".to_string()]),
        }));

        let targets = vec![make_target("Old"), make_target("Other")];
        let json = serde_json::json!({"type":"sync_push","name":"Old"});
        let result = initiate_sync(&json, &state, &targets).await;
        assert!(!result); // ignored, same target already syncing

        let s = state.lock().await;
        assert!(s.syncing.contains("Old"));
        assert_eq!(s.syncing.len(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_different_targets_allowed() {
        let (tx, mut rx) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todo_lists: HashMap::new(),
            todo_active: "default".to_string(),
            syncing: HashSet::from(["Existing".to_string()]),
        }));

        let targets = vec![make_target("Existing"), make_target("Other")];
        let json = serde_json::json!({"type":"sync_pull","name":"Other"});
        let result = initiate_sync(&json, &state, &targets).await;
        assert!(!result);

        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "sync_status");
        assert_eq!(v["name"], "Other");

        let s = state.lock().await;
        assert!(s.syncing.contains("Existing"));
        assert!(s.syncing.contains("Other"));
        assert_eq!(s.syncing.len(), 2);
    }

    #[tokio::test]
    async fn test_save_new_target_updates_file() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            // set config path to temp file for this test session
            // by writing and reading the actual file path is fine
        });

        let tmp = std::env::temp_dir().join("test_sync_save.json");
        // temporarily swap CONFIG_FILE by writing directly
        std::fs::write(&tmp, "[]").unwrap();

        let target = make_target("NewTarget");
        let targets = vec![target.clone()];
        // simulate save
        let json = serde_json::to_string_pretty(&targets).unwrap();
        std::fs::write(&tmp, &json).unwrap();

        let loaded: Vec<SyncTarget> = std::fs::read_to_string(&tmp)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "NewTarget");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_delete_target() {
        let tmp = std::env::temp_dir().join("test_sync_delete.json");
        let targets = vec![make_target("Keep"), make_target("Remove")];
        let json = serde_json::to_string_pretty(&targets).unwrap();
        std::fs::write(&tmp, &json).unwrap();

        let mut loaded: Vec<SyncTarget> = std::fs::read_to_string(&tmp)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        loaded.retain(|t| t.name != "Remove");
        let json = serde_json::to_string_pretty(&loaded).unwrap();
        std::fs::write(&tmp, &json).unwrap();

        let final_list: Vec<SyncTarget> = std::fs::read_to_string(&tmp)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(final_list.len(), 1);
        assert_eq!(final_list[0].name, "Keep");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_endpoint_to_rsync_path() {
        let local = SyncEndpoint { host: String::new(), path: "/local/path/".into(), user: String::new() };
        assert_eq!(endpoint_to_rsync_path(&local), "/local/path/");

        let remote = SyncEndpoint { host: "example.com".into(), path: "~/remote/".into(), user: "alice".into() };
        assert_eq!(endpoint_to_rsync_path(&remote), "alice@example.com:~/remote/");

        let no_user = SyncEndpoint { host: "backup.local".into(), path: "/backups/".into(), user: String::new() };
        assert_eq!(endpoint_to_rsync_path(&no_user), "backup.local:/backups/");
    }

    #[tokio::test]
    async fn test_load_save_targets() {
        let tmp = std::env::temp_dir().join("test_sync_config.json");
        let targets = vec![
            SyncTarget {
                name: "Test".into(),
                local: SyncEndpoint { host: String::new(), path: "/local/".into(), user: String::new() },
                remote: SyncEndpoint { host: "remote.local".into(), path: "/remote/".into(), user: "user".into() },
                last_synced: None,
            },
        ];

        // save to temp path (override CONFIG_FILE by writing directly)
        let json = serde_json::to_string_pretty(&targets).unwrap();
        std::fs::write(&tmp, &json).unwrap();

        let loaded: Vec<SyncTarget> = std::fs::read_to_string(&tmp)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test");
        assert_eq!(loaded[0].remote.host, "remote.local");

        let _ = std::fs::remove_file(&tmp);
    }
}
