// Hardware information modules for Slowfetch.
// Contains functions hardware, what else did you expect idiot

use std::fs;
use std::process::Command;

use crate::helpers::{create_bar, read_first_line};

/// Get the CPU model name with boost clock.
pub fn cpu() -> String {
    let model = if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        content
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|name| {
                name.split_whitespace()
                    .filter(|&w| !w.ends_with("-Core") && w != "Processor")
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    } else {
        None
    };

    let model = match model {
        Some(m) => m,
        None => return "unknown".to_string(),
    };

    // Get boost clock from cpufreq (in kHz)
    let boost_clock = read_first_line("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        .and_then(|khz_str| khz_str.parse::<u64>().ok())
        .map(|khz| {
            let ghz = khz as f64 / 1_000_000.0;
            format!(" @ {:.2}GHz", ghz)
        })
        .unwrap_or_default();

    format!("{}{}", model, boost_clock)
}

// Get memory usage as a visual bar, 10 blocks = 100% usage
pub fn memory() -> String {
    let mut total = 0;
    let mut available = 0;
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    total = val.parse::<u64>().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    available = val.parse::<u64>().unwrap_or(0);
                }
            }
            if total > 0 && available > 0 {
                break;
            }
        }
    }
    if total > 0 {
        let used = total - available;
        let usage_percent = (used as f64 / total as f64) * 100.0;
        let bar = create_bar(usage_percent);

        // Convert to GiB
        let used_gib = used as f64 / (1024.0 * 1024.0);
        let total_gib = total as f64 / (1024.0 * 1024.0);

        return format!(" {} {:.0}GB󰿟{:.0}GB", bar, used_gib, total_gib);
    }
    "unknown".to_string()
}

// Get the GPU model.
// Uses lspci -mm to get the exact name of the card
pub fn gpu() -> String {
    if let Ok(output) = Command::new("lspci").arg("-mm").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.contains("VGA") && !line.contains("3D") {
                continue;
            }

            let parts: Vec<&str> = line.split('"').collect();
            // 0=Slot, 1=Class, 3=Vendor, 5=Device, 7=SVendor, 9=SDevice
            if parts.len() < 6 {
                continue;
            }

            let vendor_raw = parts.get(3).copied().unwrap_or("");
            let device_raw = parts.get(5).copied().unwrap_or("");
            let s_device_raw = parts.get(9).copied().unwrap_or("");

            // Helper to extract content between brackets: "Name [Model]" -> "Model"
            fn in_brackets(s: &str) -> Option<&str> {
                s.find('[').and_then(|start| {
                    s.rfind(']')
                        .map(|end| if start < end { &s[start + 1..end] } else { s })
                })
            }

            // Vendor: Use content in brackets (first part if slashed), or first word
            let vendor = in_brackets(vendor_raw)
                .and_then(|s| s.split('/').next())
                .unwrap_or_else(|| vendor_raw.split_whitespace().next().unwrap_or(vendor_raw));

            // Device: Parse options from brackets, match against SDevice
            let device_opts = in_brackets(device_raw).unwrap_or(device_raw);
            let best_device = device_opts
                .split('/')
                .filter(|&opt| s_device_raw.contains(opt))
                .last()
                .unwrap_or_else(|| device_opts.split('/').next().unwrap_or(device_raw));

            return format!("{} {}", vendor, best_device).trim().to_string();
        }
    }
    "unknown".to_string()
}

// Get storage usage for all physical disks.
// and hopefully lump em together or something, idk i only have one ssd.
pub fn storage() -> String {
    if let Ok(output) = Command::new("df").arg("-B1").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut total_bytes = 0;
        let mut used_bytes = 0;
        let mut seen_fs = std::collections::HashSet::new();

        // Skip header
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // df output: Filesystem 1B-blocks Used Available ...
            if parts.len() >= 3 {
                let filesystem = parts[0];
                // Filter for real disks: starts with /dev/ and not loop
                if filesystem.starts_with("/dev/") && !filesystem.contains("/loop") {
                    // Avoid double counting if mounted multiple times
                    if seen_fs.insert(filesystem) {
                        let total = parts[1].parse::<u64>().unwrap_or(0);
                        let used = parts[2].parse::<u64>().unwrap_or(0);

                        total_bytes += total;
                        used_bytes += used;
                    }
                }
            }
        }

        if total_bytes > 0 {
            let usage_percent = (used_bytes as f64 / total_bytes as f64) * 100.0;
            let bar = create_bar(usage_percent);

            let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            return format!("{} {:.0}GB󰿟{:.0}GB", bar, used_gb, total_gb);
        }
    }
    "unknown".to_string()
}
