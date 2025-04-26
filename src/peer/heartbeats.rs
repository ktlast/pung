use crate::message::Message;
use crate::net::sender;
use crate::peer::SharedPeerList;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;

// Constants for heartbeat
const HEARTBEAT_INTERVAL: u64 = 15; // seconds
const PEER_TIMEOUT: u64 = 60; // seconds

/// Starts the heartbeat mechanism to maintain peer liveness
pub async fn start_heartbeat(
    socket: Arc<UdpSocket>,
    username: String,
    local_addr: SocketAddr,
    peer_list: SharedPeerList,
) -> std::io::Result<()> {
    // Start heartbeat sender
    let username_clone = username.clone();
    let peer_list_clone = peer_list.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        let socket_clone = socket.clone();

        loop {
            interval.tick().await;
            log::debug!("[Heartbeat] Sending heartbeats");
            if let Err(e) = send_heartbeats(
                socket_clone.clone(),
                &username_clone,
                local_addr,
                &peer_list_clone,
            )
            .await
            {
                log::error!("Error sending heartbeats: {}", e);
            }
        }
    });

    // Start peer timeout checker
    let peer_list_clone = peer_list.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(HEARTBEAT_INTERVAL));

        loop {
            interval.tick().await;
            check_peer_timeouts(&peer_list_clone).await;
        }
    });

    Ok(())
}

/// Sends heartbeat messages to all known peers
async fn send_heartbeats(
    socket: Arc<UdpSocket>,
    username: &str,
    local_addr: SocketAddr,
    peer_list: &SharedPeerList,
) -> std::io::Result<()> {
    let heartbeat_msg = Message::new_heartbeat(username.to_string(), local_addr);
    let socket_clone = socket.clone();
    // Get the current list of peers
    let peers = {
        let peer_list = peer_list.lock().await;
        peer_list.get_peers()
    };

    // Send heartbeat to each peer
    for peer in peers {
        sender::send_message(socket_clone.clone(), &heartbeat_msg, &peer.addr.to_string()).await?;
    }

    Ok(())
}

/// Checks for peers that haven't been seen recently and removes them
async fn check_peer_timeouts(peer_list: &SharedPeerList) {
    let timeout = Duration::from_secs(PEER_TIMEOUT);
    let stale_peers = {
        let mut peer_list = peer_list.lock().await;
        peer_list.remove_stale_peers(timeout)
    };

    // Log removed peers
    for username in stale_peers {
        println!("Peer timed out and was removed: {}", username);
    }
}

/// Handles an incoming heartbeat message
pub async fn handle_heartbeat_message(
    msg: &Message,
    peer_list: &SharedPeerList,
) -> std::io::Result<()> {
    if let Some(addr_str) = &msg.sender_addr {
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            let mut peer_list = peer_list.lock().await;

            // If we already know this peer, update the last_seen time
            if peer_list.update_last_seen(&addr) {
                // Peer already known, just updated last_seen
            } else {
                // This is a new peer, add it to our list
                peer_list.add_or_update_peer(addr, msg.sender.clone());
                println!("New peer discovered via heartbeat: {} ({})", msg.sender, addr);
            }
        }
    }

    Ok(())
}
