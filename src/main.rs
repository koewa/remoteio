#[macro_use] extern crate rocket;

use rocket::serde::{Serialize, json::Json};
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::tokio::{self, time::sleep};
use rocket::response::Redirect;
use rocket::fs::NamedFile;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;
use tokio::net::TcpListener;
use futures_util::{StreamExt, SinkExt};
use std::time::Duration;


// Struct for the REST API response
#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

#[get("/api")]
fn api() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "Hello from Rocket REST API!".to_string(),
    })
}

#[get("/<path>")]
async fn index(path: &str) -> NamedFile {
    NamedFile::open("src/index").await.unwrap()
}

// WebSocket handler (placeholder route)
#[get("/ws")]
fn ws_handler() -> Redirect {
    // Redirect to WebSocket server running on port 9001
    Redirect::temporary("ws://127.0.0.1:9001/ws")
}

// WebSocket TCP server using Tokio Tungstenite
async fn websocket_server(tx: Sender<String>) {
    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind address");

    println!("WebSocket server running on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("Failed to get peer address");
        println!("Incoming TCP connection from: {}", peer);

        tokio::spawn(handle_websocket(stream, tx.clone()));
    }
}

async fn handle_websocket( stream: tokio::net::TcpStream, tx: Sender<String>) {
    let ws_stream = accept_async(stream).await.expect("Error during WebSocket handshake");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            // Handle incoming messages from the client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        println!("Received message from client: {}", text);
                        // Echo the message back
                        ws_sender.send(Message::Text("message received by server: ".to_owned() + &text)).await.expect("Failed to send message");
                    }
                    Some(Ok(Message::Ping(ping))) => {
                        ws_sender.send(Message::Pong(ping)).await.expect("Failed to send pong");
                    }
                    Some(Ok(Message::Close(reason))) => {
                        println!("Client disconnected: {:?}", reason);
                        break;
                    }
                    _ => break,
                }
            },

            // Broadcast messages to clients
            msg = rx.recv() => {
                if let Ok(notification) = msg {
                    ws_sender.send(Message::Text(notification)).await.expect("Failed to send message");
                }
            }
        }
    }
}

// Task that broadcasts a message every 10 seconds
async fn broadcast_task(tx: Sender<String>) {
    loop {
        sleep(Duration::from_secs(10)).await;
        let message = "Server broadcast: Hello to all WebSocket clients!".to_string();
        println!("Broadcasting message: {}", message);

        // Send the message to all WebSocket clients
        if let Err(_) = tx.send(message) {
            println!("No active WebSocket clients to send the message to.");
        }
    }
}

#[rocket::launch]
async fn rocket() -> _ {
    // Create a broadcast channel for notifications
    let (tx, _) = channel::<String>(10);

    // Spawn a WebSocket server in the background
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        websocket_server(tx_clone).await;
    });

    // Spawn the periodic broadcast task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        broadcast_task(tx_clone).await;
    });

    // Rocket instance
    rocket::build()
        .mount("/", routes![api, index, ws_handler])
}
