use async_std::task;
use axum::{
    response::IntoResponse,
    routing::{
        get,
        Router,
        get_service},
    extract::{
        ws::{
            Message,
            WebSocketUpgrade,
            WebSocket
        },
        State
    },

};
use std::net::{IpAddr,Ipv4Addr,SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tower_http::services::ServeFile;
use tokio::net::TcpListener;
use tokio::sync::broadcast::{channel, Sender};
use tokio::sync::Mutex;

// todo
// * handle errors (no unwrap/expect)
// * return json messages instead of strings
// * move code into modules

#[derive(Debug)]
enum ServerState {
    Connected,
    Disconnected,
}

struct Status {
    state: ServerState,
    nbr_of_calls: u32,
    tx: Sender<String>,
}

//// Task that broadcasts a message every 10 seconds
async fn broadcast_task(tx: Sender<String>, name: &str) {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        let message = format!("Server broadcast: Hello from {} to all WebSocket clients!", name).to_string();
        println!("Broadcasting message: {}, from {}", message, name);

        if let Err(_) = tx.send(message) {
            println!("No active WebSocket clients to send the message to ({}).", name);
        }
    }
}

async fn websocket_handler(State(state): State<Arc<Mutex<Status>>>, ws: WebSocketUpgrade,) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<Status>>) {
    // Send a greeting message to the client
    if let Err(e) = socket.send(Message::Text("Hello from the server!".into())).await {
        eprintln!("Error sending message: {}", e);
        return;
    }

    state.lock().await.state = ServerState::Connected;
    // tx.subscribe gives a Reveiver
    let rx = & mut state.lock().await.tx.subscribe();
    // Loop to keep the connection alive
    loop {
        // trigger when something is received from the ws or when something needs to be send to the
        // ws from an internal service (using the channel)
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(msg) => {
                        println!("Received message: {}", msg);
                        {
                            let mut state_locked = state.lock().await;
                            state_locked.nbr_of_calls = state_locked.nbr_of_calls + 1;
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
    format!("Number of calls: {} \n state: {:?}", count, state.lock().await.state)
}

#[tokio::main]
async fn main() {
    let (tx, _) = channel::<String>(10);
    let state = Arc::new(Mutex::new(Status{nbr_of_calls: 0, state: ServerState::Disconnected, tx: tx.clone()}));

    let tx_clone = tx.clone();
    tokio::spawn(async {
        broadcast_task(tx_clone, "first").await;
    });
    tokio::spawn(async {
        broadcast_task(tx, "second").await;
    });
    
    let addr =  SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("started server on http://{}", addr);

    let router = Router::new()
        .route("/", get_service(ServeFile::new("src/index.html")))
        .route("/api", get(root_handler))
        .route("/ws", get(websocket_handler))
        .with_state(state);

    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
