// Helper functions

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::sync::OnceLock;

use memchr::{memchr_iter, memmem};

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
        let content = fs::read("/usr/share/hwdata/pci.ids")
            .or_else(|_| fs::read("/usr/share/misc/pci.ids"))
            .ok()?;

        let mut db: PciDatabase = HashMap::new();
        let mut current_vendor_id: Option<String> = None;

        // Use memchr for SIMD-accelerated newline finding
        let mut start = 0;
        for end in memchr_iter(b'\n', &content) {
            let line = &content[start..end];
            start = end + 1;

            // Skip empty lines and comments
            if line.is_empty() || line[0] == b'#' {
                continue;
            }

            // Vendor line: starts with hex digit, no leading tab
            if line[0] != b'\t' && line.len() >= 4 {
                if line[..4].iter().all(|b| b.is_ascii_hexdigit()) {
                    let vendor_id = std::str::from_utf8(&line[..4])
                        .ok()?
                        .to_ascii_lowercase();
                    let vendor_name = std::str::from_utf8(&line[4..])
                        .ok()
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    db.insert(vendor_id.clone(), (vendor_name, HashMap::new()));
                    current_vendor_id = Some(vendor_id);
                }
            }
            // Device line: starts with single tab (not double tab for subsystem)
            else if line[0] == b'\t' && line.get(1) != Some(&b'\t') && line.len() >= 5 {
                if let Some(ref vendor_id) = current_vendor_id {
                    let trimmed = &line[1..]; // Skip the tab
                    if trimmed[..4].iter().all(|b| b.is_ascii_hexdigit()) {
                        let device_id = std::str::from_utf8(&trimmed[..4])
                            .ok()?
                            .to_ascii_lowercase();
                        let device_name = std::str::from_utf8(&trimmed[4..])
                            .ok()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        if let Some((_, devices)) = db.get_mut(vendor_id) {
                            devices.insert(device_id, device_name);
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

    if let Ok(content) = fs::read(&path) {
        // Find "predefinedScheme" using SIMD search
        let needle = b"\"predefinedScheme\"";
        let pos = memmem::find(&content, needle)?;

        // Find the ':' after the key
        let after_key = &content[pos + needle.len()..];
        let colon_pos = memchr::memchr(b':', after_key)?;
        let after_colon = &after_key[colon_pos + 1..];

        // Find the value (between quotes)
        let quote1 = memchr::memchr(b'"', after_colon)?;
        let after_quote1 = &after_colon[quote1 + 1..];
        let quote2 = memchr::memchr(b'"', after_quote1)?;
        let value_bytes = &after_quote1[..quote2];

        let value = std::str::from_utf8(value_bytes).ok()?;

        // Return None for default scheme
        if value.to_lowercase().contains("default") {
            return None;
        }
        return Some(value.to_string());
    }
    None
}

pub fn get_dms_theme() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/.config/DankMaterialShell/settings.json", home);

    // Helper to extract JSON string value after a key using SIMD search
    fn extract_json_value(content: &[u8], key: &[u8]) -> Option<String> {
        let pos = memmem::find(content, key)?;
        let after_key = &content[pos + key.len()..];
        let colon_pos = memchr::memchr(b':', after_key)?;
        let after_colon = &after_key[colon_pos + 1..];
        let quote1 = memchr::memchr(b'"', after_colon)?;
        let after_quote1 = &after_colon[quote1 + 1..];
        let quote2 = memchr::memchr(b'"', after_quote1)?;
        let value_bytes = &after_quote1[..quote2];
        std::str::from_utf8(value_bytes).ok().map(|s| s.to_string())
    }

    if let Ok(content) = fs::read(&path) {
        // Find theme name
        let theme_name = extract_json_value(&content, b"\"currentThemeName\"");

        if let Some(ref name) = theme_name {
            // Return None for default scheme
            if name.to_lowercase().contains("default") {
                return None;
            }

            // If theme is "custom", read the custom theme file for the actual name
            if name.to_lowercase() == "custom" {
                if let Some(custom_path) = extract_json_value(&content, b"\"customThemeFile\"") {
                    if let Ok(custom_content) = fs::read(&custom_path) {
                        // Look for "name" but be careful not to match "currentThemeName"
                        // Search for standalone "name" key
                        if let Some(custom_name) = extract_json_value(&custom_content, b"\"name\"") {
                            return Some(custom_name);
                        }
                    }
                }
            }
        }

        return theme_name;
    }
    None
}
