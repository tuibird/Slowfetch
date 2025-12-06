//! Userspace information modules for Slowfetch.
//! Contains functions for shell, packages, and window manager info.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::helpers::{capitalize, get_noctalia_scheme};

/// Get the active shell with version.
pub fn shell() -> String {
    let shell_path = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    let shell_name = shell_path
        .split('/')
        .last()
        .unwrap_or("unknown")
        .to_string();

    if shell_name == "unknown" {
        return shell_name;
    }

    // Try to get version by running shell --version
    let version = Command::new(&shell_path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or("");
            // Extract version number (e.g., "5.2.26" from "bash 5.2.26(1)-release")
            first_line
                .split_whitespace()
                .find(|word| {
                    word.chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                })
                .map(|v| {
                    // Clean up version string (remove trailing parentheses, etc.)
                    v.split(|c: char| c == '(' || c == '-')
                        .next()
                        .unwrap_or(v)
                        .to_string()
                })
        });

    match version {
        Some(v) => format!("{} {}", capitalize(&shell_name), v),
        None => capitalize(&shell_name),
    }
}

/// Get the total number of installed packages.
/// Supports pacman (Arch), dpkg (Debian/Ubuntu), rpm (Fedora/RHEL), and flatpak.
pub fn packages() -> String {
    let mut counts: Vec<String> = Vec::new();

    // Pacman (Arch-based) - count directories in /var/lib/pacman/local/
    if let Ok(entries) = fs::read_dir("/var/lib/pacman/local") {
        let count = entries.filter(|e| e.is_ok()).count();
        if count > 0 {
            counts.push(format!("{} (pacman)", count));
        }
    }

    // dpkg (Debian/Ubuntu) - count lines in /var/lib/dpkg/status with "Status: install ok installed"
    if let Ok(content) = fs::read_to_string("/var/lib/dpkg/status") {
        let count = content
            .lines()
            .filter(|line| line == &"Status: install ok installed")
            .count();
        if count > 0 {
            counts.push(format!("{} (dpkg)", count));
        }
    }

    // RPM (Fedora/RHEL) - check if rpmdb exists
    if Path::new("/var/lib/rpm/rpmdb.sqlite").exists()
        || Path::new("/var/lib/rpm/Packages").exists()
    {
        if let Ok(output) = Command::new("rpm").arg("-qa").output() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            if count > 0 {
                counts.push(format!("{} (rpm)", count));
            }
        }
    }

    // Flatpak - count installed applications
    if let Ok(entries) = fs::read_dir("/var/lib/flatpak/app") {
        let count = entries.filter(|e| e.is_ok()).count();
        if count > 0 {
            counts.push(format!("{} (flatpak)", count));
        }
    }

    if counts.is_empty() {
        "unknown".to_string()
    } else {
        counts.join(", ")
    }
}

/// Get the Window Manager
pub fn wm() -> String {
    let wm_name = if let Ok(wm) = env::var("XDG_CURRENT_DESKTOP") {
        wm
    } else if let Ok(wm) = env::var("DESKTOP_SESSION") {
        wm
    } else {
        "unknown".to_string()
    };
    capitalize(&wm_name)
}

/// Get the active terminal
/// i hope this works with foot, because i aint installing that shit
pub fn terminal() -> String {
    // Check for specific terminal environment variables first
    if env::var("KITTY_PID").is_ok() {
        return "Kitty".to_string();
    }
    if env::var("KONSOLE_VERSION").is_ok() {
        return "Konsole".to_string();
    }
    if env::var("GNOME_TERMINAL_SCREEN").is_ok() {
        return "Gnome Terminal".to_string();
    }

    // Fallback to TERM_PROGRAM or TERM
    let term = env::var("TERM_PROGRAM")
        .unwrap_or_else(|_| env::var("TERM").unwrap_or_else(|_| "unknown".to_string()));

    // Clean up common suffixes like -256color
    let name = term.split("-256color").next().unwrap_or(&term);
    let name = name.split("-color").next().unwrap_or(name);

    capitalize(name)
}

/// Get the active UI/Shell (e.g., Noctalia Shell, Plasma, GNOME)
pub fn ui() -> String {
    // Check for running UI processes
    if let Ok(output) = Command::new("ps").arg("-e").arg("-o").arg("args").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("noctalia-shell") {
            let mut name = "Noctalia Shell".to_string();
            if let Some(scheme) = get_noctalia_scheme() {
                name = format!("{} ({})", name, capitalize(&scheme));
            }
            return name;
        }
        if stdout.contains("plasmashell") {
            return "Plasma".to_string();
        }
        if stdout.contains("gnome-shell") {
            return "Gnome".to_string();
        }
    }

    "unknown".to_string()
}

/// Get the system monospace font using fontconfig
pub fn terminal_font() -> String {
    if let Ok(output) = Command::new("fc-match").arg("monospace").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: filename: "Family" "Style"
        // We want "Family"
        if let Some(start) = stdout.find('"') {
            if let Some(end) = stdout[start + 1..].find('"') {
                return stdout[start + 1..start + 1 + end].to_string();
            }
        }
    }
    "unknown".to_string()
}
