use async_std::task;
use std::time::Duration;
use tokio::sync::broadcast::{Sender};

// Task that broadcasts a message every 10 seconds
async fn broadcast_task(tx: Sender<String>, name: &str) {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        let message = format!("Server broadcast: Hello from {} to all WebSocket clients!", name).to_string();
        println!("Broadcasting message: {}", message);

        if tx.send(message).is_err() {
            println!("No active WebSocket clients to send the message to ({}).", name);
        }
    }
}

pub fn setup_broadcaster(tx: Sender<String>) {
    let tx_clone = tx.clone();
    tokio::spawn(async {
        broadcast_task(tx_clone, "first").await;
    });
    tokio::spawn(async {
        broadcast_task(tx, "second").await;
    });
}
