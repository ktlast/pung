use crate::DEFAULT_RECV_PORT;
use crate::message::Message;
use crate::net::sender;
use crate::peer::SharedPeerList;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;

// Constants for discovery
const DISCOVERY_INTERVAL: u64 = 30; // seconds
const BROADCAST_ADDR: &str = "255.255.255.255";

/// Starts the peer discovery process
pub async fn start_discovery(
    socket: Arc<UdpSocket>,
    username: String,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    let socket_clone = socket.clone();
    let username_clone = username.clone();

    // Send initial discovery message
    send_discovery_message(socket, &username, local_addr).await?;

    // Periodically send discovery messages
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(DISCOVERY_INTERVAL));

        loop {
            interval.tick().await;
            if let Err(e) =
                send_discovery_message(socket_clone.clone(), &username_clone, local_addr).await
            {
                eprintln!("Error sending discovery message: {}", e);
            }
        }
    });

    Ok(())
}

/// Sends a discovery message to the broadcast address on the receive port
async fn send_discovery_message(
    socket: Arc<UdpSocket>,
    username: &str,
    local_addr: SocketAddr,
) -> std::io::Result<()> {
    let discovery_msg = Message::new_discovery(username.to_string(), local_addr);

    // Broadcast to the default receive port
    let broadcast_addr = format!("{BROADCAST_ADDR}:{}", DEFAULT_RECV_PORT);
    sender::send_message(socket.clone(), &discovery_msg, &broadcast_addr).await?;

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
            peer_list.add_or_update_peer(addr, msg.sender.clone());
            let socket_clone = socket.clone();

            // Send a discovery response back to the peer
            let response = Message::new_discovery(username.to_string(), local_addr);
            sender::send_message(socket_clone.clone(), &response, addr_str).await?;

            // Optionally, send our peer list to the new peer
            let peers = peer_list.get_peers();
            if !peers.is_empty() {
                let peer_addrs: Vec<String> = peers.iter().map(|p| p.addr.to_string()).collect();

                let peer_list_msg =
                    Message::new_peer_list(username.to_string(), peer_addrs, local_addr);
                sender::send_message(socket_clone.clone(), &peer_list_msg, addr_str).await?;
            }
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

            // Check if this is a new peer
            let is_new = !peer_list_lock.update_last_seen(&addr);

            if is_new {
                // We don't know the username yet, so use the address as a temporary name
                peer_list_lock.add_or_update_peer(addr, addr.to_string());
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
        println!("Discovered new peers from peer list");
    }

    Ok(())
}
