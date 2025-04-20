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
