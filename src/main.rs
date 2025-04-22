#[macro_use] extern crate rocket;

use rocket::serde::{Serialize, json::Json};
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::tokio::{self, time::{sleep, Duration}};
use rocket::fs::NamedFile;
use rocket::State;

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

#[get("/index.html")]
async fn index() -> NamedFile {
    NamedFile::open("src/index.html").await.unwrap()
}

#[get("/ws")]
fn ws_handler(ws: ws::WebSocket, state: & State<Sender<String>>) -> ws::Channel<'static> {
    println!("Web socket is opened");
    use rocket::futures::{SinkExt, StreamExt};
    let mut rx = state.subscribe();
    ws.channel(move |mut stream| Box::pin(async move {
        loop {
            tokio::select! {
                Some(message) = stream.next() => {
                    let message_done = message.expect("something wrong with message");
                    println!("Received from client: {}", message_done);
                    let _ = stream.send(message_done).await;
                }

                Ok(msg) = rx.recv() => {
                    println!("Sending to client: {}", msg);
                    stream.send(ws::Message::Text(msg)).await.expect("Failed to send message");
                }
            }
        }
    }))
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

    // Spawn the periodic broadcast task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        broadcast_task(tx_clone).await;
    });

    // Rocket instance
    rocket::build()
        .manage(tx)
        .mount("/", routes![api, index, ws_handler])
}
