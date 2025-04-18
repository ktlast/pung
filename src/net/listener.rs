use crate::message::Message;
use crate::utils;
use tokio::net::UdpSocket;
use bincode;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub async fn listen(socket: &UdpSocket) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    
    // Track seen message IDs to avoid showing duplicates
    // We use a HashSet wrapped in Arc<Mutex<>> for thread safety
    let seen_message_ids = Arc::new(Mutex::new(HashSet::new()));
    
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
            // Check if we've already seen this message
            let mut seen_ids = seen_message_ids.lock().unwrap();
            
            // If this is a new message (not seen before), display it
            if seen_ids.insert(msg.message_id.clone()) {
                let formatted_time = utils::display_time_from_timestamp(msg.timestamp);
                println!("[{}]: {}     ({})", msg.sender, msg.content, formatted_time);
            }
            
            // Limit the size of the seen messages set to avoid memory growth 
            if seen_ids.len() > 1000 {
                // Keep only the 500 most recent messages (simple approach)
                // In a real app, you might want a more sophisticated approach
                *seen_ids = seen_ids.iter().take(500).cloned().collect();
            }
        } else {
            eprintln!("Received invalid message from {}", addr);
        }
    }
}
