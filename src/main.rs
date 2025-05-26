mod message;
mod net;
mod peer;
mod ui;
mod utils;

use clap::{Arg, Command};
use message::Message;
use net::{listener, sender};
use peer::PeerList;
use peer::{discovery, heartbeats};
use rand::RngCore;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::task;

const DEFAULT_RECV_INIT_PORT: u16 = 9487;

// Get version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> rustyline::Result<()> {
    // Parse command line arguments using clap
    let matches = Command::new("pung")
        .version(VERSION)
        .author("Your Name")
        .about("Peer-to-peer UDP Network Gossip.")
        .arg(
            Arg::new("username")
                .short('u')
                .long("username")
                .value_name("USERNAME")
                .help("Sets the username for chat"),
        )
        .arg(
            Arg::new("receive_port")
                .short('r')
                .long("receive-port")
                .value_name("PORT")
                .help("Sets the port for receiving messages (random if not specified)"),
        )
        .arg(
            Arg::new("terminal_width")
                .short('w')
                .long("width")
                .value_name("WIDTH")
                .help("Sets the terminal width for message display (default: 80)"),
        )
        .get_matches();

    // Extract values from command line arguments
    let username = match matches.get_one::<String>("username") {
        Some(username) => {
            // Limit username to 12 characters
            if username.len() > 12 {
                username[0..12].to_string()
            } else {
                username.clone()
            }
        }
        None => {
            let mut bytes = [0u8; 2];
            rand::rng().fill_bytes(&mut bytes);
            format!("user-{}", hex::encode(bytes))
        }
    };

    // Generate a random port for sending
    let send_port = utils::get_random_port(20000, 30000);

    // Generate a random port for receiving if not specified
    let receive_port = match matches.get_one::<String>("receive_port") {
        Some(port_str) => port_str
            .parse::<u16>()
            .unwrap_or_else(|_| utils::get_random_port(10000, 20000)),
        None => utils::get_random_port(10000, 20000),
    };

    // Get terminal width from command-line arguments or use default
    let terminal_width = match matches.get_one::<String>("terminal_width") {
        Some(width_str) => width_str.parse::<usize>().unwrap_or(80),
        None => 80,
    };

    let mut startup_message: Vec<String> = vec![];
    startup_message.push(format!("{:12} = {}", "Version", VERSION));
    startup_message.push(format!("{:12} = {}", "Username", username));
    startup_message.push(format!("{:12} = {}", "Send port", send_port));
    startup_message.push(format!("{:12} = {}", "Recv port", receive_port));

    // Create shared peer list for tracking peers
    let peer_list = Arc::new(Mutex::new(PeerList::new()));

    // Get local LAN IP address
    let local_ip = utils::get_local_ip().unwrap_or_else(|| {
        println!("Warning: Could not determine local IP address, using 0.0.0.0");
        "0.0.0.0".parse().unwrap()
    });
    startup_message.push(format!("{:12} = {}", "Local IP", local_ip));

    // Bind sockets
    let socket_send = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", send_port)).await?);
    socket_send.set_broadcast(true)?;

    // Only bind the receive socket
    let socket_recv = Some(Arc::new(
        UdpSocket::bind(format!("0.0.0.0:{}", receive_port)).await?,
    ));

    // Create a proper socket address with the local IP for peer discovery
    let local_addr = SocketAddr::new(local_ip, receive_port);

    // Always send a discovery broadcast, regardless of whether the init port is available
    // This ensures we can find all peers, even after restarting

    // Try to bind to the init port, but don't worry if it's already in use
    let socket_recv_only_for_init =
        match UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_RECV_INIT_PORT)).await {
            Ok(sock) => {
                startup_message.push(format!("{:12} = {}", "Init port", DEFAULT_RECV_INIT_PORT));
                Some(Arc::new(sock))
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                startup_message.push(format!(
                    "{:12} = {}* already in use",
                    "Init port", DEFAULT_RECV_INIT_PORT
                ));
                None
            }
            Err(e) => return Err(e.into()),
        };

    // Prepare shared socket for sending
    let socket_send_clone = socket_send.clone();

    // Set up two-way communication (both sending and receiving)
    if let Some(recv_socket) = socket_recv {
        // Start the listener
        let peer_list_clone = peer_list.clone();
        let username_clone = username.clone();

        let terminal_width_clone = terminal_width;
        tokio::spawn(async move {
            if let Err(e) = listener::listen(
                recv_socket.clone(),
                Some(peer_list_clone),
                Some(username_clone),
                Some(local_addr),
                Some(terminal_width_clone),
            )
            .await
            {
                eprintln!("Listen error: {:?}", e);
            }
        });

        // Only spawn the init listener if we successfully bound to the init port
        if let Some(init_socket) = socket_recv_only_for_init {
            let peer_list_clone = peer_list.clone();
            let username_clone = username.clone();
            tokio::spawn(async move {
                if let Err(e) = listener::listen_for_init(
                    init_socket,
                    Some(peer_list_clone),
                    Some(username_clone),
                    Some(local_addr),
                )
                .await
                {
                    eprintln!("Listen for init error: {:?}", e);
                }
            });
        } else {
            // No special mode - we just don't listen on the init port
            // This is fine as we've already sent a discovery message
            println!("@@@ Continuing without init port listener (already in use)");
        }

        startup_message.push("".to_string());
        startup_message.push("Tips:".to_string());
        startup_message.push("- use [/h] to show available commands".to_string());
        startup_message.push("- use [/v] to show version and check for updates".to_string());

        utils::display_message_block("Startup", startup_message);

        // Start peer discovery - always send a broadcast to find all peers
        // This ensures we can find all peers, even after restarting
        let username_clone = username.clone();
        println!("@@@ Sending discovery broadcast to find peers...");
        discovery::start_discovery(socket_send_clone.clone(), username_clone, local_addr).await?;

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

    let rl = Arc::new(Mutex::new(DefaultEditor::new()?));

    loop {
        let rl_clone = rl.clone();
        let line_result = task::spawn_blocking(move || {
            let mut rl = rl_clone.blocking_lock();
            rl.readline("")
        })
        .await
        .map_err(|e| {
            rustyline::error::ReadlineError::Io(std::io::Error::other(format!("JoinError: {e}")))
        })?; // handle JoinError (maybe caused by panic etc)

        match line_result {
            Ok(line) => {
                print!("\x1B[1A\x1B[2K");
                std::io::stdout().flush()?;
                if line.starts_with("/") {
                    let peer_list_clone = peer_list.clone();
                    let socket_clone = socket_send_clone.clone();
                    let username_clone = username.clone();
                    if let Some(response) = ui::commands::handle_command(
                        &line,
                        peer_list_clone,
                        Some(socket_clone),
                        Some(username_clone),
                        Some(local_addr),
                    )
                    .await
                    {
                        if response == "exit" {
                            println!("@@@ bye!");
                            break;
                        }
                        println!("{}", response);
                    }
                } else if line.is_empty() {
                    continue;
                } else {
                    let msg = Message::new_chat(username.clone(), line, Some(local_addr));
                    let peers = peer_list.lock().await.get_peers();
                    for peer in &peers {
                        let target_addr = peer.addr.to_string();
                        log::debug!("[Chat] Sending chat message to: {}", target_addr);
                        sender::send_message(socket_send_clone.clone(), &msg, &target_addr).await?;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("@@@ Type [/quit] to exit.");
            }
            Err(ReadlineError::Eof) => {
                println!("@@@ Type [/quit] to exit.");
            }
            Err(err) => {
                println!("Readline error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
