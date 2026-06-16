use crate::services::types::Status;
use axum::extract::ws::WebSocket;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MESSAGE_TYPES: &[&str] = &["sync_push", "sync_pull"];

const LOCAL_DIR: &str = "/home/koewa/git/remoteio/huis/";
const REMOTE: &str = "koewa@koewa-precision-5530.local";
const REMOTE_DIR: &str = "~/Documents/huis/";

pub async fn handle_message(json: &Value, state: &Arc<Mutex<Status>>, _socket: &mut WebSocket) -> bool {
    initiate_sync(json, state).await
}

pub(crate) async fn initiate_sync(json: &Value, state: &Arc<Mutex<Status>>) -> bool {
    let direction = match json["type"].as_str() {
        Some("sync_push") => "push",
        Some("sync_pull") => "pull",
        _ => return false,
    };

    {
        let mut s = state.lock().await;
        if s.syncing.is_some() {
            return false;
        }
        s.syncing = Some(direction.to_string());
        let _ = s.tx.send(
            serde_json::json!({"type":"sync_status","direction":direction,"status":"starting"}).to_string(),
        );
    }

    let state_clone = state.clone();
    let dir = direction.to_string();
    tokio::spawn(async move {
        let result = run_sync(&dir).await;
        let mut s = state_clone.lock().await;
        s.syncing = None;
        let msg = match result {
            Ok(summary) => serde_json::json!({
                "type": "sync_result",
                "direction": dir,
                "status": "ok",
                "summary": summary,
            }),
            Err(e) => serde_json::json!({
                "type": "sync_result",
                "direction": dir,
                "status": "error",
                "message": e,
            }),
        };
        let _ = s.tx.send(msg.to_string());
    });

    false
}

async fn run_sync(direction: &str) -> Result<String, String> {
    let remote_path = format!("{}:{}", REMOTE, REMOTE_DIR);
    let (src, dst) = match direction {
        "push" => (LOCAL_DIR.to_string(), remote_path),
        "pull" => (remote_path, LOCAL_DIR.to_string()),
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
    use std::time::Duration;
    use tokio::sync::broadcast::channel;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn test_sync_push_broadcasts_status() {
        let (tx, mut rx) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todos: vec![],
            syncing: None,
        }));

        let json = serde_json::json!({"type":"sync_push"});
        let result = initiate_sync(&json, &state).await;
        assert!(!result); // false = socket stays open

        // sync_status should be broadcast
        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout waiting for sync_status")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "sync_status");
        assert_eq!(v["direction"], "push");
        assert_eq!(v["status"], "starting");

        // syncing should be set
        {
            let s = state.lock().await;
            assert_eq!(s.syncing.as_deref(), Some("push"));
        }
    }

    #[tokio::test]
    async fn test_sync_pull_broadcasts_status() {
        let (tx, mut rx) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todos: vec![],
            syncing: None,
        }));

        let json = serde_json::json!({"type":"sync_pull"});
        let result = initiate_sync(&json, &state).await;
        assert!(!result);

        let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timeout")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "sync_status");
        assert_eq!(v["direction"], "pull");

        {
            let s = state.lock().await;
            assert_eq!(s.syncing.as_deref(), Some("pull"));
        }
    }

    #[tokio::test]
    async fn test_ignore_concurrent_sync() {
        let (tx, _) = channel::<String>(16);
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx,
            shutdown: Arc::new(Notify::new()),
            todos: vec![],
            syncing: Some("push".to_string()),
        }));

        let json = serde_json::json!({"type":"sync_push"});
        let result = initiate_sync(&json, &state).await;
        assert!(!result); // ignored, not crashed

        let s = state.lock().await;
        assert_eq!(s.syncing.as_deref(), Some("push"));
    }
}
