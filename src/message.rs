use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode)]
pub enum MessageType {
    Chat,
    Discovery,
    Heartbeat,
    PeerList,
}

#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode)]
pub struct Message {
    pub sender: String,
    pub content: String,
    pub message_id: String,
    pub timestamp: i64,
    pub msg_type: MessageType,
    pub sender_addr: Option<String>, // String representation of SocketAddr for serialization
    pub known_peers: Option<Vec<(String, String)>>, // (username, addr as string)
}

impl Message {
    pub fn new_chat(sender: String, content: String, sender_addr: Option<SocketAddr>) -> Self {
        Message {
            sender,
            content,
            message_id: nanoid::nanoid!(),
            timestamp: chrono::Utc::now().timestamp(),
            msg_type: MessageType::Chat,
            sender_addr: sender_addr.map(|addr| addr.to_string()),
            known_peers: None,
        }
    }

    pub fn new_discovery(sender: String, sender_addr: SocketAddr) -> Self {
        Message {
            sender,
            content: "DISCOVERY".to_string(),
            message_id: nanoid::nanoid!(),
            timestamp: chrono::Utc::now().timestamp(),
            msg_type: MessageType::Discovery,
            sender_addr: Some(sender_addr.to_string()),
            known_peers: None,
        }
    }

    pub fn new_heartbeat(
        sender: String,
        sender_addr: SocketAddr,
        known_peers: Vec<(String, String)>,
    ) -> Self {
        Message {
            sender,
            content: "HEARTBEAT".to_string(),
            message_id: nanoid::nanoid!(),
            timestamp: chrono::Utc::now().timestamp(),
            msg_type: MessageType::Heartbeat,
            sender_addr: Some(sender_addr.to_string()),
            known_peers: Some(known_peers),
        }
    }

    pub fn new_peer_list(sender: String, peers: Vec<String>, sender_addr: SocketAddr) -> Self {
        // Format peer list as a comma-separated string
        let peer_list = peers.join(",");

        Message {
            sender,
            content: peer_list,
            message_id: nanoid::nanoid!(),
            timestamp: chrono::Utc::now().timestamp(),
            msg_type: MessageType::PeerList,
            sender_addr: Some(sender_addr.to_string()),
            known_peers: None,
        }
    }
}
