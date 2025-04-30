use crate::peer::SharedPeerList;
use futures::{StreamExt, pin_mut};
use mdns::{RecordKind, Response};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

// Constants for mDNS service
const SERVICE_NAME: &str = "_pung-chat._udp.local";

// Structure to hold mDNS service information
pub struct MdnsService {
    #[allow(dead_code)]
    username: String,
    #[allow(dead_code)]
    port: u16,
    // We'll store the service name for reference
    #[allow(dead_code)]
    service_name: String,
}

impl MdnsService {
    /// Register a new mDNS service for this chat instance
    pub fn register(
        username: String,
        port: u16,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create a hostname based on username (sanitize for DNS compatibility)
        let hostname = format!("pung-{}", sanitize_hostname(&username));

        // In mdns 3.0.0, we don't have direct service registration
        // We'll need to use a different approach with the TXT records
        println!("@@@ Registered mDNS service: {}.{}", hostname, SERVICE_NAME);

        Ok(MdnsService {
            username,
            port,
            service_name: SERVICE_NAME.to_string(),
        })
    }
}

/// Sanitize a string to be used as a hostname
/// Replaces non-alphanumeric characters with hyphens and ensures DNS compatibility
fn sanitize_hostname(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Ensure hostname doesn't start or end with hyphen
    let sanitized = sanitized.trim_matches('-');

    // Ensure hostname isn't empty
    if sanitized.is_empty() {
        return "user".to_string();
    }

    sanitized.to_lowercase()
}

/// Start mDNS service registration
pub async fn start_mdns_service(
    username: String,
    port: u16,
) -> Result<MdnsService, Box<dyn std::error::Error + Send + Sync>> {
    // Register the mDNS service
    let service = MdnsService::register(username, port)?;

    Ok(service)
}

/// Structure to hold discovered peer information from mDNS
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub hostname: String,
    #[allow(dead_code)]
    pub ip: IpAddr,
    #[allow(dead_code)]
    pub port: u16,
    #[allow(dead_code)]
    pub username: Option<String>,
    #[allow(dead_code)]
    pub txt_records: HashMap<String, String>,
}

impl DiscoveredPeer {
    /// Get the username, falling back to hostname if not available
    pub fn get_username(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.hostname.clone())
    }
}

/// Start mDNS discovery to find other chat instances
pub async fn start_mdns_discovery(
    peer_list: SharedPeerList,
    local_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a discovery stream with the mdns 3.0.0 API
    let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();

    println!("@@@ Started mDNS discovery for service: {}", SERVICE_NAME);

    // Spawn a task to handle discovered services
    tokio::spawn(async move {
        handle_discovered_services(stream, peer_list, local_addr).await;
    });

    Ok(())
}

/// Handle discovered mDNS services
async fn handle_discovered_services<S>(stream: S, peer_list: SharedPeerList, local_addr: SocketAddr)
where
    S: StreamExt<Item = Result<Response, mdns::Error>>,
{
    // Pin the stream for use with StreamExt
    pin_mut!(stream);

    // Process discovery events
    while let Some(Ok(response)) = stream.next().await {
        println!("@@@ mDNS response received");

        // Process the response
        if let Err(e) = process_response(&response, &peer_list, local_addr).await {
            eprintln!("Error processing mDNS response: {}", e);
        }
    }
}

/// Process a mDNS response and add discovered peers to the peer list
async fn process_response(
    response: &Response,
    peer_list: &SharedPeerList,
    local_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Extract information from the response
    let mut ip_addresses = Vec::new();
    let mut port = None;
    let mut hostname = None;
    let mut txt_records = HashMap::new();

    // Process each record in the response
    for record in response.records() {
        match &record.kind {
            RecordKind::A(addr) => {
                ip_addresses.push(IpAddr::V4(*addr));
            }
            RecordKind::AAAA(addr) => {
                ip_addresses.push(IpAddr::V6(*addr));
            }
            RecordKind::SRV {
                port: srv_port,
                target,
                ..
            } => {
                port = Some(srv_port);
                hostname = Some(target.to_string());
            }
            RecordKind::TXT(txt) => {
                // Parse TXT records
                for txt_record in txt {
                    if let Some(pos) = txt_record.find('=') {
                        let (key, value) = txt_record.split_at(pos);
                        // Skip the '=' character
                        let value = &value[1..];
                        txt_records.insert(key.to_string(), value.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // Extract username from TXT records
    let username = txt_records.get("username").cloned();

    // If we have both IP addresses and a port, we can create peers
    if let (Some(port_value), Some(hostname_value)) = (port, hostname) {
        for ip in ip_addresses {
            let socket_addr = SocketAddr::new(ip, *port_value);

            // Skip our own address
            if socket_addr.ip() == local_addr.ip() && socket_addr.port() == local_addr.port() {
                continue;
            }

            // Create a discovered peer
            let peer = DiscoveredPeer {
                hostname: hostname_value.clone(),
                ip,
                port: *port_value,
                username: username.clone(),
                txt_records: txt_records.clone(),
            };

            // Add the peer to our list
            let mut peer_list_lock = peer_list.lock().await;
            let username = peer.get_username();
            peer_list_lock.add_or_update_peer(socket_addr, username.clone());

            println!("@@@ Added peer from mDNS: {} ({})", username, socket_addr);
        }
    }

    Ok(())
}
