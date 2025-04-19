mod net;
mod message;
mod utils;
mod peer;

use tokio::net::UdpSocket;
use tokio::io::{self, AsyncBufReadExt};
use net::{broadcaster, listener};
use message::Message;
use peer::PeerList;
use peer::{discovery, heartbeats};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_SEND_PORT: u16 = 8888;
const DEFAULT_RECV_PORT: u16 = 8889;
// List of common ports that instances might be listening on
const COMMON_RECV_PORTS: [u16; 3] = [8889, 8888, 8856];
// Default username
const DEFAULT_USERNAME: &str = "user";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Parse command line arguments for port configuration
    let args: Vec<String> = env::args().collect();
    
    // Format: cargo run [send_port] [recv_port] [username]
    let send_port = if args.len() > 1 { args[1].parse().unwrap_or(DEFAULT_SEND_PORT) } else { DEFAULT_SEND_PORT };
    let recv_port = if args.len() > 2 { args[2].parse().unwrap_or(DEFAULT_RECV_PORT) } else { DEFAULT_RECV_PORT };
    let username = if args.len() > 3 { args[3].clone() } else { DEFAULT_USERNAME.to_string() };
    
    // We'll broadcast to all common receive ports to ensure all instances receive our messages
    // Each instance will ignore messages from itself based on the message ID
    
    println!("Starting rossip with username={}, send_port={}, recv_port={}", username, send_port, recv_port);
    
    // Create shared peer list for tracking peers
    let peer_list = Arc::new(Mutex::new(PeerList::new()));
    
    // Bind sockets
    let socket_send = UdpSocket::bind(format!("0.0.0.0:{}", send_port)).await?;
    socket_send.set_broadcast(true)?;
    
    let socket_recv = UdpSocket::bind(format!("0.0.0.0:{}", recv_port)).await?;
    
    // Get local address for peer discovery
    let local_addr = socket_send.local_addr()?;

    // Clone variables for the listener task
    let socket_recv_clone = socket_recv;
    let peer_list_clone = peer_list.clone();
    let username_clone = username.clone();
    
    // Spawn listener with peer list for discovery and heartbeat handling
    tokio::spawn(async move {
        if let Err(e) = listener::listen(&socket_recv_clone, Some(peer_list_clone), Some(username_clone), Some(local_addr)).await {
            eprintln!("Listen error: {:?}", e);
        }
    });
    
    // Start peer discovery
    let discovery_socket = UdpSocket::bind(format!("0.0.0.0:{}", send_port + 1)).await?;
    discovery_socket.set_broadcast(true)?;
    let username_clone = username.clone();
    
    discovery::start_discovery(
        discovery_socket,
        username_clone,
        local_addr,
        COMMON_RECV_PORTS.to_vec(),
    ).await?;
    
    // Start heartbeat mechanism
    let heartbeat_socket = UdpSocket::bind(format!("0.0.0.0:{}", send_port + 2)).await?;
    heartbeat_socket.set_broadcast(true)?;
    let peer_list_clone = peer_list.clone();
    let username_clone = username.clone();
    
    heartbeats::start_heartbeat(
        heartbeat_socket,
        username_clone,
        local_addr,
        peer_list_clone,
    ).await?;

    // Read user input
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Enter messages:");

    while let Ok(Some(line)) = lines.next_line().await {
        // Create a chat message
        let msg = Message::new_chat(username.clone(), line, Some(local_addr));
        
        // Broadcast to all common ports to ensure all instances receive our messages
        for &port in COMMON_RECV_PORTS.iter() {
            broadcaster::send_message(&socket_send, &msg, &format!("255.255.255.255:{}", port)).await?;
        }
    }

    Ok(())
}
