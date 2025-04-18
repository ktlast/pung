use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub sender: String,
    pub content: String,
    pub message_id: String,
    pub timestamp: i64,
}
