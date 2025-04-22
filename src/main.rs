#[macro_use] extern crate rocket;

use rocket::serde::{Serialize, json::Json};
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::tokio::{self, time::{sleep, Duration}};
use rocket::fs::NamedFile;
use rocket::State;

// todo
// * handle errors (no unwrap/expect)
// * make single threaded

// Struct for the REST API response
#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

enum ServerState {
    Connected,
    Disconnected,
}

struct Status {
    state: ServerState,
    nbr_of_calls: u32
}

#[get("/api")]
fn api(state: & State<Status>) -> Json<ApiResponse> {
    //state.nbr_of_calls = state.nbr_of_calls +1;
    Json(ApiResponse {
        message: format!("Hello from Rocket REST API! - {}", state.nbr_of_calls).to_string(),
    })
}

#[get("/index.html")]
async fn index() -> NamedFile {
    NamedFile::open("src/index.html").await.unwrap()
}

#[get("/ws")]
fn ws_handler(ws: ws::WebSocket, state: & State<Status>, sender: & State<Sender<String>>) -> ws::Channel<'static> {
    println!("Web socket is opened");
    use rocket::futures::{SinkExt, StreamExt};
    let mut rx = sender.subscribe();
    let nbr_of_calls = state.nbr_of_calls;
    ws.channel(move |mut stream| Box::pin(async move {
        loop {
            tokio::select! {
                Some(message) = stream.next() => {
                    let message_done = message.expect("something wrong with message");
                    println!("Received from client: {}", message_done);
                    let result = format!("server received: {message_done} {nbr_of_calls}");
                    let _ = stream.send(ws::Message::Text(result)).await;
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

        if let Err(_) = tx.send(message) {
            println!("No active WebSocket clients to send the message to.");
        }
    }
}

#[rocket::launch]
//#[tokio::main(flavor = "current_thread")]
async fn rocket() -> _ {
    let state = Status{nbr_of_calls: 0, state: ServerState::Disconnected};
    let (tx, _) = channel::<String>(10);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        broadcast_task(tx_clone).await;
    });
    let figment = rocket::Config::figment();
        //.merge(("workers", 1));

    rocket::custom(figment)
        .manage(tx)
        .manage(state)
        .mount("/", routes![api, index, ws_handler])
}
