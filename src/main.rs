use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, get_service, Router},
};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast::channel;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tower_http::services::ServeDir;
use clap::Parser;

mod services;
use services::types::{ServerState, Status};

async fn websocket_handler(
    State(state): State<Arc<Mutex<Status>>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<Status>>) {
    state.lock().await.state = ServerState::Connected;
    let mut rx = state.lock().await.tx.subscribe();

    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(msg) => {
                        println!("Received message: {}", msg);
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg) {
                            if let Some(type_str) = json["type"].as_str() {
                                if services::dispatch(type_str, &json, &state, &mut socket).await {
                                    break;
                                }
                            }
                        }
                    }
                    Message::Close(_) => {
                        println!("Closing WebSocket connection.");
                        state.lock().await.state = ServerState::Disconnected;
                        break;
                    }
                    _ => {}
                }
            }
            // forward the internal channel to the websocket
            Ok(msg) = rx.recv() => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn root_handler(State(state): State<Arc<Mutex<Status>>>) -> String {
    let state_locked = state.lock().await;
    let count: usize = state_locked.todo_lists.values().map(|v| v.len()).sum();
    format!(
        "state: {:?} \n todo items: {}",
        state_locked.state,
        count,
    )
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (tx, _) = channel::<String>(64);
    let shutdown = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(Status {
        state: ServerState::Disconnected,
        tx: tx.clone(),
        shutdown: shutdown.clone(),
        todo_lists: services::load_todos(),
        syncing: HashSet::new(),
    }));

    services::setup_process_monitor(tx.clone());

    let ip: IpAddr = args.ip.parse().expect("Invalid IP address");
    let addr = SocketAddr::new(ip, args.port);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("started server on http://{}", addr);

    let router = Router::new()
        .route("/api", get(root_handler))
        .route("/ws", get(websocket_handler))
        .fallback(get_service(ServeDir::new("client/dist")))
        .with_state(state);

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
            println!("Shutting down gracefully...");
        })
        .await
        .unwrap();
}

#[cfg(test)]
mod integration {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    async fn spawn_server() -> SocketAddr {
        let (tx, _) = channel::<String>(64);
        let shutdown = Arc::new(Notify::new());
        let state = Arc::new(Mutex::new(Status {
            state: ServerState::Disconnected,
            tx: tx.clone(),
            shutdown: shutdown.clone(),
            todo_lists: HashMap::from([("default".to_string(), vec!["test todo".to_string()])]),
            syncing: HashSet::new(),
        }));
        services::setup_process_monitor(tx.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let router = Router::new()
            .route("/api", get(root_handler))
            .route("/ws", get(websocket_handler))
            .fallback(get_service(ServeDir::new("client/dist")))
            .with_state(state);

        tokio::spawn(async move {
            axum::serve(listener, router.into_make_service())
                .await
                .unwrap();
        });

        addr
    }

    #[tokio::test]
    async fn test_api_returns_status() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let request = format!("GET /api HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", addr);
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("todo items: 1"));
        assert!(response.contains("Disconnected"));
    }

    #[tokio::test]
    async fn test_todo_list_after_connect() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};
        let ws_addr = format!("ws://{}/ws", addr);
        let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
        let (mut write, mut read) = ws.split();

        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_list"}"#.into(),
        )).await.unwrap();

        let msg = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["lists"]["default"][0], "test todo");
    }

    #[tokio::test]
    async fn test_todo_add_and_list() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};
        let ws_addr = format!("ws://{}/ws", addr);
        let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
        let (mut write, mut read) = ws.split();

        // request initial list
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_list"}"#.into(),
        )).await.unwrap();
        let _ = read.next().await.unwrap().unwrap();

        // add a new todo
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_add","text":"added via ws"}"#.into(),
        )).await.unwrap();

        // the add broadcasts the updated list — consume that message
        let broadcast = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(broadcast.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["lists"]["default"].as_array().unwrap().len(), 2);
        assert_eq!(v["lists"]["default"][1], "added via ws");
    }

    #[tokio::test]
    async fn test_todo_remove() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};
        let ws_addr = format!("ws://{}/ws", addr);
        let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
        let (mut write, mut read) = ws.split();

        // add two items
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_add","text":"first"}"#.into(),
        )).await.unwrap();
        let _ = read.next().await.unwrap().unwrap(); // broadcast after add

        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_add","text":"second"}"#.into(),
        )).await.unwrap();
        let _ = read.next().await.unwrap().unwrap(); // broadcast after add

        // remove the first item (index 0 — the initial "test todo")
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_remove","id":0}"#.into(),
        )).await.unwrap();

        let broadcast = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(broadcast.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
        let items = v["lists"]["default"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "first");
        assert_eq!(items[1], "second");
    }

    #[tokio::test]
    async fn test_todo_reorder_and_edit() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};
        let ws_addr = format!("ws://{}/ws", addr);
        let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
        let (mut write, mut read) = ws.split();

        // add two items (current: ["test todo", "first", "second"])
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_add","text":"first"}"#.into(),
        )).await.unwrap();
        let _ = read.next().await.unwrap().unwrap();

        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_add","text":"second"}"#.into(),
        )).await.unwrap();
        let _ = read.next().await.unwrap().unwrap();

        // reorder: move "second" (index 2) before "first" (index 1)
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_reorder","from":2,"to":1}"#.into(),
        )).await.unwrap();

        let broadcast = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(broadcast.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["lists"]["default"][1], "second");
        assert_eq!(v["lists"]["default"][2], "first");

        // edit index 1 ("second") to "edited"
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_edit","id":1,"text":"edited"}"#.into(),
        )).await.unwrap();

        let broadcast = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(broadcast.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
        assert_eq!(v["lists"]["default"][1], "edited");
    }

    #[tokio::test]
    async fn test_todo_persistence_across_reconnect() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};

        // --- first connection: add a todo ---
        {
            let ws_addr = format!("ws://{}/ws", addr);
            let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
            let (mut write, mut read) = ws.split();

            write.send(tokio_tungstenite::tungstenite::Message::Text(
                r#"{"type":"todo_add","text":"persistent item"}"#.into(),
            )).await.unwrap();
            let _ = read.next().await.unwrap().unwrap(); // broadcast
        }

        // --- second connection: verify it's still there ---
        {
            let ws_addr = format!("ws://{}/ws", addr);
            let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
            let (mut write, mut read) = ws.split();

            write.send(tokio_tungstenite::tungstenite::Message::Text(
                r#"{"type":"todo_list"}"#.into(),
            )).await.unwrap();

            let msg = read.next().await.unwrap().unwrap();
            let v: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(v["type"], "todo_list");
            let items: Vec<String> = v["lists"]["default"]
                .as_array().unwrap()
                .iter().map(|x| x.as_str().unwrap().to_string())
                .collect();
            assert!(items.contains(&"persistent item".to_string()), "persisted item should survive reconnect");
        }
    }

    #[tokio::test]
    async fn test_unknown_type_ignored() {
        let addr = spawn_server().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        use futures_util::{SinkExt, StreamExt};
        let ws_addr = format!("ws://{}/ws", addr);
        let (ws, _) = tokio_tungstenite::connect_async(&ws_addr).await.unwrap();
        let (mut write, mut read) = ws.split();

        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"unknown","foo":"bar"}"#.into(),
        )).await.unwrap();

        // nothing should be sent back for unknown types
        tokio::time::sleep(Duration::from_millis(200)).await;
        // Verify the connection is still alive by requesting todo_list
        write.send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"todo_list"}"#.into(),
        )).await.unwrap();

        let msg = read.next().await.unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(v["type"], "todo_list");
    }
}
