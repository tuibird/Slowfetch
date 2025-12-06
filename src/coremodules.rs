//! Core system information modules for Slowfetch.
//! Contains functions for basic system info: user, host, OS, kernel, uptime.

use std::fs;

use crate::helpers::read_first_line;

/// Get the OS name from /etc/os-release.
pub fn os() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line
                    .trim_start_matches("PRETTY_NAME=")
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
            }
        }
    }
    "Linux".to_string()
}

/// Get the kernel version.
pub fn kernel() -> String {
    read_first_line("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".to_string())
}

/// Get the system uptime.
pub fn uptime() -> String {
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(seconds_str) = content.split_whitespace().next() {
            if let Ok(seconds) = seconds_str.parse::<f64>() {
                let s = seconds as u64;
                let h = s / 3600;
                let m = (s % 3600) / 60;
                if h > 0 {
                    return format!("{}h {}m", h, m);
                } else {
                    return format!("{}m", m);
                }
            }
        }
    }
    "unknown".to_string()
}
