// Hardware information modules for Slowfetch.
// Contains functions hardware, what else did you expect idiot

use std::fs;
use std::process::Command;

use crate::cache;
use crate::helpers::{create_bar, get_pci_database, read_first_line};

// Get the CPU model name with boost clock.
pub fn cpu() -> String {
    let model = if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        content
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|name| {
                let words: Vec<&str> = name.split_whitespace().collect();
                // Find where GPU info starts (e.g., "with Radeon Graphics", "w/ Intel UHD")
                let gpu_start = words.iter().position(|&w| {
                    w.eq_ignore_ascii_case("with") || w.eq_ignore_ascii_case("w/")
                });
                let words = match gpu_start {
                    Some(idx) => &words[..idx],
                    None => &words[..],
                };
                words
                    .iter()
                    .filter(|&&w| !w.ends_with("-Core") && w != "Processor")
                    .copied()
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

        // Convert to GB (decimal: 1 KB = 1000 bytes, meminfo reports in KB)
        let used_gb = used as f64 / 1_000_000.0;
        let total_gb = total as f64 / 1_000_000.0;

        return format!(" {} {:.0}GB/{:.0}GB", bar, used_gb, total_gb);
    }
    "unknown".to_string()
}

// Get the GPU model.
// Uses persistent cache to avoid slow subprocess calls on repeated runs.
// If cache isnt used, it tries vulkaninfo first for speed, then glxinfo, then sysfs + pci.ids, then lspci as final fallback
pub fn gpu() -> String {
    // Check cache first (unless --refresh was passed)
    if let Some(cached) = cache::get_cached_gpu() {
        return cached;
    }

    // No cache hit, fetch fresh value
    let result = gpu_fresh();

    // Cache the result for next time
    cache::cache_gpu(&result);

    result
}

// Fetch GPU info fresh (no cache)
fn gpu_fresh() -> String {
    // Try vulkaninfo first - fastest option (~19ms)
    if let Some(name) = gpu_from_vulkaninfo() {
        return name;
    }

    // Try glxinfo as fallback (~52ms)
    if let Some(name) = gpu_from_glxinfo() {
        return name;
    }

    // Fallback to sysfs + pci.ids lookup (~1ms but less accurate names)
    if let Some(name) = gpu_from_sysfs() {
        return name;
    }

    // Final fallback: lspci -mm (slow af but should get it done)
    gpu_from_lspci().unwrap_or_else(|| "unknown".to_string())
}

// Get GPU name from vulkaninfo
fn gpu_from_vulkaninfo() -> Option<String> {
    let output = Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("deviceName") {
            // Format: "	deviceName         = AMD Radeon RX 9070 XT (RADV GFX1201)"
            let name = line.split('=').nth(1)?.trim();
            // Remove the parenthetical driver info
            let name = name.split('(').next().unwrap_or(name).trim();
            // Skip CPU/APU devices (they also show up in vulkaninfo)
            if !name.is_empty() && !name.contains("Processor") && !name.contains("llvmpipe") {
                return Some(name.to_string());
            }
        }
    }
    None
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

// Get GPU name from sysfs + pci.ids database (using cached HashMap)
fn gpu_from_sysfs() -> Option<String> {
    let drm_path = std::path::Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return None;
    }

    // Get cached PCI database
    let pci_db = get_pci_database().as_ref()?;

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

        // O(1) HashMap lookup instead of O(n) linear scan
        let (vendor_name, devices) = pci_db.get(&vendor_id)?;
        let device_name = devices.get(&device_id)?;

        // Extract the part in brackets if present
        let display_name = device_name
            .find('[')
            .and_then(|start| device_name.rfind(']').map(|end| &device_name[start + 1..end]))
            .unwrap_or(device_name);

        let vendor_short = vendor_name
            .find('[')
            .and_then(|start| vendor_name.rfind(']').map(|end| &vendor_name[start + 1..end]))
            .and_then(|s| s.split('/').next())
            .unwrap_or("GPU");

        return Some(format!("{} {}", vendor_short, display_name));
    }
    None
}

// Get GPU name from lspci -mm (final fallback)
fn gpu_from_lspci() -> Option<String> {
    let output = Command::new("lspci").arg("-mm").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // lspci -mm format: Slot Class Vendor Device SVendor SDevice PhySlot Rev ProgIf
    // Fields are quoted, e.g.: 03:00.0 "VGA compatible controller" "AMD" "Navi 48" ...
    for line in stdout.lines() {
        // Look for VGA or 3D controller
        if line.contains("VGA compatible controller") || line.contains("3D controller") {
            // Parse the quoted fields
            let fields: Vec<&str> = line
                .split('"')
                .enumerate()
                .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
                .collect();

            // fields[0] = class, fields[1] = vendor, fields[2] = device name
            if fields.len() >= 3 {
                let vendor = fields[1];
                let device = fields[2];

                // Skip integrated/CPU graphics if possible
                if device.contains("Processor") || device.contains("Integrated") {
                    continue;
                }

                // Shorten common vendor names
                let vendor_short = match vendor {
                    v if v.contains("Advanced Micro Devices") || v.contains("AMD") => "AMD",
                    v if v.contains("NVIDIA") => "NVIDIA",
                    v if v.contains("Intel") => "Intel",
                    _ => vendor,
                };

                return Some(format!("{} {}", vendor_short, device));
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

            // Convert to GB (decimal: 1 GB = 1,000,000,000 bytes)
            let used_gb = used_bytes as f64 / 1_000_000_000.0;
            let total_gb = total_bytes as f64 / 1_000_000_000.0;

            // Use TB for total if >= 1000GB, frees up horizontal line space
            if total_gb >= 1000.0 {
                let total_tb = total_gb / 1000.0;
                // Trim .00 if it's a whole number (e.g., 1.00TB -> 1TB)
                let total_str = if (total_tb - total_tb.round()).abs() < 0.005 {
                    format!("{}TB", total_tb.round() as u64)
                } else {
                    format!("{:.2}TB", total_tb)
                };
                return format!("{} {:.0}GB/{}", bar, used_gb, total_str);
            }

            return format!("{} {:.0}GB/{:.0}GB", bar, used_gb, total_gb);
        }
    }
    "unknown".to_string()
}
