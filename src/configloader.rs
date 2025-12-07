// Configuration loader for Slowfetch
// Loads settings from config.toml

use std::fs;
use std::path::PathBuf;

/// OS art setting - can be disabled, auto-detect, or specific OS
#[derive(Debug, Clone)]
pub enum OsArtSetting {
    Disabled,
    Auto,
    Specific(String),
}

#[derive(Debug)]
pub struct Config {
    pub os_art: OsArtSetting,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            os_art: OsArtSetting::Disabled,
        }
    }
}

/// Get the config file path, checking common locations
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

/// Load configuration from file
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

/// Parse the TOML config content
fn parse_config(content: &str) -> Config {
    let mut config = Config::default();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
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
    }

    config
}
