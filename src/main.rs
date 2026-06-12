use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, get_service, Router},
};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::broadcast::{channel, Sender};
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tower_http::services::ServeFile;
use clap::Parser;

mod service;

// todo
// * handle errors (no unwrap/expect)
// * return json messages instead of strings
// * move code into modules
// * forward messages received from ws to service
// * clean shutdown (cancel tasks)

#[derive(Debug)]
enum ServerState {
    Connected,
    Disconnected,
}

struct Status {
    state: ServerState,
    nbr_of_calls: u32,
    tx: Sender<String>,
    shutdown: Arc<Notify>,
}

async fn websocket_handler(
    State(state): State<Arc<Mutex<Status>>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<Status>>) {
    if let Err(e) = socket
        .send(Message::Text("Hello from the server!".into()))
        .await
    {
        eprintln!("Error sending message: {}", e);
        return;
    }

    state.lock().await.state = ServerState::Connected;
    let mut rx = state.lock().await.tx.subscribe();

    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(msg) => {
                        println!("Received message: {}", msg);
                        if msg == "shutdown" {
                            let _ = socket.send(Message::Text("Server shutting down...".into())).await;
                            let _ = state.lock().await.tx.send("Server shutting down...".to_string());
                            state.lock().await.shutdown.notify_one();
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            break;
                        }
                        {
                            let mut state_locked = state.lock().await;
                            state_locked.nbr_of_calls += 1;
                        }
                        if let Err(e) = socket.send(Message::Text(format!("Echo: {}", msg).into())).await {
                            eprintln!("Error sending message: {}", e);
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
            Ok(msg) = rx.recv() => {
                println!("Sending to client: {}", msg);
                socket.send(Message::Text(msg.into())).await.expect("Failed to send message");
            }
        }
    }
}

async fn root_handler(State(state): State<Arc<Mutex<Status>>>) -> String {
    let count = state.lock().await.nbr_of_calls;
    format!(
        "Number of calls: {} \n state: {:?}",
        count,
        state.lock().await.state
    )
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// ip address to bind to
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    /// Port to listen to
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (tx, _) = channel::<String>(64);
    let shutdown = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(Status {
        nbr_of_calls: 0,
        state: ServerState::Disconnected,
        tx: tx.clone(),
        shutdown: shutdown.clone(),
    }));

    service::setup_process_monitor(tx.clone());

    let ip: IpAddr = args.ip.parse().expect("Invalid IP address");
    let addr = SocketAddr::new(ip, args.port);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("started server on http://{}", addr);

    let router = Router::new()
        .route("/", get_service(ServeFile::new("src/ui/index.html")))
        .route("/api", get(root_handler))
        .route("/ws", get(websocket_handler))
        .with_state(state);

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
            println!("Shutting down gracefully...");
        })
        .await
        .unwrap();
}
