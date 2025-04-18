mod net;
mod message;
mod utils;

use tokio::net::UdpSocket;
use tokio::io::{self, AsyncBufReadExt};
use net::{broadcaster, listener};
use message::Message;
use std::env;

const DEFAULT_SEND_PORT: u16 = 8888;
const DEFAULT_RECV_PORT: u16 = 8889;
// List of common ports that instances might be listening on
const COMMON_RECV_PORTS: [u16; 3] = [8889, 8888, 8856];

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Parse command line arguments for port configuration
    let args: Vec<String> = env::args().collect();
    
    // Format: cargo run [send_port] [recv_port]
    let send_port = if args.len() > 1 { args[1].parse().unwrap_or(DEFAULT_SEND_PORT) } else { DEFAULT_SEND_PORT };
    let recv_port = if args.len() > 2 { args[2].parse().unwrap_or(DEFAULT_RECV_PORT) } else { DEFAULT_RECV_PORT };
    
    // We'll broadcast to all common receive ports to ensure all instances receive our messages
    // Each instance will ignore messages from itself based on the message ID
    
    println!("Starting rossip with send_port={}, recv_port={}", send_port, recv_port);
    
    let socket_send = UdpSocket::bind(format!("0.0.0.0:{}", send_port)).await?;
    socket_send.set_broadcast(true)?;
    
    let socket_recv = UdpSocket::bind(format!("0.0.0.0:{}", recv_port)).await?;

    // Spawn listener
    tokio::spawn(async move {
        if let Err(e) = listener::listen(&socket_recv).await {
            eprintln!("Listen error: {:?}", e);
        }
    });

    // Read user input
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Enter messages:");

    while let Ok(Some(line)) = lines.next_line().await {
        let msg = Message {
            sender: "test".to_string(),
            content: line,
            message_id: nanoid::nanoid!(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        // Broadcast to all common ports to ensure all instances receive our messages
        for &port in COMMON_RECV_PORTS.iter() {
            broadcaster::send_message(&socket_send, &msg, &format!("255.255.255.255:{}", port)).await?;
        }
    }

    Ok(())
}
