// Configuration loader for Slowfetch
// Loads settings from config.toml

use std::fs;
use std::path::PathBuf;

// Embed the default config file at compile time
const DEFAULT_CONFIG: &str = include_str!("config.toml");

// OS art setting - can be disabled, auto-detect, or specific OS
#[derive(Debug, Clone)]
pub enum OsArtSetting {
    Disabled,
    Auto,
    Specific(String),
}

// Color configuration - all colors stored as RGB tuples
#[derive(Debug, Clone)]
pub struct ColorConfig {
    // Theme colors
    pub border: (u8, u8, u8),
    pub title: (u8, u8, u8),
    pub key: (u8, u8, u8),
    pub value: (u8, u8, u8),
    // ASCII art colors (1-9)
    pub art_1: (u8, u8, u8),
    pub art_2: (u8, u8, u8),
    pub art_3: (u8, u8, u8),
    pub art_4: (u8, u8, u8),
    pub art_5: (u8, u8, u8),
    pub art_6: (u8, u8, u8),
    pub art_7: (u8, u8, u8),
    pub art_8: (u8, u8, u8),
    pub art_9: (u8, u8, u8),
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            // Default theme colors (Dracula-inspired)
            border: (0xFF, 0x79, 0xC6), // #FF79C6 - magenta/pink
            title: (0xFF, 0x79, 0xC6),  // #FF79C6 - magenta/pink
            key: (0xBD, 0x93, 0xF9),    // #BD93F9 - purple
            value: (0x8B, 0xE9, 0xFD),  // #8BE9FD - cyan
            // Default art colors (rainbow spectrum)
            art_1: (0xFF, 0x00, 0x00), // #FF0000 - Red
            art_2: (0xFF, 0x80, 0x00), // #FF8000 - Orange
            art_3: (0xFF, 0xFF, 0x00), // #FFFF00 - Yellow
            art_4: (0x00, 0xFF, 0x00), // #00FF00 - Green
            art_5: (0x00, 0xFF, 0xFF), // #00FFFF - Cyan
            art_6: (0x00, 0xBF, 0xFF), // #00BFFF - Light Blue
            art_7: (0x55, 0x55, 0xFF), // #5555FF - Blue
            art_8: (0xAA, 0x55, 0xFF), // #AA55FF - Violet
            art_9: (0xFF, 0x55, 0xFF), // #FF55FF - Magenta
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub os_art: OsArtSetting,
    pub colors: ColorConfig,
    pub custom_art: Option<String>,
    pub image: bool,
    pub image_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            os_art: OsArtSetting::Disabled,
            colors: ColorConfig::default(),
            custom_art: None,
            image: false,
            image_path: None,
        }
    }
}

// Parse a hex color string like "#FF79C6" or "FF79C6" into RGB tuple
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim().trim_matches('"');
    let hex = hex.strip_prefix('#').unwrap_or(hex);

    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some((r, g, b))
}

// Get the config file path, checking common locations
fn get_config_path() -> Option<PathBuf> {
    // Check XDG_CONFIG_HOME/slowfetch/config.toml first
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg_config).join("slowfetch/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    // Check ~/.config/slowfetch/config.toml
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(&home).join(".config/slowfetch/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    // Check config.toml in current directory (for development)
    let local_path = PathBuf::from("config.toml");
    if local_path.exists() {
        return Some(local_path);
    }

    None
}

// Load configuration from file
pub fn load_config() -> Config {
    let path = match get_config_path() {
        Some(p) => p,
        None => return Config::default(),
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };

    parse_config(&content)
}

// Parse the TOML config content
fn parse_config(content: &str) -> Config {
    let mut config = Config::default();
    let mut in_colors_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Track which section we're in
        if line.starts_with('[') {
            in_colors_section = line == "[colors]";
            continue;
        }

        // Parse color settings
        if in_colors_section {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                if let Some(color) = parse_hex_color(value) {
                    match key {
                        "border" => config.colors.border = color,
                        "title" => config.colors.title = color,
                        "key" => config.colors.key = color,
                        "value" => config.colors.value = color,
                        "art_1" => config.colors.art_1 = color,
                        "art_2" => config.colors.art_2 = color,
                        "art_3" => config.colors.art_3 = color,
                        "art_4" => config.colors.art_4 = color,
                        "art_5" => config.colors.art_5 = color,
                        "art_6" => config.colors.art_6 = color,
                        "art_7" => config.colors.art_7 = color,
                        "art_8" => config.colors.art_8 = color,
                        "art_9" => config.colors.art_9 = color,
                        _ => {}
                    }
                }
            }
            continue;
        }

        // Parse os_art setting
        if line.starts_with("os_art") {
            if let Some(value) = line.split('=').nth(1) {
                let value = value.trim();

                if value == "true" {
                    config.os_art = OsArtSetting::Auto;
                } else if value == "false" {
                    config.os_art = OsArtSetting::Disabled;
                } else if value.starts_with('"') && value.ends_with('"') {
                    // Extract string value between quotes
                    let os_name = value.trim_matches('"').to_string();
                    if !os_name.is_empty() {
                        config.os_art = OsArtSetting::Specific(os_name);
                    }
                }
            }
        }

        // Parse custom_art setting
        if line.starts_with("custom_art") {
            if let Some(value) = line.split('=').nth(1) {
                let value = value.trim();
                if value.starts_with('"') && value.ends_with('"') {
                    let path = value.trim_matches('"').to_string();
                    if !path.is_empty() {
                        // Expand ~ to home directory
                        let expanded_path = if path.starts_with("~/") {
                            if let Ok(home) = std::env::var("HOME") {
                                path.replacen("~", &home, 1)
                            } else {
                                path
                            }
                        } else {
                            path
                        };
                        config.custom_art = Some(expanded_path);
                    }
                }
            }
        }

        // Parse image toggle
        if line.starts_with("image") && !line.starts_with("image_path") {
            if let Some(value) = line.split('=').nth(1) {
                let value = value.trim();
                config.image = value == "true";
            }
        }

        // Parse image_path setting
        if line.starts_with("image_path") {
            if let Some(value) = line.split('=').nth(1) {
                let value = value.trim();
                if value.starts_with('"') && value.ends_with('"') {
                    let path = value.trim_matches('"').to_string();
                    if !path.is_empty() {
                        // Expand ~ to home directory
                        let expanded_path = if path.starts_with("~/") {
                            if let Ok(home) = std::env::var("HOME") {
                                path.replacen("~", &home, 1)
                            } else {
                                path
                            }
                        } else {
                            path
                        };
                        config.image_path = Some(expanded_path);
                    }
                }
            }
        }
    }

    config
}
