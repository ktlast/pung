use crate::MAX_USERNAME_LEN;
use crate::VERSION;
use crate::peer::{SharedPeerList, discovery};
use crate::ui;
use crate::utils;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub async fn handle_command(
    input_line: &str,
    peer_list: SharedPeerList,
    socket: Option<Arc<UdpSocket>>,
    username: Option<String>,
    local_addr: Option<SocketAddr>,
    app_state: Arc<DashMap<&str, String>>,
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
                    "Peers (/p)",
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
            utils::display_message_block("Help? (/h)", vec![
                "Parameters On Startup:".to_string(),
                format!("    -u <username>         ─ Sets the username for chat; max length: {}", MAX_USERNAME_LEN).to_string(),
                "    -r <receive-port>     ─ Sets the port for receiving messages (random if not specified)".to_string(),
                "    -w <width>            ─ Sets the terminal width for message display (default: 80)".to_string(),
                "".to_string(),
                "    Example:".to_string(),
                "        ./pung -u pungman -w 90".to_string(),
                "".to_string(),
                "".to_string(),
                "Available commands:".to_string(),
                "    /[ b | broadcast ]    ─ Manually send a discovery broadcast to find peers".to_string(),
                "    /[ h | help ]         ─ Show this help message".to_string(),
                "    /[ p | peers ]        ─ Show list of connected peers".to_string(),
                "    /[ q | quit ]         ─ Quit the application".to_string(),
                "    /[ s | state ]        ─ Show application state".to_string(),
                "    /[ t | tips ]         ─ Show tips".to_string(),
                "    /[ v | version ]      ─ Show version and check for updates".to_string(),
                "".to_string(),
                "".to_string(),
                "Legend of prefixes:".to_string(),
                "    @@@                   ─ Normal system messages".to_string(),
                "    ###                   ─ Peer related events".to_string(),
            ]);
            None
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
        "/version" | "/v" => {
            // Don't check for updates if we're running from source
            if VERSION != "0.0.0" {
                if let Some(latest_version) = utils::check_for_updates(VERSION).await {
                    let mut new_version_message: Vec<String> = vec![];
                    new_version_message.push("New version available!".to_string());
                    new_version_message
                        .push(format!("- Update: [{}] -> [{}]", VERSION, latest_version));
                    new_version_message.push("".to_string());
                    new_version_message.push("Download the latest version from:".to_string());
                    new_version_message
                        .push("- https://github.com/ktlast/pung/releases/latest".to_string());
                    new_version_message.push("".to_string());
                    new_version_message.push("Or via oneliner:".to_string());
                    new_version_message.push("- bash <(curl -s https://raw.githubusercontent.com/ktlast/pung/master/get-pung.sh)".to_string());
                    utils::display_message_block("New version", new_version_message);
                }
            }
            Some(format!("@@@ Version: {}", VERSION))
        }
        "/tips" | "/t" => {
            ui::app_state::show_tips();
            None
        }
        "/state" | "/s" => {
            ui::app_state::show_static_state(&app_state);
            None
        }
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
