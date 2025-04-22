#[macro_use] extern crate rocket;
use rocket_ws::{WebSocket, Stream};

#[get("/hello/<name>/<age>")]
fn helloage(name: &str, age: u8) -> String {
  format!("Hello, {} year old named {}!\n", age, name)
}

#[get("/hello/<name>")]
fn hello(name: &str) -> String {
  let message = format!("Hello, {}!\n", name);
  message
}

#[post("/login", data = "<login>")]
fn login(login: &str) -> String {
  format!("Hello, {}!\n", login)
}

#[get("/echo")]
fn echo_stream(ws: WebSocket) -> Stream!['static] {
  Stream! { ws =>
    for await message in ws {
      yield message?;
    }
  }
}

#[launch]
fn rocket() -> _ {
  rocket::build().mount("/", routes![
    hello,
    helloage,
    login,
    echo_stream]
  )
}
