use crate::VERSION;
use crate::peer::{SharedPeerList, discovery};
use std::sync::Arc;
use tokio::net::UdpSocket;
use std::net::SocketAddr;

pub async fn handle_command(
    input_line: &str, 
    peer_list: SharedPeerList,
    socket: Option<Arc<UdpSocket>>,
    username: Option<String>,
    local_addr: Option<SocketAddr>
) -> Option<String> {
    // Extract the command part (first word) for matching
    let command = input_line.split_whitespace().next().unwrap_or("");

    match command {
        "/peers" | "/p" => {
            let peers = peer_list.lock().await.get_peers();
            if peers.is_empty() {
                Some("@@@ No peers connected.".to_string())
            } else {
                let mut response = String::from("@@@ Current peers:\n");
                for (i, peer) in peers.iter().enumerate() {
                    // Convert Instant to a timestamp for display
                    let elapsed = peer.last_seen.elapsed();
                    let seconds_ago = elapsed.as_secs();

                    response.push_str(&format!(
                        "@@@   {}. {} at {} (last seen {}s ago)\n",
                        i + 1,
                        peer.username,
                        peer.addr,
                        seconds_ago
                    ));
                }
                Some(response)
            }
        }
        "/remove" | "/rm" => {
            // Parse the index from the command
            let parts: Vec<&str> = input_line.split_whitespace().collect();
            if parts.len() != 2 {
                return Some("@@@ Usage: /remove <index>".to_string());
            }

            // Parse the index
            match parts[1].parse::<usize>() {
                Ok(index) => {
                    // Adjust index to be 0-based (user sees 1-based)
                    let index = index.saturating_sub(1);

                    // Try to remove the peer
                    let mut peer_list_lock = peer_list.lock().await;
                    match peer_list_lock.remove_peer_by_index(index) {
                        Some(peer) => Some(format!(
                            "@@@ Removed peer: {} at {}",
                            peer.username, peer.addr
                        )),
                        None => Some("@@@ Invalid peer index".to_string()),
                    }
                }
                Err(_) => Some("@@@ Invalid index format. Usage: /remove <index>".to_string()),
            }
        }
        "/quit" | "/q" => Some("exit".to_string()),
        "/help" | "/h" => {
            let help_text = "\
        help?
        Available commands:
            /[ p | peers ]           - Show list of connected peers
            /[ b | broadcast ]       - Manually send a discovery broadcast to find peers
            /[ rm | remove ] <index> - Remove a peer by its index
            /[ h | help ]            - Show this help message
            /[ v | version ]         - Show version
            /[ q | quit ]            - Quit the application

        Legend of prefixes:
            @@@                      - Normal system messages
            ###                      - Peer related events
            ";
            Some("@@@ ".to_string() + help_text)
        }
        "/broadcast" | "/b" => {
            // Check if we have all the required parameters
            if let (Some(socket), Some(username), Some(addr)) = (socket, username, local_addr) {
                match discovery::start_discovery(socket, username, addr).await {
                    Ok(_) => Some("@@@ Discovery broadcast sent. Searching for peers...".to_string()),
                    Err(e) => Some(format!("@@@ Failed to send discovery broadcast: {}", e)),
                }
            } else {
                Some("@@@ Cannot send broadcast: missing required parameters".to_string())
            }
        },
        "/version" | "/v" => Some(format!("@@@ Version: {}", VERSION)),
        _ => {
            if input_line.starts_with("/") {
                // Unknown command starting with /
                Some(format!(
                    "@@@ Unknown command: {}. Type /help for available commands.",
                    input_line
                ))
            } else {
                None // Not a command, should be treated as a regular message
            }
        }
    }
}
