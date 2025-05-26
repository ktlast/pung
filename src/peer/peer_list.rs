use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// Peer information structure
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: SocketAddr,
    pub username: String,
    pub last_seen: Instant,
}

// PeerList to track all known peers
#[derive(Debug, Clone)]
pub struct PeerList {
    // Use a combination of username and address as the key to prevent username conflicts
    peers: HashMap<String, PeerInfo>,
    // Track recently removed peers to prevent zombie peers from being re-added
    // The key is the socket address as a string, and the value is the time when the peer was removed
    recently_removed: HashMap<String, Instant>,
}

impl PeerList {
    pub fn new() -> Self {
        PeerList {
            peers: HashMap::new(),
            recently_removed: HashMap::new(),
        }
    }

    // Generate a unique key for a peer based on username and address
    fn generate_peer_key(username: &str, addr: &SocketAddr) -> String {
        format!("{}@{}", username, addr)
    }

    pub fn add_or_update_peer(&mut self, addr: SocketAddr, username: String) {
        // If username is empty or just an IP address, generate a better name
        let username = if username.is_empty() || username.contains(':') {
            format!("anonymous@{}", addr)
        } else {
            username
        };

        // Don't add new anonymous peers from other instances
        // Only update existing ones or add non-anonymous peers
        if username.starts_with("anonymous@") {
            // Check if this peer already exists
            let existing = self.peers.values().any(|peer| peer.addr == addr);
            if !existing {
                // Skip adding new anonymous peers
                return;
            }
        }

        // Generate a unique key for this peer
        let key = Self::generate_peer_key(&username, &addr);

        // Check if we already have this exact peer (by username and address)
        if let Some(existing_peer) = self.peers.get_mut(&key) {
            // Just update the last_seen time
            existing_peer.last_seen = Instant::now();
        } else {
            // Add the new peer (do NOT merge or remove by address only)
            self.peers.insert(
                key,
                PeerInfo {
                    addr,
                    username,
                    last_seen: Instant::now(),
                },
            );
        }
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }

    // Find a peer by EXACT address (including port) and return its username if found
    pub fn find_username_by_addr(&self, addr: &SocketAddr) -> Option<String> {
        for peer in self.peers.values() {
            // Only match if the FULL address (IP AND port) matches
            if peer.addr.ip() == addr.ip() && peer.addr.port() == addr.port() {
                return Some(peer.username.clone());
            }
        }
        None
    }

    pub fn remove_stale_peers(&mut self, timeout: Duration) -> Vec<String> {
        let now = Instant::now();
        let stale_peers: Vec<(String, SocketAddr)> = self
            .peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > timeout)
            .map(|(username, info)| (username.clone(), info.addr))
            .collect();

        for (username, addr) in &stale_peers {
            self.peers.remove(username);
            // Add to recently removed peers
            self.recently_removed.insert(addr.to_string(), now);
        }

        // Return just the usernames for backward compatibility
        stale_peers
            .into_iter()
            .map(|(username, _)| username)
            .collect()
    }

    // Check if a peer was recently removed (within the grace period)
    pub fn was_recently_removed(&self, addr: &SocketAddr, grace_period: Duration) -> bool {
        if let Some(removed_time) = self.recently_removed.get(&addr.to_string()) {
            let now = Instant::now();
            return now.duration_since(*removed_time) < grace_period;
        }
        false
    }

    // Clean up old entries from the recently_removed list
    pub fn clean_removed_list(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.recently_removed
            .retain(|_, removed_time| now.duration_since(*removed_time) < max_age);
    }
}

// Create a thread-safe shared PeerList
pub type SharedPeerList = Arc<Mutex<PeerList>>;
