use crate::peer::SharedPeerList;

pub async fn handle_command(input_line: &str, peer_list: SharedPeerList) -> Option<String> {
    // Extract the command part (first word) for matching
    let command = input_line.split_whitespace().next().unwrap_or("");

    match command {
        "/peers" => {
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
        "/remove" => {
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
        "/help" => {
            let help_text = "\
            Available commands:
            /peers - Show list of connected peers
            /remove <index> - Remove a peer by its index
            /help  - Show this help message
            ";
            Some("@@@ ".to_string() + help_text)
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
