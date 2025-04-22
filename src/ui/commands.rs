use crate::peer::SharedPeerList;

pub async fn handle_command(input_line: &str, peer_list: SharedPeerList) -> Option<String> {
    match input_line {
        "/peers" => {
            let peers = peer_list.lock().await.get_peers();
            if peers.is_empty() {
                Some("No peers connected.".to_string())
            } else {
                let mut response = String::from("Current peers:\n");
                for (i, peer) in peers.iter().enumerate() {
                    // Convert Instant to a timestamp for display
                    let elapsed = peer.last_seen.elapsed();
                    let seconds_ago = elapsed.as_secs();

                    response.push_str(&format!(
                        "{}. {} at {} (last seen {} seconds ago)\n",
                        i + 1,
                        peer.username,
                        peer.addr,
                        seconds_ago
                    ));
                }
                Some(response)
            }
        }
        "/help" => {
            let help_text = "\
            Available commands:
            /peers - Show list of connected peers
            /help  - Show this help message
            ";
            Some(help_text.to_string())
        }
        _ => {
            if input_line.starts_with("/") {
                Some(format!(
                    "Unknown command: {}. Type /help for available commands.",
                    input_line
                ))
            } else {
                None // Not a command, should be treated as a regular message
            }
        }
    }
}
