use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Notify;

#[derive(Debug)]
pub enum ServerState {
    Connected,
    Disconnected,
}

pub struct Status {
    pub state: ServerState,
    pub nbr_of_calls: u32,
    pub tx: Sender<String>,
    pub shutdown: Arc<Notify>,
    pub todos: Vec<String>,
}
