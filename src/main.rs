#[macro_use] extern crate rocket;
use rocket_ws::{WebSocket, Stream};
use rocket::tokio::sync::broadcast;
use rocket::State;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Error {
  DivisionByZero,
  NonPositiveLogarithm,
  NegativeSquareRoot,
}

#[derive(Clone)]
struct WebSocketBroadcaster {
  sender: broadcast::Sender<String>,
  message: Result<String, Error>
}

#[get("/hello/<name>/<age>")]
fn helloage(name: &str, age: u8) -> String {
  format!("Hello, {} year old named {}!\n", age, name)
}

#[get("/hello/<name>")]
fn hello(name: &str, broadcaster: &State<Arc<WebSocketBroadcaster>>) -> String {
  let message = format!("Hello, {}!\n", name);
  broadcaster.sender.send(message.clone()).ok();
  //broadcaster.message = message.clone();
  message
}

#[post("/login", data = "<login>")]
fn login(login: &str) -> String {
  format!("Hello, {}!\n", login)
}

#[get("/echo")]
fn echo_stream(ws: WebSocket, broadcaster: &State<Arc<WebSocketBroadcaster>>) -> Stream!['static] {
  Stream! { ws =>
    for await message in ws {
      yield rocket_ws::Message(String::from("init"));
      //yield broadcaster.message?;
      //yield message?;
      //yield Result<String, MathError>(broadcaster.message)?
    }
  }
}

#[launch]
fn rocket() -> _ {
  let (sender, _) = broadcast::channel(100);
  let initstring : String = String::from("init");
  let broadcaster = Arc::new(WebSocketBroadcaster { sender, message: Ok(initstring.clone())});
  rocket::build().mount("/", routes![
    hello,
    helloage,
    login,
    echo_stream]
  )
  .manage(broadcaster)
}
