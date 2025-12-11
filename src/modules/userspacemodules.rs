// Userspace/software/whatever information modules for Slowfetch

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;

use crate::helpers::{capitalize, get_dms_theme, get_noctalia_scheme};

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

    // Try to get version by running shell --version, cross your fingers
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
                    // Clean up version string
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

// Get the total number of installed packages.
// Supports pacman aka Arch, hopefully supports debian and fedora but idk, im not setting up a vm to test sorry
pub fn packages() -> String {
    let mut counts: Vec<String> = Vec::new();

    // Spawn flatpak count in separate thread (runs in parallel with other checks)
    // the thinking here most people have packages + flatpacks, so why not just get this out of the way
    let flatpak_handle = thread::spawn(|| {
        fs::read_dir("/var/lib/flatpak/app").ok().and_then(|entries| {
            let count = entries.filter(|e| e.is_ok()).count();
            if count > 0 { Some(format!("  {}", count)) } else { None }
        })
    });

    // Pacman - count directories in /var/lib/pacman/local/
    if let Ok(entries) = fs::read_dir("/var/lib/pacman/local") {
        let count = entries.filter(|e| e.is_ok()).count();
        if count > 0 {
            counts.push(format!("󰮯 {}", count));
        }
    }

    // dpkg (Debian/Ubuntu) - count lines in /var/lib/dpkg/status with "Status: install ok installed"
    if let Ok(content) = fs::read_to_string("/var/lib/dpkg/status") {
        let count = content
            .lines()
            .filter(|line| line == &"Status: install ok installed")
            .count();
        if count > 0 {
            counts.push(format!(" {}", count));
        }
    }

    // RPM check if rpmdb exists
    if Path::new("/var/lib/rpm/rpmdb.sqlite").exists()
        || Path::new("/var/lib/rpm/Packages").exists()
    {
        if let Ok(output) = Command::new("rpm").arg("-qa").output() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            if count > 0 {
                counts.push(format!(" {}", count));
            }
        }
    }

    // Collect flatpak result (maintains display order after rpm)
    if let Some(s) = flatpak_handle.join().ok().flatten() {
        counts.push(s);
    }

    // Nix - count packages in user profile
    if let Some(home) = env::var("HOME").ok() {
        let nix_profile = format!("{}/.nix-profile/manifest.nix", home);
        if Path::new(&nix_profile).exists() {
            // Count packages via nix-env -q
            if let Ok(output) = Command::new("nix-env").arg("-q").output() {
                let count = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty()) //hopefully counting non empty lines
                    .count();
                if count > 0 {
                    counts.push(format!(" {}", count));
                }
            }
        }
    }

    // XBPS (Void Linux) - count directories in /var/db/xbps/
    if Path::new("/var/db/xbps").exists() {
        if let Ok(entries) = fs::read_dir("/var/db/xbps") {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count();
            if count > 0 {
                counts.push(format!(" {}", count));
            }
        }
    }

    if counts.is_empty() {
        "unknown".to_string()
    } else {
        counts.join(" | ")
    }
}

// Get the Window Manager (using /proc instead of subprocess)
pub fn wm() -> String {
    // Known WMs to search for (search term -> display name)
    let wm_list = [
        ("mutter", "Mutter"),
        ("kwin", "KWin"),
        ("sway", "Sway"),
        ("hyprland", "Hyprland"),
        ("Hyprland", "Hyprland"),
        ("river", "River"),
        ("wayfire", "Wayfire"),
        ("labwc", "LabWC"),
        ("dwl", "dwl"),
        ("niri", "Niri"),
        ("openbox", "Openbox"),
        ("i3", "i3"),
        ("bspwm", "bspwm"),
        ("dwm", "dwm"),
        ("awesome", "Awesome"),
        ("xfwm4", "Xfwm4"),
        ("marco", "Marco"),
        ("metacity", "Metacity"),
        ("compiz", "Compiz"),
        ("enlightenment", "Enlightenment"),
        ("fluxbox", "Fluxbox"),
        ("icewm", "IceWM"),
        ("xmonad", "XMonad"),
        ("qtile", "Qtile"),
        ("herbstluftwm", "herbstluftwm"),
        ("weston", "Weston"),
        ("cage", "Cage"),
        ("gamescope", "Gamescope"),
    ];

    // Read /proc directly instead of spawning ps | grep (saves 3ish ms)
    let proc_path = Path::new("/proc");
    if let Ok(entries) = fs::read_dir(proc_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            // Only check numeric directories (PIDs)
            if !name.to_string_lossy().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                continue;
            }

            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                for (wm_search, wm_display) in &wm_list {
                    if cmdline.contains(wm_search) {
                        return wm_display.to_string();
                    }
                }
            }
        }
    }

    // Fallback to environment variables
    if let Ok(wm) = env::var("XDG_CURRENT_DESKTOP") {
        return capitalize(&wm);
    }
    if let Ok(wm) = env::var("DESKTOP_SESSION") {
        return capitalize(&wm);
    }

    "unknown".to_string()
}

// Get the active terminal
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

// Get the active UI/Shell, i dont know what to call this shit because i already used shell for the terminal shell
pub fn ui() -> String {
    // Read /proc directly instead of spawning ps, saves like 3-4ms
    let proc_path = Path::new("/proc");
    if let Ok(entries) = fs::read_dir(proc_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            // Only check numeric directories (PIDs)
            if !name.to_string_lossy().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                continue;
            }

            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                if cmdline.contains("noctalia-shell") {
                    let mut name = "Noctalia Shell".to_string();
                    if let Some(scheme) = get_noctalia_scheme() {
                        name = format!("{} |  {}", name, capitalize(&scheme));
                    }
                    return name;
                }
                if cmdline.contains("dms") {
                    let mut name = "DMS".to_string();
                    if let Some(theme) = get_dms_theme() {
                        let formatted_theme = theme
                            .replace("cat-", "Catppuccin (")
                            + if theme.starts_with("cat-") { ")" } else { "" };
                        name = format!("{} |  {}", name, capitalize(&formatted_theme));
                    }
                    return name;
                }

                //i know this janky but idk
                if cmdline.contains("plasmashell") {
                    return "Plasma Shell".to_string();
                }
                if cmdline.contains("gnome-shell") {
                    return "Gnome Shell".to_string();
                }
                if cmdline.contains("waybar") {
                    return "Custom Waybar setup".to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

// Get the user's preferred editor from environment variables.
// Returns empty string if unset or set to nano (dont @ me)
pub fn editor() -> String {
    let visual = env::var("VISUAL").ok();
    let editor = env::var("EDITOR").ok();

    // Helper to extract and format editor name
    let format_editor = |path: &str| -> Option<String> {
        let name = path.split('/').last().unwrap_or(path);
        if name == "nano" {
            None
        } else {
            Some(capitalize(name))
        }
    };

    match (visual.as_deref().and_then(format_editor), editor.as_deref().and_then(format_editor)) {
        (Some(v), Some(e)) if v != e => format!("󰍹 {} |  {}", v, e),
        (Some(v), _) => v,
        (None, Some(e)) => e,
        (None, None) => String::new()
    }
}
