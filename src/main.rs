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

#[tokio::main]
async fn main() -> rustyline::Result<()> {
    // Parse command line arguments using clap
    let matches = Command::new("pung")
        .version("1.0")
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
        Some(username) => username.clone(),
        None => {
            let mut bytes = [0u8; 2];
            rand::thread_rng().fill_bytes(&mut bytes);
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

    // We'll broadcast to all common receive ports to ensure all instances receive our messages
    // Each instance will ignore messages from itself based on the message ID

    println!(
        "@@@ Starting pung with username={}, send_port={}, recv_port={}",
        username, send_port, receive_port
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

    // Only bind the receive socket
    let socket_recv = Some(Arc::new(
        UdpSocket::bind(format!("0.0.0.0:{}", receive_port)).await?,
    ));

    let socket_recv_only_for_init =
        Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_RECV_INIT_PORT)).await?);

    // Create a proper socket address with the local IP for peer discovery
    let local_addr = SocketAddr::new(local_ip, receive_port);

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

        let peer_list_clone = peer_list.clone();
        let username_clone = username.clone();
        tokio::spawn(async move {
            if let Err(e) = listener::listen_for_init(
                socket_recv_only_for_init.clone(),
                Some(peer_list_clone),
                Some(username_clone),
                Some(local_addr),
            )
            .await
            {
                eprintln!("Listen for init error: {:?}", e);
            }
        });

        // Start peer discovery
        let username_clone = username.clone();
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

    println!("@@@ To show known peers, type [/peers]");
    let rl = Arc::new(Mutex::new(DefaultEditor::new()?));

    loop {
        let rl_clone = rl.clone();
        let line_result = task::spawn_blocking(move || {
            let mut rl = rl_clone.blocking_lock();
            rl.readline("")
        })
        .await
        .map_err(|e| {
            rustyline::error::ReadlineError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("JoinError: {e}"),
            ))
        })?; // handle JoinError (maybe caused by panic etc)

        match line_result {
            Ok(line) => {
                print!("\x1B[1A\x1B[2K");
                std::io::stdout().flush()?;
                if line.starts_with("/") {
                    let peer_list_clone = peer_list.clone();
                    if let Some(response) =
                        ui::commands::handle_command(&line, peer_list_clone).await
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
