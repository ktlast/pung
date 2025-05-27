use crate::utils;
use dashmap::DashMap;

pub fn show_static_state(app_state: &DashMap<&str, String>) {
    // Collect entries, sort by key, then format
    let mut static_entries: Vec<_> = app_state
        .iter()
        .filter(|entry| entry.key().starts_with("static:"))
        .collect();

    // Sort by key
    static_entries.sort_by(|a, b| a.key().cmp(b.key()));

    // Format the sorted entries
    let static_settings: Vec<_> = static_entries
        .into_iter()
        .map(|entry| {
            format!(
                "{:15} = {}",
                entry
                    .key()
                    .replace("static:", "")
                    .split("_")
                    .collect::<Vec<_>>()
                    .join(" "),
                entry.value()
            )
        })
        .collect();

    utils::display_message_block("State", static_settings);
}

pub fn show_tips() {
    let startup_message: Vec<String> = vec![
        "1) use [/h] to show available commands".to_string(),
        "2) use [/v] to show version and check for updates".to_string(),
    ];
    utils::display_message_block("Tips", startup_message);
}
