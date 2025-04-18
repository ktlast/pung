use chrono::{DateTime, TimeZone, Utc, FixedOffset};

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