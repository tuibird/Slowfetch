// Userspace/software/whatever information modules for Slowfetch

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::helpers::{capitalize, get_dms_theme, get_noctalia_scheme};

/// Get the active shell with version.
pub fn shell() -> String {
    let shell_path = match env::var("SHELL") {
        Ok(p) => p,
        Err(_) => return "unknown".to_string(),
    };

    let shell_name = match shell_path.rsplit('/').next() {
        Some(name) if !name.is_empty() => name,
        _ => return "unknown".to_string(),
    };

    // Try to get version by running shell --version
    let version = Command::new(&shell_path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            // Find first line directly in bytes to avoid full UTF-8 conversion
            let stdout = &output.stdout;
            let first_line_end = stdout.iter().position(|&b| b == b'\n').unwrap_or(stdout.len());
            let first_line = std::str::from_utf8(&stdout[..first_line_end]).ok()?;

            // Extract version number (e.g., "5.2.26" from "bash 5.2.26(1)-release")
            first_line
                .split_ascii_whitespace()
                .find(|word| word.as_bytes().first().map_or(false, |b| b.is_ascii_digit()))
                .map(|v| {
                    // Clean up version string - find first ( or -
                    let end = v.find(|c: char| c == '(' || c == '-').unwrap_or(v.len());
                    v[..end].to_string()
                })
        });

    match version {
        Some(v) => format!("{} {}", capitalize(shell_name), v),
        None => capitalize(shell_name),
    }
}

// Get the total number of installed packages.
// Supports pacman aka Arch, hopefully supports debian and fedora but idk, im not setting up a vm to test sorry
pub fn packages() -> String {
    let mut counts: Vec<String> = Vec::with_capacity(4);

    // Pacman - count directories in /var/lib/pacman/local/
    if let Ok(entries) = fs::read_dir("/var/lib/pacman/local") {
        let count = entries.filter(|e| e.is_ok()).count();
        if count > 0 {
            counts.push(format!("󰮯 {}", count));
        }
    }

    // dpkg (Debian/Ubuntu) - count occurrences of status line using byte search
    if let Ok(content) = fs::read("/var/lib/dpkg/status") {
        const NEEDLE: &[u8] = b"\nStatus: install ok installed\n";
        let count = content.windows(NEEDLE.len()).filter(|w| *w == NEEDLE).count();
        if count > 0 {
            counts.push(format!(" {}", count));
        }
    }

    // RPM check if rpmdb exists
    if Path::new("/var/lib/rpm/rpmdb.sqlite").exists()
        || Path::new("/var/lib/rpm/Packages").exists()
    {
        if let Ok(output) = Command::new("rpm").arg("-qa").output() {
            // Count newlines in bytes directly - faster than UTF-8 conversion
            let count = output.stdout.iter().filter(|&&b| b == b'\n').count();
            if count > 0 {
                counts.push(format!(" {}", count));
            }
        }
    }

    // Flatpak - count installed applications
    if let Ok(entries) = fs::read_dir("/var/lib/flatpak/app") {
        let count = entries.filter(|e| e.is_ok()).count();
        if count > 0 {
            counts.push(format!("  {}", count));
        }
    }

    // Nix - count packages in user profile
    if let Ok(home) = env::var("HOME") {
        let nix_profile = format!("{}/.nix-profile/manifest.nix", home);
        if Path::new(&nix_profile).exists() {
            // Count packages via nix-env -q
            if let Ok(output) = Command::new("nix-env").arg("-q").output() {
                // Count non-empty lines via byte splitting
                let count = output.stdout.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count();
                if count > 0 {
                    counts.push(format!(" {}", count));
                }
            }
        }
    }

    // XBPS (Void Linux) - count directories in /var/db/xbps/
    if let Ok(entries) = fs::read_dir("/var/db/xbps") {
        let count = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |ft| ft.is_dir()))
            .count();
        if count > 0 {
            counts.push(format!(" {}", count));
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
    // Check environment variables first - much faster than /proc scan
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        // Map common desktop values to their WM names
        let wm = match desktop.to_lowercase().as_str() {
            "hyprland" => "Hyprland",
            "sway" => "Sway",
            "kde" | "plasma" => "KWin",
            "gnome" => "Mutter",
            "xfce" => "Xfwm4",
            "i3" => "i3",
            "bspwm" => "bspwm",
            "awesome" => "Awesome",
            "qtile" => "Qtile",
            "niri" => "Niri",
            _ => return desktop,
        };
        return wm.to_string();
    }

    if let Ok(session) = env::var("DESKTOP_SESSION") {
        return capitalize(&session);
    }

    // Fallback: scan /proc for WM processes
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

    // Read /proc directly instead of spawning ps | grep (saves 0.3ish ms)
    let proc_path = Path::new("/proc");
    if let Ok(entries) = fs::read_dir(proc_path) {
        for entry in entries.flatten() {
            // Fast check: first byte must be a digit (PID directories)
            let name = entry.file_name();
            let name_bytes = name.as_encoded_bytes();
            if name_bytes.is_empty() || !name_bytes[0].is_ascii_digit() {
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
    // Fast path: check env vars for common desktop shells
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        match desktop.to_lowercase().as_str() {
            "kde" | "plasma" => return "Plasma Shell".to_string(),
            "gnome" => return "Gnome Shell".to_string(),
            _ => {}
        }
    }

    // Scan /proc for custom shells (noctalia, dms, waybar) - i really dont want to do this but i cant think of another way rn
    let proc_path = Path::new("/proc");
    if let Ok(entries) = fs::read_dir(proc_path) {
        for entry in entries.flatten() {
            // Fast check: first byte must be a digit (PID directories)
            let name = entry.file_name();
            let name_bytes = name.as_encoded_bytes();
            if name_bytes.is_empty() || !name_bytes[0].is_ascii_digit() {
                continue;
            }

            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                if cmdline.contains("noctalia-shell") {
                    let mut name = "Noctalia Shell".to_string();
                    if let Some(scheme) = get_noctalia_scheme() {
                        name = format!("{} |  {}", name, capitalize(&scheme));
                    }
                    return name;
                }
                if cmdline.contains("dms") {
                    let mut name = "DMS".to_string();
                    if let Some(theme) = get_dms_theme() {
                        let formatted_theme = theme
                            .replace("cat-", "Catppuccin (")
                            + if theme.starts_with("cat-") { ")" } else { "" };
                        name = format!("{} |  {}", name, capitalize(&formatted_theme));
                    }
                    return name;
                }

                //i know this janky but idk, its a fallback
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
        (Some(v), Some(e)) if v != e => format!("󰍹 {} |  {}", v, e),
        (Some(v), _) => v,
        (None, Some(e)) => e,
        (None, None) => String::new()
    }
}
