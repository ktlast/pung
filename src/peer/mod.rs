pub mod discovery;
pub mod heartbeats;
pub mod peer_list;

// Re-export the peer list types for backward compatibility
pub use peer_list::{PeerList, SharedPeerList};
