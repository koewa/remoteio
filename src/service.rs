use async_std::task;
use std::time::Duration;
use tokio::sync::broadcast::{channel, Receiver, Sender};

// Task that broadcasts a message every 10 seconds
pub struct Service {}

fn run_timer(name: String, tx: Sender<String>, interval: Duration) {
    tokio::spawn(async move {
        loop {
            task::sleep(interval).await;
            if tx.send(name.clone()).is_err() {
                println!("no one listening to task");
            }
        }
    });
}

impl Service {
    fn new(tx: Sender<String>) {
        let (txtask, _) = channel::<String>(10);
        let this = Self {};

        run_timer("first".to_string(), txtask.clone(), Duration::from_secs(2));
        run_timer("second".to_string(), txtask.clone(), Duration::from_secs(3));

        tokio::spawn(async move {
            this.broadcast_task(txtask.subscribe(), tx).await;
        });
    }

    pub async fn broadcast_task(self, mut rx: Receiver<String>, tx: Sender<String>) {
        loop {
            if let Ok(name) = rx.recv().await {
                let msg = format!(
                    "Server broadcast: Hello from server to all WebSocket clients ({})!",
                    name
                )
                .to_string();
                println!("Broadcasting message: {}", msg);

                if tx.send(msg).is_err() {
                    println!("No active WebSocket clients to send the message to!",);
                }
            } else {
                println!("Some error occured");
            }
        }
    }
}

pub fn setup_broadcaster(tx: Sender<String>) {
    Service::new(tx);
}
