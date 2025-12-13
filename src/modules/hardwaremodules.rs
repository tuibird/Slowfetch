// Hardware information modules for Slowfetch.
// Contains functions hardware, what else did you expect idiot

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::process::Command;

use memchr::{memchr_iter, memmem};

use crate::cache;
use crate::helpers::{create_bar, get_pci_database, read_first_line};

// Get the CPU model name with boost clock.
// Uses persistent cache to avoid repeated /proc reads.
pub fn cpu() -> String {
    // Check cache first (unless --refresh was passed)
    if let Some(cached) = cache::get_cached_cpu() {
        return cached;
    }

    // No cache hit, fetch fresh value
    let result = cpu_fresh();

    // Cache the result for next time
    cache::cache_cpu(&result);

    result
}

// Fetch CPU info fresh (no cache)
// Uses BufReader to stop reading after finding model name (avoids reading entire /proc/cpuinfo)
fn cpu_fresh() -> String {
    let model = if let Ok(file) = File::open("/proc/cpuinfo") {
        let reader = BufReader::new(file);
        let mut found_model: Option<String> = None;

        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("model name") {
                if let Some(name) = line.split(':').nth(1) {
                    let words: Vec<&str> = name.split_whitespace().collect();
                    // Find where GPU info starts (e.g., "with Radeon Graphics", "w/ Intel UHD")
                    let gpu_start = words.iter().position(|&w| {
                        w.eq_ignore_ascii_case("with") || w.eq_ignore_ascii_case("w/")
                    });
                    let words = match gpu_start {
                        Some(idx) => &words[..idx],
                        None => &words[..],
                    };
                    found_model = Some(
                        words
                            .iter()
                            .filter(|&&w| !w.ends_with("-Core") && w != "Processor")
                            .copied()
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    break; // Stop reading after finding model name
                }
            }
        }
        found_model
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
// Uses BufReader to stop reading after finding MemTotal and MemAvailable
pub fn memory() -> String {
    let mut total: u64 = 0;
    let mut available: u64 = 0;

    if let Ok(file) = File::open("/proc/meminfo") {
        let reader = BufReader::new(file);

        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    total = val.parse().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    available = val.parse().unwrap_or(0);
                }
            }
            // MemTotal is line 1, MemAvailable is line 3 in /proc/meminfo
            // Stop reading once we have both values
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
    let stdout = &output.stdout;

    // Find "deviceName" using SIMD-accelerated search
    let needle = b"deviceName";
    let pos = memmem::find(stdout, needle)?;

    // Find the '=' after deviceName
    let after_needle = &stdout[pos + needle.len()..];
    let eq_pos = memchr::memchr(b'=', after_needle)?;
    let after_eq = &after_needle[eq_pos + 1..];

    // Find end of line
    let line_end = memchr::memchr(b'\n', after_eq).unwrap_or(after_eq.len());
    let name_bytes = &after_eq[..line_end];

    // Convert to string and trim
    let name = std::str::from_utf8(name_bytes).ok()?.trim();

    // Remove the parenthetical driver info
    let name = name.split('(').next().unwrap_or(name).trim();

    // Skip CPU/APU devices (they also show up in vulkaninfo)
    if !name.is_empty() && !name.contains("Processor") && !name.contains("llvmpipe") {
        return Some(name.to_string());
    }
    None
}

// Get GPU name from glxinfo (requires X11/Wayland with GL)
fn gpu_from_glxinfo() -> Option<String> {
    let output = Command::new("glxinfo").output().ok()?;
    let stdout = &output.stdout;

    // Find "OpenGL renderer" using SIMD-accelerated search
    let needle = b"OpenGL renderer";
    let pos = memmem::find(stdout, needle)?;

    // Find the ':' after the needle
    let after_needle = &stdout[pos + needle.len()..];
    let colon_pos = memchr::memchr(b':', after_needle)?;
    let after_colon = &after_needle[colon_pos + 1..];

    // Find end of line
    let line_end = memchr::memchr(b'\n', after_colon).unwrap_or(after_colon.len());
    let renderer_bytes = &after_colon[..line_end];

    // Convert to string and trim
    let renderer = std::str::from_utf8(renderer_bytes).ok()?.trim();

    // Remove the parenthetical info if present
    let name = renderer.split('(').next().unwrap_or(renderer).trim();
    if !name.is_empty() && name != "llvmpipe" {
        return Some(name.to_string());
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
        let name_bytes = name.as_encoded_bytes();

        // Only process card entries, not card0-DP-1 etc
        // Check starts with "card" and doesn't contain '-'
        if name_bytes.len() < 5
            || &name_bytes[..4] != b"card"
            || memchr::memchr(b'-', name_bytes).is_some()
        {
            continue;
        }

        let uevent_path = entry.path().join("device/uevent");
        let uevent = fs::read(&uevent_path).ok()?;

        // Find PCI_ID using SIMD search
        let pci_id_needle = b"PCI_ID=";
        let pos = memmem::find(&uevent, pci_id_needle)?;
        let after_needle = &uevent[pos + pci_id_needle.len()..];

        // Find end of line
        let line_end = memchr::memchr(b'\n', after_needle).unwrap_or(after_needle.len());
        let pci_id = std::str::from_utf8(&after_needle[..line_end]).ok()?;

        // Find colon separator
        let colon_pos = memchr::memchr(b':', pci_id.as_bytes())?;
        let vendor_id = pci_id[..colon_pos].to_lowercase();
        let device_id = pci_id[colon_pos + 1..].to_lowercase();

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
    let stdout = &output.stdout;

    // lspci -mm format: Slot Class Vendor Device SVendor SDevice PhySlot Rev ProgIf
    // Fields are quoted, e.g.: 03:00.0 "VGA compatible controller" "AMD" "Navi 48" ...

    // Search for VGA or 3D controller lines using SIMD
    let vga_needle = b"VGA compatible controller";
    let d3_needle = b"3D controller";

    let mut search_pos = 0;
    while search_pos < stdout.len() {
        // Find next potential GPU line
        let vga_pos = memmem::find(&stdout[search_pos..], vga_needle);
        let d3_pos = memmem::find(&stdout[search_pos..], d3_needle);

        let match_pos = match (vga_pos, d3_pos) {
            (Some(v), Some(d)) => Some(v.min(d)),
            (Some(v), None) => Some(v),
            (None, Some(d)) => Some(d),
            (None, None) => None,
        };

        let Some(rel_pos) = match_pos else { break };
        let abs_pos = search_pos + rel_pos;

        // Find line start (search backwards for newline)
        let line_start = stdout[..abs_pos]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(0);

        // Find line end
        let line_end = memchr::memchr(b'\n', &stdout[abs_pos..])
            .map(|p| abs_pos + p)
            .unwrap_or(stdout.len());

        let line = std::str::from_utf8(&stdout[line_start..line_end]).ok()?;

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
            if !device.contains("Processor") && !device.contains("Integrated") {
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

        search_pos = line_end + 1;
    }
    None
}

// Get storage usage for all physical disks using statvfs syscall.
// Reads /proc/mounts and uses statvfs for each real filesystem - much faster than spawning df
pub fn storage() -> String {
    let mut total_bytes: u64 = 0;
    let mut used_bytes: u64 = 0;
    let mut seen_devices = std::collections::HashSet::new();

    // Read /proc/mounts as bytes for SIMD-accelerated parsing
    if let Ok(content) = fs::read("/proc/mounts") {
        let mut start = 0;
        for end in memchr_iter(b'\n', &content) {
            let line = &content[start..end];
            start = end + 1;

            // Find first space (device ends here)
            let Some(space1) = memchr::memchr(b' ', line) else {
                continue;
            };
            let device = &line[..space1];

            // Find second space (mount point ends here)
            let rest = &line[space1 + 1..];
            let Some(space2) = memchr::memchr(b' ', rest) else {
                continue;
            };
            let mount_point_bytes = &rest[..space2];

            // Filter for real disks: starts with /dev/ and not loop devices
            if device.len() < 5
                || &device[..5] != b"/dev/"
                || memmem::find(device, b"/loop").is_some()
            {
                continue;
            }

            let Ok(device_str) = std::str::from_utf8(device) else {
                continue;
            };
            let Ok(mount_point) = std::str::from_utf8(mount_point_bytes) else {
                continue;
            };

            // Avoid double counting if device mounted multiple times
            if !seen_devices.insert(device_str.to_string()) {
                continue;
            }

            // Use statvfs syscall to get filesystem stats
            if let Some((total, used)) = get_fs_stats(mount_point) {
                total_bytes += total;
                used_bytes += used;
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
    "unknown".to_string()
}

// Get filesystem stats using statvfs syscall
// Returns (total_bytes, used_bytes) or None on failure
fn get_fs_stats(path: &str) -> Option<(u64, u64)> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(path).ok()?;
    let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();

    // SAFETY: statvfs is a standard POSIX syscall, c_path is valid null-terminated string
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };

    if result != 0 {
        return None;
    }

    // SAFETY: statvfs succeeded, stat is now initialized
    let stat = unsafe { stat.assume_init() };

    let block_size = stat.f_frsize as u64;
    let total_blocks = stat.f_blocks as u64;
    let free_blocks = stat.f_bfree as u64;

    let total = total_blocks * block_size;
    let used = (total_blocks - free_blocks) * block_size;

    Some((total, used))
}

// Get battery status if device is a laptop (chassis check)
pub fn laptop_battery() -> String {
    // Check chassis type to determine if it's a laptop
    // 8: Portable, 9: Laptop, 10: Notebook, 11: Hand Held, 12: Docking Station,
    // 14: Sub Notebook, 30: Tablet, 31: Convertible, 32: Detachable
    let is_laptop = read_first_line("/sys/class/dmi/id/chassis_type")
        .and_then(|t| t.trim().parse::<u32>().ok())
        .map(|t| matches!(t, 8 | 9 | 10 | 11 | 12 | 14 | 30 | 31 | 32))
        .unwrap_or(false);

    if !is_laptop {
        return "unknown".to_string();
    }

    // Find first available battery (usually BAT0 or BAT1)
    let power_supply = std::path::Path::new("/sys/class/power_supply");
    if let Ok(entries) = fs::read_dir(power_supply) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("BAT") {
                let path = entry.path();

                // Get capacity
                let capacity = read_first_line(path.join("capacity").to_str().unwrap_or(""))
                    .and_then(|c| c.parse::<u8>().ok())
                    .unwrap_or(0);

                // Get status
                let status = read_first_line(path.join("status").to_str().unwrap_or(""))
                    .unwrap_or_else(|| "Unknown".to_string());

                let status_icon = match status.as_str() {
                    "Charging" => "󰂐",
                    "Discharging" => "󰂍",
                    _ => &status,
                };

                let bar = create_bar(capacity as f64);

                return format!("{} {}% {}", bar, capacity, status_icon);
            }
        }
    }

    "unknown".to_string()
}

// Get screen resolution and refresh rate using xrandr
// Returns a Vec of (key, value) pairs for each monitor, primary first
pub fn screen() -> Vec<(String, String)> {
    let output = Command::new("xrandr")
        .arg("--current")
        .output()
        .ok();

    if let Some(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        // Store (is_primary, display_string)
        let mut screens: Vec<(bool, String)> = Vec::new();
        let mut current_is_primary = false;
        let mut current_is_portrait = false;

        for line in stdout.lines() {
            // Check for output connection line (e.g., "DP-3 connected primary 2560x1440...")
            if line.contains(" connected") {
                current_is_primary = line.contains(" primary ");
                // Portrait mode indicated by "left" or "right" rotation before the parentheses
                // The part in parentheses lists available rotations, not current rotation
                let before_paren = line.split('(').next().unwrap_or(line);
                current_is_portrait =
                    before_paren.contains(" left") || before_paren.contains(" right");
            }
            // Look for lines indicating the active mode (contains *)
            else if line.contains('*') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let res = parts[0];
                    // Rate often looks like "60.00*+" or "144.00*" or "59.95*"
                    // Filter out non-numeric chars except dot
                    let rate_str = parts[1];
                    let rate: String = rate_str
                        .chars()
                        .filter(|c| c.is_digit(10) || *c == '.')
                        .collect();

                    // Orientation icon: 󰆠 for landscape, 󰆡 for portrait
                    let icon = if current_is_portrait { "󰆡" } else { "󰏠" };

                    // Parse as float for rounding
                    let display_str = if let Ok(rate_f) = rate.parse::<f64>() {
                        format!("{} {} @ {}Hz", icon, res, rate_f.round() as u64)
                    } else {
                        format!("{} {} @ {}Hz", icon, res, rate)
                    };
                    screens.push((current_is_primary, display_str));
                }
            }
        }

        // Sort so primary monitor comes first
        screens.sort_by(|a, b| b.0.cmp(&a.0));

        if !screens.is_empty() {
            if screens.len() == 1 {
                return vec![("Display".to_string(), screens[0].1.clone())];
            }
            // Multiple monitors: header line + tree-style entries
            let mut result = vec![("Displays".to_string(), String::new())];
            let last_idx = screens.len() - 1;
            for (i, (_, s)) in screens.iter().enumerate() {
                if i == last_idx {
                    result.push(("╰─".to_string(), s.clone()));
                } else {
                    result.push(("├─".to_string(), s.clone()));
                }
            }
            return result;
        }
    }

    vec![]
}
