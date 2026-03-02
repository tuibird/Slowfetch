// GPU information module for Slowfetch.
// Handles GPU detection (both integrated and discrete) and display formatting.

#[cfg(test)]
#[path = "gpumodule_tests.rs"]
mod tests;

use std::fs;
use std::process::Command;

use memchr::memmem;

use crate::cache;
use crate::configloader::GpuDisplayMode;
use crate::helpers::get_pci_database;

// GPU information struct - stores both integrated and discrete GPUs
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub integrated: Option<String>,
    pub discrete: Option<String>,
}

impl GpuInfo {
    pub fn new() -> Self {
        Self {
            integrated: None,
            discrete: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.integrated.is_none() && self.discrete.is_none()
    }
}

// Format GPU display based on GpuInfo and display mode configuration
// Returns a Vec of (key, value) pairs for rendering
pub fn format_gpu_display(info: &GpuInfo, mode: GpuDisplayMode) -> Vec<(String, String)> {
    match mode {
        GpuDisplayMode::Auto => {
            // Show dGPU if present, else iGPU
            if let Some(ref dgpu) = info.discrete {
                vec![("GPU".to_string(), dgpu.clone())]
            } else if let Some(ref igpu) = info.integrated {
                vec![("GPU".to_string(), igpu.clone())]
            } else {
                vec![("GPU".to_string(), "unknown".to_string())]
            }
        }
        GpuDisplayMode::Integrated => {
            // Show only iGPU
            if let Some(ref igpu) = info.integrated {
                vec![("GPU".to_string(), igpu.clone())]
            } else {
                vec![] // Don't show anything if no iGPU
            }
        }
        GpuDisplayMode::Discrete => {
            // Show only dGPU
            if let Some(ref dgpu) = info.discrete {
                vec![("GPU".to_string(), dgpu.clone())]
            } else {
                vec![] // Don't show anything if no dGPU
            }
        }
        GpuDisplayMode::Both => {
            // Show both GPUs in tree-style format (like screen module)
            let mut result = Vec::new();
            let has_both = info.discrete.is_some() && info.integrated.is_some();
            let has_any = info.discrete.is_some() || info.integrated.is_some();

            if !has_any {
                return vec![("GPU".to_string(), "unknown".to_string())];
            }

            // Header line
            result.push(("GPUs".to_string(), String::new()));

            // Show discrete GPU first (if present)
            if let Some(ref dgpu) = info.discrete {
                let branch = if has_both { "├─" } else { "╰─" };
                result.push((branch.to_string(), dgpu.clone()));
            }

            // Show integrated GPU second (if present)
            if let Some(ref igpu) = info.integrated {
                let branch = "╰─";
                result.push((branch.to_string(), igpu.clone()));
            }

            result
        }
    }
}

// Get GPU information (both integrated and discrete GPUs).
// Uses persistent cache to avoid slow subprocess calls on repeated runs.
// Prioritizes sysfs-based detection for speed and accuracy.
pub fn gpu() -> GpuInfo {
    // Check cache first (unless --refresh was passed)
    if let Some(cached) = cache::get_cached_gpu_info() {
        return cached;
    }

    // No cache hit, fetch fresh value
    let result = gpu_fresh();

    // Cache the result for next time
    cache::cache_gpu_info(&result);

    result
}

// Fetch GPU info fresh (no cache)
// Hybrid approach: Use sysfs for fast dual-GPU classification,
// but refine discrete GPU names with vulkaninfo/glxinfo when available
fn gpu_fresh() -> GpuInfo {
    // Start with sysfs multi-GPU detection for fast classification
    let mut info = match gpu_from_sysfs_multi() {
        Some(i) => i,
        None => GpuInfo::new(),
    };

    // If found a discrete GPU from sysfs, try to get a more specific name
    // from vulkaninfo/glxinfo (they often provide better model specificity)
    if info.discrete.is_some() {
        if let Some(specific_name) = gpu_from_vulkaninfo(false) {
            info.discrete = Some(specific_name);
        } else if let Some(specific_name) = gpu_from_glxinfo() {
            info.discrete = Some(specific_name);
        }
    }

    // If sysfs didn't find anything, try fallback methods
    if info.is_empty() {
        // Try to find a discrete GPU first
        if let Some(name) = gpu_from_vulkaninfo(false) {
            info.discrete = Some(name);
        } else if let Some(name) = gpu_from_lspci(false) {
            info.discrete = Some(name);
        }

        // Try to find integrated GPU
        if let Some(name) = igpu_from_cpuinfo() {
            info.integrated = Some(name);
        } else if let Some(name) = gpu_from_vulkaninfo(true) {
            if info.discrete.is_none() {
                info.integrated = Some(name);
            }
        } else if let Some(name) = gpu_from_glxinfo() {
            if info.discrete.is_none() {
                info.integrated = Some(name);
            }
        }

        // Final sysfs fallback (old single-GPU version)
        if info.is_empty() {
            if let Some(name) = gpu_from_sysfs() {
                info.discrete = Some(name);
            }
        }

        // Very final fallback: lspci allowing integrated
        if info.is_empty() {
            if let Some(name) = gpu_from_lspci(true) {
                info.discrete = Some(name);
            }
        }
    }

    info
}


// Extract integrated GPU name from CPU model string in /proc/cpuinfo
// e.g. "AMD Ryzen 7 PRO 8840U w/ Radeon 780M Graphics" -> "AMD Radeon 780M"
fn igpu_from_cpuinfo() -> Option<String> {
    let content = fs::read("/proc/cpuinfo").ok()?;
    let needle = b"model name";
    let pos = memmem::find(&content, needle)?;
    let after_needle = &content[pos + needle.len()..];
    let colon_pos = memchr::memchr(b':', after_needle)?;
    let after_colon = &after_needle[colon_pos + 1..];
    let line_end = memchr::memchr(b'\n', after_colon).unwrap_or(after_colon.len());
    let name = std::str::from_utf8(&after_colon[..line_end]).ok()?.trim();

    // Find "with" or "w/" to locate GPU info
    let lower = name.to_lowercase();
    let gpu_start = lower.find(" with ").or_else(|| lower.find(" w/ "))?;
    let gpu_part = &name[gpu_start..];

    // Skip "with " or "w/ "
    let gpu_name = gpu_part
        .trim_start_matches(" with ")
        .trim_start_matches(" With ")
        .trim_start_matches(" w/ ")
        .trim_end_matches(" Graphics")
        .trim();

    if gpu_name.is_empty() {
        return None;
    }

    // Add vendor prefix if not present
    let vendor = if name.contains("AMD") || name.contains("Ryzen") {
        "AMD"
    } else if name.contains("Intel") {
        "Intel"
    } else {
        ""
    };

    if vendor.is_empty() || gpu_name.starts_with(vendor) {
        Some(gpu_name.to_string())
    } else {
        Some(format!("{} {}", vendor, gpu_name))
    }
}


// Get GPU name from vulkaninfo
// allow_igpu: if true, don't filter out integrated graphics (for laptops without discrete GPU)
fn gpu_from_vulkaninfo(allow_igpu: bool) -> Option<String> {
    let output = Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .ok()?;

    // Don't check exit status - vulkaninfo may return non-zero even with valid output
    let stdout = &output.stdout;

    // Search for all deviceName entries and find the appropriate one
    let needle = b"deviceName";
    let mut search_pos = 0;

    while let Some(relative_pos) = memmem::find(&stdout[search_pos..], needle) {
        let pos = search_pos + relative_pos;

        // Find the '=' after deviceName
        let after_needle = &stdout[pos + needle.len()..];
        let eq_pos = match memchr::memchr(b'=', after_needle) {
            Some(p) => p,
            None => {
                search_pos = pos + needle.len();
                continue;
            }
        };
        let after_eq = &after_needle[eq_pos + 1..];

        // Find end of line
        let line_end = memchr::memchr(b'\n', after_eq).unwrap_or(after_eq.len());
        let name_bytes = &after_eq[..line_end];

        // Convert to string and trim
        let name = match std::str::from_utf8(name_bytes).ok() {
            Some(n) => n.trim(),
            None => {
                search_pos = pos + needle.len();
                continue;
            }
        };

        // Remove the parenthetical driver info
        let name = name.split('(').next().unwrap_or(name).trim();

        // Skip llvmpipe (software renderer)
        if name.is_empty() || name.contains("llvmpipe") {
            search_pos = pos + needle.len();
            continue;
        }

        let type_needle = b"deviceType";
        let is_integrated = if let Some(type_pos) = memmem::find(&stdout[search_pos..], type_needle) {
            let type_line = &stdout[search_pos + type_pos .. search_pos + type_pos + 100];
            type_line.windows(14).any(|w| w == b"INTEGRATED_GPU")
        } else {
            false
        };

        // Skip integrated GPU unless allowed
        if !allow_igpu && is_integrated || name.contains("Processor") {
            search_pos = pos + needle.len();
            continue;
        }

        // Found a valid GPU
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
// Old version - kept for fallback compatibility
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

        // Extract the part in brackets if present using byte-level search
        let device_bytes = device_name.as_bytes();
        let display_name = memchr::memchr(b'[', device_bytes)
            .and_then(|start| {
                // Search backwards from end for ']'
                device_bytes.iter().rposition(|&b| b == b']').map(|end| {
                    std::str::from_utf8(&device_bytes[start + 1..end]).unwrap_or(device_name)
                })
            })
            .unwrap_or(device_name);

        let vendor_bytes = vendor_name.as_bytes();
        let vendor_short = memchr::memchr(b'[', vendor_bytes)
            .and_then(|start| {
                vendor_bytes.iter().rposition(|&b| b == b']').and_then(|end| {
                    let bracketed = std::str::from_utf8(&vendor_bytes[start + 1..end]).ok()?;
                    // Find first '/' using memchr
                    let slash_pos = memchr::memchr(b'/', bracketed.as_bytes());
                    Some(match slash_pos {
                        Some(p) => &bracketed[..p],
                        None => bracketed,
                    })
                })
            })
            .unwrap_or("GPU");

        return Some(format!("{} {}", vendor_short, display_name));
    }
    None
}

// Known AMD APU architecture codenames
// These appear in pci.ids without "Radeon" or "Graphics" suffix
const AMD_APU_CODENAMES: &[&str] = &[
    "Strix Point", // Ryzen AI 9 HX 300 series
    "Strix Halo",  // Ryzen AI Max series
    "Phoenix",     // Ryzen 7000/8000 mobile
    "Hawk Point",  // Ryzen 8040 series
    "Rembrandt",   // Ryzen 6000 mobile
    "Barcelo",     // Ryzen 5000 mobile refresh
    "Cezanne",     // Ryzen 5000 mobile
    "Lucienne",    // Ryzen 5000 mobile
    "Renoir",      // Ryzen 4000 mobile
    "Picasso",     // Ryzen 3000 mobile
    "Raven",       // Ryzen 2000 mobile
    "Stoney",      // older AMD APU
    "Carrizo",     // older AMD APU
    "Kaveri",      // older AMD APU
    "Kabini",      // older AMD APU
    "Mullins",     // older AMD APU
];

// Check if a display name indicates a clearly discrete GPU
fn is_clearly_discrete_gpu(display_name: &str) -> bool {
    display_name.contains("RX ")
        || display_name.contains("RTX ")
        || display_name.contains("GTX ")
        || display_name.contains("Navi")
        || display_name.contains("GeForce")
        || display_name.contains("Quadro")
        || display_name.contains("Arc") // Intel Arc discrete
        || display_name.contains("Vega 56")
        || display_name.contains("Vega 64")
        || display_name.contains("DG1") // Intel DG1
        || display_name.contains("DG2") // Intel DG2
}

// Check if a display name indicates an integrated GPU
fn is_integrated_gpu(display_name: &str, vendor_id: &str, cpu_igpu_name: Option<&String>) -> bool {
    // Check explicit integrated markers
    if display_name.contains("Integrated") {
        return true;
    }

    // Check for AMD APU codenames (e.g., "Phoenix1", "Rembrandt")
    if vendor_id == "1002" {
        for codename in AMD_APU_CODENAMES {
            if display_name.contains(codename) {
                return true;
            }
        }
        // AMD mobile iGPU model numbers (e.g., "780M", "680M")
        // These are integrated when they appear alone (not as part of RX series)
        if (display_name.contains("890M")
            || display_name.contains("880M")
            || display_name.contains("8060S") 
            || display_name.contains("780M")
            || display_name.contains("760M")
            || display_name.contains("740M")
            || display_name.contains("680M")
            || display_name.contains("660M"))
            && !display_name.contains("RX ")
        {
            return true;
        }
    }

    // Check "Graphics" marker (but not for discrete cards that might have it)
    if display_name.contains("Graphics") {
        return true;
    }

    // Intel-specific integrated patterns
    if vendor_id == "8086" {
        if display_name.contains("UHD")
            || display_name.contains("Iris")
            || display_name.contains("HD Graphics")
            || display_name.contains("Xe Graphics")
            // Meteor Lake and newer just say "Intel Graphics"
            || display_name == "Intel Graphics"
            || display_name.contains("Meteor Lake")
            || display_name.contains("Lunar Lake")
            || display_name.contains("Arrow Lake")
        {
            return true;
        }
    }

    // Cross-reference with cpuinfo
    if let Some(cpuname) = cpu_igpu_name {
        if display_name.contains(cpuname.as_str()) || cpuname.contains(display_name) {
            return true;
        }
    }

    false
}

// Struct to hold parsed card info for two-pass classification
struct CardInfo {
    vendor_id: String,
    display_name: String,
    full_name: String,
    is_boot_vga: bool,
}

// Get all GPUs from sysfs and classify them as integrated or discrete
// Uses two-pass approach: first identify discrete GPUs, then classify remainder
fn gpu_from_sysfs_multi() -> Option<GpuInfo> {
    let drm_path = std::path::Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return None;
    }

