use crate::message::Message;
use crate::net::sender;
use crate::peer::SharedPeerList;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;

// Constants for heartbeat
const HEARTBEAT_INTERVAL: u64 = 6; // seconds
const PEER_TIMEOUT: u64 = 15; // seconds

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
        let socket_clone = socket.clone();
        
        // Send a heartbeat immediately when starting
        log::debug!("[Heartbeat] Sending initial heartbeat");
        if let Err(e) = send_heartbeats(
            socket_clone.clone(),
            &username_clone,
            local_addr,
            &peer_list_clone,
        )
        .await
        {
            log::error!("Error sending initial heartbeat: {}", e);
        }
        
        // Then set up the regular interval for subsequent heartbeats
        let mut interval = time::interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        
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
        // Check for timeouts immediately when starting
        check_peer_timeouts(&peer_list_clone).await;
        
        // Then set up the regular interval for subsequent checks
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
    // Gather known peers as (username, addr) pairs, skipping self
    let peers = {
        let peer_list = peer_list.lock().await;
        peer_list
            .get_peers()
            .into_iter()
            .map(|p| (p.username.clone(), p.addr.to_string()))
            .collect::<Vec<_>>()
    };

    let heartbeat_msg = Message::new_heartbeat(username.to_string(), local_addr, peers.clone());
    let socket_clone = socket.clone();
    // Send heartbeat to each peer
    for (_, peer_addr_str) in peers {
        if let Ok(peer_addr) = peer_addr_str.parse::<SocketAddr>() {
            sender::send_message(socket_clone.clone(), &heartbeat_msg, &peer_addr.to_string())
                .await?;
        }
    }
    Ok(())
}

/// Checks for peers that haven't been seen recently and removes them
async fn check_peer_timeouts(peer_list: &SharedPeerList) {
    let timeout = Duration::from_secs(PEER_TIMEOUT);

    // Each (username, IP, port) combination is treated as a unique peer
    // No consolidation is performed - this allows multiple instances on the same machine

    // Then remove stale peers
    let stale_peers = {
        let mut peer_list = peer_list.lock().await;
        peer_list.remove_stale_peers(timeout)
    };

    // Log removed peers
    for username in stale_peers {
        println!("### Peer timed out and was removed: {}", username);
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

            // Always add or update the sender with the exact (username, IP, port)
            // This is the only peer we know for sure is active (since we just received a message from it)
            peer_list.add_or_update_peer(addr, msg.sender.clone());

            // IMPORTANT: We do NOT update the last_seen timestamp for peers in the known_peers list
            // We only use known_peers to discover new peers, not to refresh existing ones
            // This ensures that when a peer is closed, it will be properly removed after timeout
            if let Some(known_peers) = &msg.known_peers {
                for (peer_name, peer_addr_str) in known_peers {
                    if let Ok(peer_addr) = peer_addr_str.parse::<SocketAddr>() {
                        // Only add this peer if it's new (not already in our list)
                        // This prevents refreshing peers that might no longer be active
                        if peer_list.find_username_by_addr(&peer_addr).is_none() {
                            println!("@@@ Discovered new peer from heartbeat: {} ({})", peer_name, peer_addr);
                            peer_list.add_or_update_peer(peer_addr, peer_name.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
