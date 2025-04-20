mod message;
mod net;
mod peer;
mod utils;

use message::Message;
use net::{listener, sender};
use peer::PeerList;
use peer::{discovery, heartbeats};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

const DEFAULT_SEND_PORT: u16 = 8888;
const DEFAULT_RECV_PORT: u16 = 9487;
// List of common ports that instances might be listening on
// We only use one receive port now
// Default username
const DEFAULT_USERNAME: &str = "user";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Parse command line arguments for port configuration
    let args: Vec<String> = env::args().collect();

    // Format: cargo run [username] [send_port] [sender_only]
    // We're now using the DEFAULT_RECV_PORT constant directly
    let username = if args.len() > 1 {
        args[1].clone()
    } else {
        DEFAULT_USERNAME.to_string()
    };
    let send_port = if args.len() > 2 {
        args[2].parse().unwrap_or(DEFAULT_SEND_PORT)
    } else {
        DEFAULT_SEND_PORT
    };
    let sender_only = if args.len() > 3 {
        args[3].to_lowercase() == "true" || args[3] == "1"
    } else {
        false
    };

    // We'll broadcast to all common receive ports to ensure all instances receive our messages
    // Each instance will ignore messages from itself based on the message ID

    println!(
        "@@@ Starting rossip with username={}, send_port={}, recv_port={}, sender_only={}",
        username, send_port, DEFAULT_RECV_PORT, sender_only
    );

    // Create shared peer list for tracking peers
    let peer_list = Arc::new(Mutex::new(PeerList::new()));

    // Get local LAN IP address
    let local_ip = utils::get_local_ip().unwrap_or_else(|| {
        println!("Warning: Could not determine local IP address, using 0.0.0.0");
        "0.0.0.0".parse().unwrap()
    });
    println!("@@@ Using local IP address: {}", local_ip);

    // Bind sockets
    let socket_send = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", send_port)).await?);
    socket_send.set_broadcast(true)?;

    // Only bind the receive socket if not in sender-only mode
    let socket_recv = if !sender_only {
        Some(Arc::new(
            UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_RECV_PORT)).await?,
        ))
    } else {
        None
    };

    // Create a proper socket address with the local IP for peer discovery
    let local_addr = SocketAddr::new(local_ip, send_port);

    // Prepare shared socket for sending
    let socket_send_clone = socket_send.clone();

    if sender_only {
        println!("Running in sender-only mode. Will not receive any messages.");
        // Start peer discovery
        let username_clone = username.clone();
        discovery::start_discovery(socket_send_clone.clone(), username_clone, local_addr).await?;
    } else {
        // Set up two-way communication (both sending and receiving)
        if let Some(recv_socket) = socket_recv {
            // Start the listener
            let peer_list_clone = peer_list.clone();
            let username_clone = username.clone();

            tokio::spawn(async move {
                if let Err(e) = listener::listen(
                    recv_socket.clone(),
                    Some(peer_list_clone),
                    Some(username_clone),
                    Some(local_addr),
                )
                .await
                {
                    eprintln!("Listen error: {:?}", e);
                }
            });

            // Start peer discovery
            let username_clone = username.clone();
            discovery::start_discovery(socket_send_clone.clone(), username_clone, local_addr)
                .await?;

            // Start heartbeat mechanism
            let peer_list_clone = peer_list.clone();
            let username_clone = username.clone();
            heartbeats::start_heartbeat(
                socket_send_clone.clone(),
                username_clone,
                local_addr,
                peer_list_clone,
            )
            .await?;
        }
    }

    // Read user input
    let peers = {
        let peer_list_lock = peer_list.lock().await;
        peer_list_lock.get_peers()
    };
    println!("@@@ Known peers: {:?}", peers);
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Enter messages:");

    while let Ok(Some(line)) = lines.next_line().await {
        // Create a chat message
        let msg = Message::new_chat(username.clone(), line, Some(local_addr));

        // Get the list of known peers
        let peers = {
            let peer_list_lock = peer_list.lock().await;
            peer_list_lock.get_peers()
        };

        // Send the message to each known peer
        for peer in peers {
            // Use the peer's IP address but with the DEFAULT_RECV_PORT
            let peer_ip = peer.addr.ip();
            let target_addr = format!("{peer_ip}:{DEFAULT_RECV_PORT}");
            sender::send_message(socket_send_clone.clone(), &msg, &target_addr).await?;
        }
    }

    Ok(())
}
