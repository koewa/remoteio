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

//// Struct for the REST API response
//#[derive(Serialize)]
//struct ApiResponse {
//    message: String,
//}
//
enum ServerState {
    Connected,
    Disconnected,
}

struct Status {
    state: ServerState,
    nbr_of_calls: u32,
    rx: Sender<String>,
}

//#[get("/api")]
//fn api(state: & State<Status>) -> Json<ApiResponse> {
//    //state.nbr_of_calls = state.nbr_of_calls +1;
//    Json(ApiResponse {
//        message: format!("Hello from Rocket REST API! - {}", state.nbr_of_calls).to_string(),
//    })
//}
//
//#[get("/ws")]
//fn ws_handler(ws: ws::WebSocket, state: & State<Status>, sender: & State<Sender<String>>) -> ws::Channel<'static> {
//    println!("Web socket is opened");
//    state.state = ServerState::Connected;
//    use rocket::futures::{SinkExt, StreamExt};
//    let mut rx = sender.subscribe();
//    let nbr_of_calls = state.nbr_of_calls;
//    ws.channel(move |mut stream| Box::pin(async move {
//        loop {
//            tokio::select! {
//                Some(message) = stream.next() => {
//                    let message_done = message.expect("something wrong with message");
//                    println!("Received from client: {}", message_done);
//                    let result = format!("server received: {message_done} {nbr_of_calls}");
//                    let _ = stream.send(ws::Message::Text(result)).await;
//                }
//
//                Ok(msg) = rx.recv() => {
//                    println!("Sending to client: {}", msg);
//                    stream.send(ws::Message::Text(msg)).await.expect("Failed to send message");
//                }
//            }
//        }
//    }))
//}

//// Task that broadcasts a message every 10 seconds
async fn broadcast_task(tx: Sender<String>) {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        let message = "Server broadcast: Hello to all WebSocket clients!".to_string();
        println!("Broadcasting message: {}", message);

        if let Err(_) = tx.send(message) {
            println!("No active WebSocket clients to send the message to.");
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

    let mut rx = state.lock().await.rx.subscribe();
    // Loop to keep the connection alive
    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(msg) => {
                        println!("Received message: {}", msg);
                        let mut count = state.lock().await.nbr_of_calls;
                        count = count + 1;
                        state.lock().await.nbr_of_calls = count;
                        if let Err(e) = socket.send(Message::Text(format!("Echo: {}", msg).into())).await {
                            eprintln!("Error sending message: {}", e);
                        }
                    }
                    Message::Close(_) => {
                        println!("Closing WebSocket connection.");
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
    format!("Hello, world! {}", count)
}

#[tokio::main]
async fn main() {
    let (tx, _) = channel::<String>(10);
    let tx_clone = tx.clone();
    let state = Arc::new(Mutex::new(Status{nbr_of_calls: 0, state: ServerState::Disconnected, rx: tx}));

    tokio::spawn(async move {
        broadcast_task(tx_clone).await;
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
