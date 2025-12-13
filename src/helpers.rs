// Helper functions

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;
use crate::modules::fontmodule::{find_font, is_nerd_font};

// Cache for font detection - only computed once
static CACHED_FONT: OnceLock<String> = OnceLock::new();
static CACHED_IS_NERD: OnceLock<bool> = OnceLock::new();

fn get_cached_is_nerd_font() -> bool {
    *CACHED_IS_NERD.get_or_init(|| {
        let font = CACHED_FONT.get_or_init(find_font);
        is_nerd_font(font)
    })
}

// Parsed PCI database: vendor_id -> (vendor_name, device_id -> device_name)
pub type PciDatabase = HashMap<String, (String, HashMap<String, String>)>;
static PCI_DB: OnceLock<Option<PciDatabase>> = OnceLock::new();

pub fn get_pci_database() -> &'static Option<PciDatabase> {
    PCI_DB.get_or_init(|| {
        let content = fs::read_to_string("/usr/share/hwdata/pci.ids")
            .or_else(|_| fs::read_to_string("/usr/share/misc/pci.ids"))
            .ok()?;

        let mut db: PciDatabase = HashMap::new();
        let mut current_vendor: Option<(String, String)> = None;

        for line in content.lines() {
            // Skip comments and empty lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // Vendor line: starts with hex digit, no leading whitespace
            if !line.starts_with('\t') && line.len() >= 4 {
                if let Some(vendor_id) = line.get(..4) {
                    if vendor_id.chars().all(|c| c.is_ascii_hexdigit()) {
                        let vendor_name = line.get(4..).map(|s| s.trim().to_string()).unwrap_or_default();
                        current_vendor = Some((vendor_id.to_lowercase(), vendor_name));
                        db.insert(vendor_id.to_lowercase(), (line.get(4..).map(|s| s.trim().to_string()).unwrap_or_default(), HashMap::new()));
                    }
                }
            }
            // Device line: starts with single tab + hex digit
            else if line.starts_with('\t') && !line.starts_with("\t\t") {
                if let Some((vendor_id, _)) = &current_vendor {
                    let trimmed = line.trim_start_matches('\t');
                    if trimmed.len() >= 4 {
                        if let Some(device_id) = trimmed.get(..4) {
                            if device_id.chars().all(|c| c.is_ascii_hexdigit()) {
                                let device_name = trimmed.get(4..).map(|s| s.trim().to_string()).unwrap_or_default();
                                if let Some((_, devices)) = db.get_mut(vendor_id) {
                                    devices.insert(device_id.to_lowercase(), device_name);
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(db)
    })
}

// Helper to read the first line of a file using buffered I/O
// Only reads until first newline instead of entire file
pub fn read_first_line(path: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    // Trim trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Some(line)
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

// Draw the bar, auto-selecting style based on font (cached)
pub fn create_bar(usage_percent: f64) -> String {
    if get_cached_is_nerd_font() {
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

pub fn get_dms_theme() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/.config/DankMaterialShell/settings.json", home);

    if let Ok(content) = fs::read_to_string(&path) {
        let mut theme_name: Option<String> = None;
        let mut custom_theme_file: Option<String> = None;

        for line in content.lines() {
            if line.contains("\"currentThemeName\"") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let value = parts[1].trim();
                    let clean_value = value.trim_matches(|c| c == '"' || c == ',' || c == ' ');
                    if clean_value.to_lowercase().contains("default") {
                        return None;
                    }
                    theme_name = Some(clean_value.to_string());
                }
            }
            if line.contains("\"customThemeFile\"") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let value = parts[1..].join(":");
                    let clean_value = value.trim().trim_matches(|c| c == '"' || c == ',' || c == ' ');
                    custom_theme_file = Some(clean_value.to_string());
                }
            }
        }

        // If theme is "custom", read the custom theme file for the actual name
        if let Some(ref name) = theme_name {
            if name.to_lowercase() == "custom" {
                if let Some(custom_path) = custom_theme_file {
                    if let Ok(custom_content) = fs::read_to_string(&custom_path) {
                        for line in custom_content.lines() {
                            if line.contains("\"name\"") {
                                let parts: Vec<&str> = line.split(':').collect();
                                if parts.len() >= 2 {
                                    let value = parts[1].trim();
                                    let clean_value = value.trim_matches(|c| c == '"' || c == ',' || c == ' ');
                                    return Some(clean_value.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        return theme_name;
    }
    None
}
