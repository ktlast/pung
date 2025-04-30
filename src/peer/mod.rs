pub mod discovery;
pub mod heartbeats;
pub mod mdns_discovery;

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
}

impl PeerList {
    pub fn new() -> Self {
        PeerList {
            peers: HashMap::new(),
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

        // Check if we already have this exact peer
        if let Some(existing_peer) = self.peers.get_mut(&key) {
            // Just update the last_seen time
            existing_peer.last_seen = Instant::now();
        } else {
            // Check if we have another peer with the same address
            let addr_exists = self.peers.values().any(|peer| peer.addr == addr);

            // If the address exists, update that peer instead of creating a new one
            if addr_exists {
                // Find and remove the old peer entry
                let old_key = self
                    .peers
                    .iter()
                    .find(|(_, peer)| peer.addr == addr)
                    .map(|(key, _)| key.clone());

                if let Some(old_key) = old_key {
                    self.peers.remove(&old_key);
                }
            }

            // Add the new peer
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

    pub fn update_last_seen(&mut self, addr: &SocketAddr) -> bool {
        // Find peer by address
        let peer_key = self
            .peers
            .iter()
            .find(|(_, peer)| &peer.addr == addr)
            .map(|(key, _)| key.clone());

        if let Some(key) = peer_key {
            if let Some(peer) = self.peers.get_mut(&key) {
                peer.last_seen = Instant::now();
                return true;
            }
        }
        false
    }

    // Find a peer by address and return its username if found
    pub fn find_username_by_addr(&self, addr: &SocketAddr) -> Option<String> {
        for peer in self.peers.values() {
            if &peer.addr == addr {
                return Some(peer.username.clone());
            }
        }
        None
    }

    pub fn remove_stale_peers(&mut self, timeout: Duration) -> Vec<String> {
        let now = Instant::now();
        let stale_peers: Vec<String> = self
            .peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > timeout)
            .map(|(username, _)| username.clone())
            .collect();

        for username in &stale_peers {
            self.peers.remove(username);
        }

        stale_peers
    }

    /// Consolidate duplicate users with the same username and IP
    /// This helps clean up the peer list when users restart their application
    /// and get assigned a new port
    pub fn consolidate_duplicate_users(&mut self) {
        // First, collect all peers by username and IP
        let mut user_groups: HashMap<(String, std::net::IpAddr), Vec<String>> = HashMap::new();

        // Group peers by username and IP
        for (key, peer) in &self.peers {
            let username = peer.username.clone();
            let ip = peer.addr.ip();
            let entry = user_groups.entry((username, ip)).or_default();
            entry.push(key.clone());
        }

        // For each group with more than one entry, keep only the most recently seen
        for ((_username, _ip), keys) in user_groups {
            if keys.len() > 1 {
                // Find the most recently seen peer in this group
                let mut most_recent_key = keys[0].clone();
                let mut most_recent_time = Instant::now() - Duration::from_secs(9999); // Very old time

                for key in &keys {
                    if let Some(peer) = self.peers.get(key) {
                        if peer.last_seen > most_recent_time {
                            most_recent_time = peer.last_seen;
                            most_recent_key = key.clone();
                        }
                    }
                }

                // Remove all but the most recent peer
                for key in keys {
                    if key != most_recent_key {
                        if let Some(peer) = self.peers.remove(&key) {
                            println!(
                                "@@@ Consolidated duplicate peer: {} at {}",
                                peer.username, peer.addr
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn remove_peer_by_index(&mut self, index: usize) -> Option<PeerInfo> {
        // Get all peers as a vector
        let peers = self.get_peers();

        // Check if the index is valid
        if index < peers.len() {
            // Get the address and username of the peer at the specified index
            let addr = peers[index].addr;
            let username = peers[index].username.clone();

            // Generate the key for this peer
            let key = Self::generate_peer_key(&username, &addr);

            // Remove the peer from the HashMap and return it
            self.peers.remove(&key)
        } else {
            None
        }
    }
}

// Create a thread-safe shared PeerList
pub type SharedPeerList = Arc<Mutex<PeerList>>;
