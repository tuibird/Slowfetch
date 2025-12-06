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
// Tries glxinfo first for accurate name, falls back to sysfs + pci.ids
pub fn gpu() -> String {
    // Try glxinfo first, it gives clean gpu names
    if let Some(name) = gpu_from_glxinfo() {
        return name;
    }

    // Fallback to sysfs + pci.ids lookup
    gpu_from_sysfs().unwrap_or_else(|| "unknown".to_string())
}

// Get GPU name from glxinfo (requires X11/Wayland with GL)
fn gpu_from_glxinfo() -> Option<String> {
    let output = Command::new("glxinfo").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("OpenGL renderer") {
            // Format: "OpenGL renderer string: AMD Radeon RX 9070 XT (radeonsi, ...)"
            let renderer = line.split(':').nth(1)?.trim();
            // Remove the parenthetical info if present
            let name = renderer.split('(').next().unwrap_or(renderer).trim();
            if !name.is_empty() && name != "llvmpipe" {
                return Some(name.to_string());
            }
        }
    }
    None
}

// Get GPU name from sysfs + pci.ids database
fn gpu_from_sysfs() -> Option<String> {
    let drm_path = std::path::Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return None;
    }

    // Load pci.ids database
    let pci_ids = fs::read_to_string("/usr/share/hwdata/pci.ids")
        .or_else(|_| fs::read_to_string("/usr/share/misc/pci.ids"))
        .ok()?;

    for entry in fs::read_dir(drm_path).ok()?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Only process card entries, not card0-DP-1 etc
        if !name_str.starts_with("card") || name_str.contains('-') {
            continue;
        }

        let uevent_path = entry.path().join("device/uevent");
        let uevent = fs::read_to_string(&uevent_path).ok()?;

        // Parse PCI_ID from uevent (format: "1002:7550")
        let pci_id_line = uevent.lines().find(|l| l.starts_with("PCI_ID="))?;
        let pci_id = pci_id_line.trim_start_matches("PCI_ID=");
        let parts: Vec<&str> = pci_id.split(':').collect();
        if parts.len() != 2 {
            continue;
        }

        let vendor_id = parts[0].to_lowercase();
        let device_id = parts[1].to_lowercase();

        let vendor_name = lookup_pci_vendor(&pci_ids, &vendor_id);
        let device_name = lookup_pci_device(&pci_ids, &vendor_id, &device_id)?;

        // Extract the part in brackets if present
        let display_name = device_name
            .find('[')
            .and_then(|start| device_name.rfind(']').map(|end| &device_name[start + 1..end]))
            .unwrap_or(&device_name);

        let vendor_short = vendor_name
            .as_deref()
            .and_then(|v| v.find('[').and_then(|start| v.rfind(']').map(|end| &v[start + 1..end])))
            .and_then(|s| s.split('/').next())
            .unwrap_or("GPU");

        return Some(format!("{} {}", vendor_short, display_name));
    }
    None
}

fn lookup_pci_vendor(pci_ids: &str, vendor_id: &str) -> Option<String> {
    for line in pci_ids.lines() {
        if line.starts_with(vendor_id) && line.chars().nth(4) == Some(' ') {
            return Some(line[4..].trim().to_string());
        }
    }
    None
}

fn lookup_pci_device(pci_ids: &str, vendor_id: &str, device_id: &str) -> Option<String> {
    let mut in_vendor = false;
    for line in pci_ids.lines() {
        if line.starts_with(vendor_id) && line.chars().nth(4) == Some(' ') {
            in_vendor = true;
            continue;
        }
        if in_vendor && !line.starts_with('\t') && !line.is_empty() && !line.starts_with('#') {
            break;
        }
        if in_vendor && line.starts_with('\t') && !line.starts_with("\t\t") {
            let trimmed = line.trim_start_matches('\t');
            if trimmed.starts_with(device_id) && trimmed.chars().nth(4) == Some(' ') {
                return Some(trimmed[4..].trim().to_string());
            }
        }
    }
    None
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
