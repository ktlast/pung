use crate::VERSION;
use crate::peer::{SharedPeerList, discovery};
use crate::utils;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub async fn handle_command(
    input_line: &str,
    peer_list: SharedPeerList,
    socket: Option<Arc<UdpSocket>>,
    username: Option<String>,
    local_addr: Option<SocketAddr>,
) -> Option<String> {
    // Extract the command part (first word) for matching
    let command = input_line.split_whitespace().next().unwrap_or("");

    match command {
        "/peers" | "/p" => {
            let peers = peer_list.lock().await.get_peers();
            if peers.is_empty() {
                Some("@@@ No peers connected.".to_string())
            } else {
                utils::display_message_block(
                    "Peers",
                    peers
                        .iter()
                        .enumerate() // Add enumeration to get index
                        .map(|(i, peer)| {
                            format!(
                                "{}) {:15} @ {:20} ({}s ago)",
                                i + 1, // Add 1 to make it 1-based instead of 0-based
                                peer.username,
                                peer.addr,
                                peer.last_seen.elapsed().as_secs()
                            )
                        })
                        .collect(),
                );
                None
            }
        }
        "/quit" | "/q" => Some("exit".to_string()),
        "/help" | "/h" => {
            let help_text = "\
        help?
        Available commands:
            /[ p | peers ]           - Show list of connected peers
            /[ b | broadcast ]       - Manually send a discovery broadcast to find peers
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
                    Ok(_) => {
                        Some("@@@ Discovery broadcast sent. Searching for peers...".to_string())
                    }
                    Err(e) => Some(format!("@@@ Failed to send discovery broadcast: {}", e)),
                }
            } else {
                Some("@@@ Cannot send broadcast: missing required parameters".to_string())
            }
        }
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
