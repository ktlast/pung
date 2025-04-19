use crate::message::{Message, MessageType};
use crate::peer::SharedPeerList;
use crate::peer::discovery;
use crate::peer::heartbeats;
use crate::utils;
use bincode;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

pub async fn listen(
    socket: Arc<UdpSocket>,
    peer_list: Option<SharedPeerList>,
    username: Option<String>,
    local_addr: Option<SocketAddr>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];

    // Track seen message IDs to avoid showing duplicates
    // We use a HashSet wrapped in Arc<Mutex<>> for thread safety
    let seen_message_ids = Arc::new(Mutex::new(HashSet::new()));
    let socket_clone = socket.clone();

    loop {
        let (len, addr) = socket_clone.clone().recv_from(&mut buf).await?;
        if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
            // Check if we've already seen this message
            let mut seen_ids = seen_message_ids.lock().await;

            // Process the message based on its type
            match msg.msg_type {
                MessageType::Chat => {
                    // If this is a new message (not seen before), display it
                    if seen_ids.insert(msg.message_id.clone()) {
                        let formatted_time = utils::display_time_from_timestamp(msg.timestamp);
                        println!("[{}]: {}     ({})", msg.sender, msg.content, formatted_time);
                    }
                }
                MessageType::Discovery => {
                    // Handle discovery message if peer tracking is enabled
                    if let (Some(peer_list), Some(username), Some(local_addr)) =
                        (&peer_list, &username, local_addr)
                    {
                        if let Err(e) = discovery::handle_discovery_message(
                            &msg,
                            peer_list,
                            socket_clone.clone(),
                            username,
                            local_addr,
                        )
                        .await
                        {
                            eprintln!("Error handling discovery message: {}", e);
                        }
                    }
                }
                MessageType::Heartbeat => {
                    // Handle heartbeat message if peer tracking is enabled
                    if let Some(peer_list) = &peer_list {
                        if let Err(e) = heartbeats::handle_heartbeat_message(&msg, peer_list).await
                        {
                            eprintln!("Error handling heartbeat message: {}", e);
                        }
                    }
                }
                MessageType::PeerList => {
                    // Handle peer list message if peer tracking is enabled
                    if let (Some(peer_list), Some(username), Some(local_addr)) =
                        (&peer_list, &username, local_addr)
                    {
                        if let Err(e) = discovery::handle_peer_list_message(
                            &msg,
                            peer_list,
                            socket_clone.clone(),
                            username,
                            local_addr,
                        )
                        .await
                        {
                            eprintln!("Error handling peer list message: {}", e);
                        }
                    }
                }
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
