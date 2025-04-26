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
    peers: HashMap<SocketAddr, PeerInfo>,
}

impl PeerList {
    pub fn new() -> Self {
        PeerList {
            peers: HashMap::new(),
        }
    }

    pub fn add_or_update_peer(&mut self, addr: SocketAddr, username: String) {
        self.peers.insert(
            addr,
            PeerInfo {
                addr,
                username,
                last_seen: Instant::now(),
            },
        );
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }

    pub fn update_last_seen(&mut self, addr: &SocketAddr) -> bool {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.last_seen = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn remove_stale_peers(&mut self, timeout: Duration) -> Vec<SocketAddr> {
        let now = Instant::now();
        let stale_peers: Vec<SocketAddr> = self
            .peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > timeout)
            .map(|(addr, _)| *addr)
            .collect();

        for addr in &stale_peers {
            self.peers.remove(addr);
        }

        stale_peers
    }
}

// Create a thread-safe shared PeerList
pub type SharedPeerList = Arc<Mutex<PeerList>>;
