mod message;
mod net;
mod peer;
mod utils;

use clap::{Arg, Command};
use message::Message;
use net::{listener, sender};
use peer::PeerList;
use peer::{discovery, heartbeats};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Parse command line arguments using clap
    let matches = Command::new("Rossip Chat")
        .version("1.0")
        .author("Your Name")
        .about("A simple UDP-based chat application")
        .arg(
            Arg::new("username")
                .short('u')
                .long("username")
                .value_name("USERNAME")
                .help("Sets the username for chat")
                .default_value("user"),
        )
        .arg(
            Arg::new("receive_port")
                .short('r')
                .long("receive-port")
                .value_name("PORT")
                .help("Sets the port for receiving messages")
                .default_value("9487"),
        )
        .arg(
            Arg::new("sender_only")
                .short('s')
                .long("sender-only")
                .help("Run in sender-only mode (no receiving)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Extract values from command line arguments
    let username = matches.get_one::<String>("username").unwrap().clone();

    // If send_port is not provided or is set to 0, generate a random port
    let send_port = utils::get_random_port(20000, 30000);
    let receive_port = matches
        .get_one::<String>("receive_port")
        .unwrap()
        .parse::<u16>()
        .unwrap_or(9487);
    let sender_only = matches.get_flag("sender_only");

    // We'll broadcast to all common receive ports to ensure all instances receive our messages
    // Each instance will ignore messages from itself based on the message ID

    println!(
        "@@@ Starting rossip with username={}, send_port={}, recv_port={}, sender_only={}",
        username, send_port, receive_port, sender_only
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
            UdpSocket::bind(format!("0.0.0.0:{}", receive_port)).await?,
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
        discovery::start_discovery(
            socket_send_clone.clone(),
            username_clone,
            local_addr,
            receive_port,
        )
        .await?;
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
            discovery::start_discovery(
                socket_send_clone.clone(),
                username_clone,
                local_addr,
                receive_port,
            )
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
            // Use the peer's IP address but with the receive_port
            let peer_ip = peer.addr.ip();
            let target_addr = format!("{peer_ip}:{receive_port}");
            sender::send_message(socket_send_clone.clone(), &msg, &target_addr).await?;
        }
    }

    Ok(())
}
