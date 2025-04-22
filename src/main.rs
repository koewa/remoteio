#[macro_use] extern crate rocket;
use rocket_ws::{WebSocket, Stream};
//use rocket::tokio::sync::broadcast;
//use rocket::response::content;
//use rocket::State;
//use std::sync::Arc;

//#[derive(Clone)]
//struct WebSocketBroadcaster {
//  sender: broadcast::Sender<String>,
//}

#[get("/hello/<name>/<age>")]
fn helloage(name: &str, age: u8) -> String {
  format!("Hello, {} year old named {}!\n", age, name)
}

#[get("/hello/<name>")]
fn hello(name: &str/*, broadcaster: &State<Arc<WebSocketBroadcaster>>*/) -> String {
  let message = format!("Hello, {}!\n", name);
  //broadcaster.sender.send(message.clone()).ok();
  message
}

#[post("/login", data = "<login>")]
fn login(login: &str) -> String {
  format!("Hello, {}!\n", login)
}

#[get("/echo")]
fn echo_stream(ws: WebSocket/*, broadcaster: &State<Arc<WebSocketBroadcaster>>*/) -> Stream!['static] {
  Stream! { ws =>
    //let mut receiver = broadcaster.sender.subscribe();
    //while let Ok(message) = receiver.recv().await {
    //  broadcaster.sender.send(message).await.ok();
    //}
    for await message in ws {
      yield message?;
    }
  }
}

#[launch]
fn rocket() -> _ {
  //let (sender, _) = broadcast::channel(100);
  //let broadcaster = Arc::new(WebSocketBroadcaster { sender });
  rocket::build().mount("/", routes![
    hello,
    helloage,
    login,
    echo_stream]
  )
  //.manage(broadcaster)
}
