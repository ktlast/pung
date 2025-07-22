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
use unicode_width::UnicodeWidthStr;

pub async fn listen(
    socket: Arc<UdpSocket>,
    peer_list: Option<SharedPeerList>,
    username: Option<String>,
    local_addr: Option<SocketAddr>,
    terminal_width: Option<usize>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];

    // Track seen message IDs to avoid showing duplicates
    // We use a HashSet wrapped in Arc<Mutex<>> for thread safety
    let seen_message_ids = Arc::new(Mutex::new(HashSet::new()));
    let socket_clone = socket.clone();

    loop {
        let (len, addr) = socket_clone.clone().recv_from(&mut buf).await?;
        if let Ok((msg, _)) =
            bincode::decode_from_slice::<Message, _>(&buf[..len], bincode::config::standard())
        {
            // Check if we've already seen this message
            let mut seen_ids = seen_message_ids.lock().await;

            // Process the message based on its type
            match msg.msg_type {
                MessageType::Chat => {
                    // If this is a new message (not seen before), display it
                    if seen_ids.insert(msg.message_id.clone()) {
                        let formatted_time = utils::display_time_from_timestamp(msg.timestamp);
                        let sender_name = &msg.sender;

                        // Verify the sender's username against our peer list if available
                        let verified_sender = if let (Some(peer_list), Some(sender_addr)) =
                            (&peer_list, &msg.sender_addr)
                        {
                            if let Ok(socket_addr) = sender_addr.parse::<SocketAddr>() {
                                let peer_list_lock = peer_list.lock().await;
                                // Use find_username_by_addr to verify the sender's username
                                match peer_list_lock.find_username_by_addr(&socket_addr) {
                                    Some(verified_name) => {
                                        if &verified_name != sender_name {
                                            // Username mismatch - use the verified one but note the discrepancy
                                            format!("{verified_name} (claimed: {sender_name})")
                                        } else {
                                            // Username matches what we expect
                                            verified_name
                                        }
                                    }
                                    None => {
                                        // We don't know this peer yet, use the claimed name but mark as unverified
                                        format!("{sender_name} (unverified)")
                                    }
                                }
                            } else {
                                sender_name.clone()
                            }
                        } else {
                            sender_name.clone()
                        };

                        // Use provided terminal width or default to 80 characters
                        let term_width = terminal_width.unwrap_or(80);

                        // Calculate the base message length (sender + content)
                        let base_msg = format!("[{verified_sender}]: {}", msg.content);
                        let time_display = format!(" ({formatted_time})");

                        // Calculate padding needed to right-align the timestamp
                        // Use UnicodeWidthStr to get the correct display width for multi-byte characters
                        let base_msg_width = UnicodeWidthStr::width(base_msg.as_str());
                        let time_display_width = UnicodeWidthStr::width(time_display.as_str());
                        let padding = term_width
                            .saturating_sub(base_msg_width)
                            .saturating_sub(time_display_width);

                        // Format with proper padding
                        println!("{base_msg}{}{time_display}", " ".repeat(padding));
                    }
                }
                MessageType::Discovery => {} // Do nothing
                MessageType::Heartbeat => {
                    log::debug!("[Heartbeat] message received from: {}", msg.sender);
                    if let Some(addr) = &msg.sender_addr {
                        log::debug!("[Heartbeat] Sender address: {addr}");
                    }
                    // Handle heartbeat message if peer tracking is enabled
                    if let Some(peer_list) = &peer_list {
                        if let Err(e) = heartbeats::handle_heartbeat_message(&msg, peer_list).await
                        {
                            log::error!("Error handling heartbeat message: {e}");
                        }
                    }
                }
                MessageType::PeerList => {
                    // DEBUG: Display peer list message
                    log::debug!("[PeerList] message received from: {}", msg.sender);
                    if let Some(addr) = &msg.sender_addr {
                        log::debug!("[PeerList] Sender address: {addr}");
                    }
                    log::debug!("[PeerList] Peer list content: {}", msg.content);

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
                            log::error!("Error handling peer list message: {e}");
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
            log::error!("Received invalid message from {addr}");
        }
    }
}

pub async fn listen_for_init(
    socket_recv_only_for_init: Arc<UdpSocket>,
    peer_list: Option<SharedPeerList>,
    username: Option<String>,
    local_addr: Option<SocketAddr>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    // Start peer discovery
    loop {
        let (len, addr) = socket_recv_only_for_init
            .clone()
            .recv_from(&mut buf)
            .await?;
        if let Ok((msg, _)) =
            bincode::decode_from_slice::<Message, _>(&buf[..len], bincode::config::standard())
        {
            // Process the message based on its type
            if let MessageType::Discovery = msg.msg_type {
                // DEBUG: Display discovery message
                log::debug!("[Discovery] message received from: {}", msg.sender);
                if let Some(addr) = &msg.sender_addr {
                    log::debug!("[Discovery] Sender address: {addr}");
                }

                // Handle discovery message if peer tracking is enabled
                if let (Some(peer_list), Some(username), Some(local_addr)) =
                    (&peer_list, &username, local_addr)
                {
                    if let Err(e) = discovery::handle_discovery_message(
                        &msg,
                        peer_list,
                        socket_recv_only_for_init.clone(),
                        username,
                        local_addr,
                    )
                    .await
                    {
                        log::error!("Error handling discovery message: {e}");
                    }
                }
            }
        } else {
            log::error!("Received invalid message from {addr}");
        }
    }
}
