use axum::Router;
use tower_http::services::ServeFile;
use tokio::net::TcpListener;
use std::{net::IpAddr,net::Ipv4Addr, net::SocketAddr};
use axum::routing::get_service;

// todo
// * handle errors (no unwrap/expect)

//// Struct for the REST API response
//#[derive(Serialize)]
//struct ApiResponse {
//    message: String,
//}
//
//enum ServerState {
//    Connected,
//    Disconnected,
//}
//
//struct Status {
//    state: ServerState,
//    nbr_of_calls: u32
//}
//
//#[get("/api")]
//fn api(state: & State<Status>) -> Json<ApiResponse> {
//    //state.nbr_of_calls = state.nbr_of_calls +1;
//    Json(ApiResponse {
//        message: format!("Hello from Rocket REST API! - {}", state.nbr_of_calls).to_string(),
//    })
//}
//
//#[get("/")]
//async fn root() -> NamedFile {
//    index().await
//}
//
//#[get("/index.html")]
//async fn index() -> Response {
//    Response
//    Html("src/index.html").await.unwrap()
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
//
//// Task that broadcasts a message every 10 seconds
//async fn broadcast_task(tx: Sender<String>) {
//    loop {
//        sleep(Duration::from_secs(10)).await;
//        let message = "Server broadcast: Hello to all WebSocket clients!".to_string();
//        println!("Broadcasting message: {}", message);
//
//        if let Err(_) = tx.send(message) {
//            println!("No active WebSocket clients to send the message to.");
//        }
//    }
//}

#[tokio::main]
async fn main() {
    // let state = Status{nbr_of_calls: 0, state: ServerState::Disconnected};
    // let (tx, _) = channel::<String>(10);

    // let tx_clone = tx.clone();
    // tokio::spawn(async move {
    //     broadcast_task(tx_clone).await;
    // });
    // let figment = rocket::Config::figment();

    // rocket::custom(figment)
    //     .manage(tx)
    //     .manage(state)
    //     .mount("/", routes![api, index, root, ws_handler])
    
    
    let addr =  SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("started server on http://{}", addr);

    let router = Router::new()
        .route("/", get_service(ServeFile::new("src/index.html")));
    
    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