    // Get cached PCI database
    let pci_db = get_pci_database().as_ref()?;

    // Get CPU model string for iGPU name cross-reference
    let cpu_igpu_name = igpu_from_cpuinfo();

    // Collect all card info first
    let mut cards: Vec<CardInfo> = Vec::new();

    for entry in fs::read_dir(drm_path).ok()?.flatten() {
        let name = entry.file_name();
        let name_bytes = name.as_encoded_bytes();

        // Only process card entries, not card0-DP-1 etc
        if name_bytes.len() < 5
            || &name_bytes[..4] != b"card"
            || memchr::memchr(b'-', name_bytes).is_some()
        {
            continue;
        }

        let device_path = entry.path().join("device");
        let uevent_path = device_path.join("uevent");
        let uevent = match fs::read(&uevent_path) {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Find PCI_ID using SIMD search
        let pci_id_needle = b"PCI_ID=";
        let pos = match memmem::find(&uevent, pci_id_needle) {
            Some(p) => p,
            None => continue,
        };
        let after_needle = &uevent[pos + pci_id_needle.len()..];

        // Find end of line
        let line_end = memchr::memchr(b'\n', after_needle).unwrap_or(after_needle.len());
        let pci_id = match std::str::from_utf8(&after_needle[..line_end]) {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        // Check boot_vga flag
        let boot_vga_path = device_path.join("boot_vga");
        let is_boot_vga = fs::read_to_string(&boot_vga_path)
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .map(|v| v == 1)
            .unwrap_or(false);

        // Parse PCI ID
        let colon_pos = match memchr::memchr(b':', pci_id.as_bytes()) {
            Some(p) => p,
            None => continue,
        };
        let vendor_id = pci_id[..colon_pos].to_lowercase();
        let device_id = pci_id[colon_pos + 1..].to_lowercase();

        // Lookup in PCI database
        let (vendor_name, devices) = match pci_db.get(&vendor_id) {
            Some(v) => v,
            None => continue,
        };
        let device_name = match devices.get(&device_id) {
            Some(d) => d,
            None => continue,
        };

        // Extract display name from brackets in device name
        let device_bytes = device_name.as_bytes();
        let display_name = memchr::memchr(b'[', device_bytes)
            .and_then(|start| {
                device_bytes.iter().rposition(|&b| b == b']').map(|end| {
                    std::str::from_utf8(&device_bytes[start + 1..end]).unwrap_or(device_name)
                })
            })
            .unwrap_or(device_name);

        // Extract vendor short name
        let vendor_bytes = vendor_name.as_bytes();
        let vendor_short = memchr::memchr(b'[', vendor_bytes)
            .and_then(|start| {
                vendor_bytes.iter().rposition(|&b| b == b']').and_then(|end| {
                    let bracketed = std::str::from_utf8(&vendor_bytes[start + 1..end]).ok()?;
                    let slash_pos = memchr::memchr(b'/', bracketed.as_bytes());
                    Some(match slash_pos {
                        Some(p) => &bracketed[..p],
                        None => bracketed,
                    })
                })
            })
            .unwrap_or("GPU");

        let full_name = format!("{} {}", vendor_short, display_name);

        cards.push(CardInfo {
            vendor_id,
            display_name: display_name.to_string(),
            full_name,
            is_boot_vga,
        });
    }

    if cards.is_empty() {
        return None;
    }

    let mut info = GpuInfo::new();

    // PASS 1: Find clearly discrete GPUs first
    // This prevents misclassification when an iGPU with a generic name comes first
    for card in &cards {
        if is_clearly_discrete_gpu(&card.display_name) {
            if info.discrete.is_none() {
                info.discrete = Some(card.full_name.clone());
            }
            break;
        }
    }

    // PASS 2: Classify remaining cards
    // Now its known if a discrete GPU exists, can better classify others
    let has_discrete = info.discrete.is_some();

    for card in &cards {
        // Skip if this is the discrete GPU already found
        if info.discrete.as_ref() == Some(&card.full_name) {
            continue;
        }

        let is_discrete = is_clearly_discrete_gpu(&card.display_name);
        let is_integrated =
            is_integrated_gpu(&card.display_name, &card.vendor_id, cpu_igpu_name.as_ref());

        if is_discrete {
            // Another discrete GPU (multi-GPU setup)
            if info.discrete.is_none() {
                info.discrete = Some(card.full_name.clone());
            }
        } else if is_integrated {
            if info.integrated.is_none() {
                // Prefer cpuinfo name for AMD iGPUs (more specific)
                if card.vendor_id == "1002" {
                    if let Some(ref exact_name) = cpu_igpu_name {
                        info.integrated = Some(exact_name.clone());
                    } else {
                        info.integrated = Some(card.full_name.clone());
                    }
                } else {
                    info.integrated = Some(card.full_name.clone());
                }
            }
        } else if has_discrete && card.is_boot_vga {
            // If its a discrete GPU and this unknown card is boot_vga,
            // it's likely the integrated GPU (common in laptops)
            if info.integrated.is_none() {
                if let Some(ref exact_name) = cpu_igpu_name {
                    info.integrated = Some(exact_name.clone());
                } else {
                    info.integrated = Some(card.full_name.clone());
                }
            }
        } else if !has_discrete {
            // No discrete GPU found, treat unknown as discrete
            if info.discrete.is_none() {
                info.discrete = Some(card.full_name.clone());
            }
        }
    }

    if info.integrated.is_some() || info.discrete.is_some() {
        Some(info)
    } else {
        None
    }
}

// Get GPU name from lspci -mm (final fallback)
// allow_igpu: if true, don't filter out integrated graphics (for laptops without discrete GPU)
fn gpu_from_lspci(allow_igpu: bool) -> Option<String> {
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

            // Skip integrated/CPU graphics unless allowed (for laptops without discrete GPU)
            let is_integrated = device.contains("Processor") || device.contains("Integrated");
            if allow_igpu || !is_integrated {
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
