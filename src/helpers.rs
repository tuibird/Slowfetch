// Helper functions

use std::fs;
use crate::fontmodule::{find_font, is_nerd_font};

// Helper to read the first line of a file, yeah ik this dumb dont @ me
pub fn read_first_line(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.to_string()))
}

// Helper to capitalize the first letter of a string.
// No im not importing a crate for this.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// Draw the bar with nerd font icons
pub fn create_bar_pretty(usage_percent: f64) -> String {
    // Calculate filled blocks, 10 blocks = 100%
    let filled_blocks = ((usage_percent / 10.0).round() as usize).min(10);

    if filled_blocks == 0 {
        // Empty bar = Start empty + 9 empty middle + End
        format!("{}", "".repeat(9))
    } else {
        // Filled/Semi-filled = Start filled + (N-1) filled middle + remaining empty + End
        let filled_middle = filled_blocks - 1;
        let empty_middle = 10 - filled_blocks;
        format!(
            "{}{}",
            "".repeat(filled_middle),
            "".repeat(empty_middle)
        )
    }
}

// Draw the bar with regular characters
pub fn create_bar_ascii(usage_percent: f64) -> String {
    // Calculate filled blocks, 10 blocks = 100%
    let filled_blocks = ((usage_percent / 10.0).round() as usize).min(10);
    let empty_blocks = 10 - filled_blocks;

    format!("[{}{}]", "=".repeat(filled_blocks), " ".repeat(empty_blocks))
}

// Draw the bar, auto-selecting style based on font
pub fn create_bar(usage_percent: f64) -> String {
    let font = find_font();
    if is_nerd_font(&font) {
        create_bar_pretty(usage_percent)
    } else {
        create_bar_ascii(usage_percent)
    }
}

// get the current Noctalia color scheme, yeah this one is just for me :P
pub fn get_noctalia_scheme() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/.config/noctalia/settings.json", home);

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            if line.contains("\"predefinedScheme\"") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let value = parts[1].trim();
                    // Remove bullshit
                    let clean_value = value.trim_matches(|c| c == '"' || c == ',' || c == ' ');
                    // Return None for default scheme
                    if clean_value.to_lowercase().contains("default") {
                        return None;
                    }
                    return Some(clean_value.to_string());
                }
            }
        }
    }
    None
}
