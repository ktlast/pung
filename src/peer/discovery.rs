use crate::DEFAULT_RECV_INIT_PORT;
use crate::message::Message;
use crate::net::sender;
use crate::peer::SharedPeerList;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::UdpSocket;

// Constants for discovery
const BROADCAST_ADDR: &str = "255.255.255.255";
const DEFAULT_BROADCAST_INTERVAL_SEC: u64 = 900;

/// Starts the peer discovery process
pub async fn start_discovery(
    socket: Arc<UdpSocket>,
    username: String,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    tokio::spawn(async move {
        // Send initial discovery message
        if let Err(e) = send_discovery_message(socket.clone(), &username, local_addr).await {
            log::error!("Error sending initial discovery message: {}", e);
        }

        // Start a timer to send discovery messages at regular intervals
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
            DEFAULT_BROADCAST_INTERVAL_SEC,
        ));
        loop {
            interval.tick().await;
            if let Err(e) = send_discovery_message(socket.clone(), &username, local_addr).await {
                log::error!("Error sending discovery message: {}", e);
            }
        }
    });
    Ok(())
}

/// Sends a discovery message to the broadcast address on multiple ports
pub async fn send_discovery_message(
    socket: Arc<UdpSocket>,
    username: &str,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    let discovery_msg = Message::new_discovery(username.to_string(), local_addr);

    // Broadcast to the default init port
    let broadcast_addr = format!("{BROADCAST_ADDR}:{}", DEFAULT_RECV_INIT_PORT);
    sender::send_message(socket.clone(), &discovery_msg, &broadcast_addr).await?;

    // Also broadcast to the local port that this peer is using
    // This helps reach peers that couldn't bind to the default init port
    let local_port = local_addr.port();
    if local_port != DEFAULT_RECV_INIT_PORT {
        let alt_broadcast_addr = format!("{BROADCAST_ADDR}:{}", local_port);
        sender::send_message(socket.clone(), &discovery_msg, &alt_broadcast_addr).await?;
    }

    Ok(())
}

/// Handles an incoming discovery message
pub async fn handle_discovery_message(
    msg: &Message,
    peer_list: &SharedPeerList,
    socket: Arc<UdpSocket>,
    username: &str,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    if let Some(addr_str) = &msg.sender_addr {
        if let Ok(addr) = SocketAddr::from_str(addr_str) {
            // Add the peer to our list
            let mut peer_list = peer_list.lock().await;

            // Check if this is a new peer before printing a message
            let is_new = peer_list.find_username_by_addr(&addr).is_none();

            // Always add or update the peer with their exact (username, IP, port)
            // This ensures proper uniqueness and prevents cross-refreshing
            peer_list.add_or_update_peer(addr, msg.sender.clone());

            // Only print a message if this is a new peer
            if is_new {
                println!("### New peer discovered: {} ({})", msg.sender, addr);
            }

            let socket_clone = socket.clone();

            // Send a discovery response back to the peer
            let response = Message::new_discovery(username.to_string(), local_addr);
            sender::send_message(socket_clone.clone(), &response, addr_str).await?;

            // Always send our peer list to the new peer (even if it's just us)
            // This ensures complete peer discovery across the network
            let peers = peer_list.get_peers();

            // Include ourselves in the peer list if we're not already there
            let mut has_self = false;
            for peer in &peers {
                if peer.addr == local_addr {
                    has_self = true;
                    break;
                }
            }

            // Create the list of peer addresses to share
            let mut peer_addrs: Vec<String> = peers.iter().map(|p| p.addr.to_string()).collect();

            // Always include ourselves in the peer list we share
            if !has_self {
                peer_addrs.push(local_addr.to_string());
            }

            // Send the peer list message
            let peer_list_msg =
                Message::new_peer_list(username.to_string(), peer_addrs, local_addr);
            sender::send_message(socket_clone.clone(), &peer_list_msg, addr_str).await?;

            // Log that we shared our peer list
            println!("@@@ Shared peer list with {} ({})", msg.sender, addr);
        }
    }

    Ok(())
}

/// Handles an incoming peer list message
pub async fn handle_peer_list_message(
    msg: &Message,
    peer_list: &SharedPeerList,
    socket: Arc<UdpSocket>,
    username: &str,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    // Parse the peer list from the message content
    let peer_addrs: Vec<&str> = msg.content.split(',').collect();
    let mut new_peers = false;
    let socket_clone = socket.clone();

    // Add each peer to our list
    let mut peer_list_lock = peer_list.lock().await;

    for addr_str in peer_addrs {
        if addr_str.is_empty() {
            continue;
        }

        if let Ok(addr) = SocketAddr::from_str(addr_str) {
            // Don't add ourselves
            if addr == local_addr {
                continue;
            }

            // Skip if this looks like an anonymous peer from another instance
            // This helps prevent the proliferation of anonymous peers
            if addr_str.contains("anonymous@") {
                log::debug!("Skipping anonymous peer: {}", addr_str);
                continue;
            }

            // Always add or update the peer with their exact (username, IP, port)
            // This ensures proper uniqueness and prevents cross-refreshing
            let is_new = peer_list_lock.find_username_by_addr(&addr).is_none();

            // Add the peer with their address
            if is_new {
                // For new peers, use a temporary name until we learn their real username
                let temp_name = format!("peer@{}", addr);
                peer_list_lock.add_or_update_peer(addr, temp_name);
                new_peers = true;

                // Send a discovery message to this new peer
                let discovery_msg = Message::new_discovery(username.to_string(), local_addr);
                sender::send_message(socket_clone.clone(), &discovery_msg, &addr.to_string())
                    .await?;
            }
        }
    }

    // If we added new peers, log it
    if new_peers {
        println!("### Discovered new peers from peer list");
    }

    Ok(())
}
