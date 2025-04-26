pub mod discovery;
pub mod heartbeats;

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
    peers: HashMap<String, PeerInfo>,
}

impl PeerList {
    pub fn new() -> Self {
        PeerList {
            peers: HashMap::new(),
        }
    }

    pub fn add_or_update_peer(&mut self, addr: SocketAddr, username: String) {
        // If username is empty or just an IP address, generate a better name
        let username = if username.is_empty() || username.contains(':') {
            format!("anonymous@{}", addr)
        } else {
            username
        };

        // Update existing peer if username exists, otherwise add new
        if let Some(existing_peer) = self.peers.get_mut(&username) {
            existing_peer.addr = addr;
            existing_peer.last_seen = Instant::now();
        } else {
            self.peers.insert(
                username.clone(),
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
        for peer in self.peers.values_mut() {
            if &peer.addr == addr {
                peer.last_seen = Instant::now();
                return true;
            }
        }
        false
    }

    // Find a peer by address and return its username if found
    pub fn find_username_by_addr(&self, addr: &SocketAddr) -> Option<String> {
        for (username, peer) in &self.peers {
            if &peer.addr == addr {
                return Some(username.clone());
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
}

// Create a thread-safe shared PeerList
pub type SharedPeerList = Arc<Mutex<PeerList>>;
