use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use get_if_addrs::get_if_addrs;
use rand::Rng;
use std::net::IpAddr;

pub fn display_time_from_timestamp(timestamp: i64) -> String {
    // Default to UTC+8 timezone
    display_time_from_timestamp_with_tz(timestamp, 8)
}

pub fn display_time_from_timestamp_with_tz(timestamp: i64, offset_hours: i32) -> String {
    // Create a fixed offset for the specified timezone
    let timezone = FixedOffset::east_opt(offset_hours * 3600).unwrap(); // offset_hours * 3600 seconds

    // First convert to UTC time
    let utc_time: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).unwrap();

    // Then convert to the desired timezone
    let local_time = utc_time.with_timezone(&timezone);

    // Format the time in the local timezone
    local_time.format("%H:%M:%S").to_string()
}

/// Get the local IP address (non-loopback) for the LAN
pub fn get_local_ip() -> Option<IpAddr> {
    match get_if_addrs() {
        Ok(if_addrs) => {
            // First try to find an IPv4 address that's not loopback
            for interface in &if_addrs {
                if !interface.is_loopback() && interface.addr.ip().is_ipv4() {
                    return Some(interface.addr.ip());
                }
            }

            // If no IPv4 found, try IPv6
            for interface in &if_addrs {
                if !interface.is_loopback() && interface.addr.ip().is_ipv6() {
                    return Some(interface.addr.ip());
                }
            }

            None
        }
        Err(_) => None,
    }
}

/// Generate a random port number within the specified range
pub fn get_random_port(min: u16, max: u16) -> u16 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Check if a new version is available on GitHub
/// Returns Some(latest_version) if a newer version is available, None otherwise or on error
pub async fn check_for_updates(current_version: &str) -> Option<String> {
    // GitHub API URL for the latest release
    let url = "https://api.github.com/repos/ktlast/pung/releases/latest";

    // Send request with proper User-Agent header (required by GitHub API)
    match reqwest::Client::new()
        .get(url)
        .header("User-Agent", format!("pung/{}", current_version))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                // Parse the JSON response
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    // Extract the tag_name (version) from the response
                    if let Some(tag_name) = json.get("tag_name").and_then(|v| v.as_str()) {
                        let latest_version = tag_name.trim_start_matches('v');

                        // Compare versions (simple string comparison, assumes semver format)
                        if latest_version != current_version {
                            return Some(latest_version.to_string());
                        }
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}
