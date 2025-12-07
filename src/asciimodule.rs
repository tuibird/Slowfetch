// ASCII art module for Slowfetch
// Uses inkline to render colorized ASCII art

use inkline::AsciiArt;
use tintify::{AnsiColors, DynColors};

// The ASCII art for the Slowfetch logo Wide version.
const ASCII_ART_WIDE: &str = include_str!("assets/default/wide.txt");

// The ASCII art for the Slowfetch logo medium version.
const ASCII_ART_MEDIUM: &str = include_str!("assets/default/medium.txt");

// The ASCII art for the Slowfetch logo narrow version.
const ASCII_ART_NARROW: &str = include_str!("assets/default/narrow.txt");

// OS-specific ASCII art
const ASCII_ART_ARCH: &str = include_str!("assets/arch.txt");
const ASCII_ART_CACHYOS: &str = include_str!("assets/cachy.txt");
const ASCII_ART_FEDORA: &str = include_str!("assets/fedora.txt");
const ASCII_ART_UBUNTU: &str = include_str!("assets/ubuntu.txt");
const ASCII_ART_NIX: &str = include_str!("assets/nix.txt");

// Smol versions of OS-specific ASCII art
const ASCII_ART_ARCH_SMOL: &str = include_str!("assets/archsmol.txt");
const ASCII_ART_CACHYOS_SMOL: &str = include_str!("assets/cachysmol.txt");
const ASCII_ART_FEDORA_SMOL: &str = include_str!("assets/fedorasmol.txt");
const ASCII_ART_UBUNTU_SMOL: &str = include_str!("assets/ubuntusmol.txt");
const ASCII_ART_NIX_SMOL: &str = include_str!("assets/nixsmol.txt");

// color setup, yep.
fn get_colors() -> &'static [DynColors] {
    &[
        DynColors::Ansi(AnsiColors::BrightRed),     // {1} - Red
        DynColors::Ansi(AnsiColors::BrightYellow),  // {2} - Orange
        DynColors::Ansi(AnsiColors::Yellow),        // {3} - Yellow
        DynColors::Ansi(AnsiColors::BrightGreen),   // {4} - Green -
        DynColors::Ansi(AnsiColors::BrightBlue),    // {5} - Blue
        DynColors::Ansi(AnsiColors::BrightMagenta), // {6} - Violet (pink)
        DynColors::Ansi(AnsiColors::BrightCyan),    // {7} - Cyan (light blue)
        DynColors::Ansi(AnsiColors::BrightWhite),   // {8} - White (its white dude lmao)
    ]
}

// Render the wide ASCII art logo and return lines as a Vec
pub fn get_wide_logo_lines() -> Vec<String> {
    let art = AsciiArt::new(ASCII_ART_WIDE, get_colors(), true);
    art.map(|line| line.to_string()).collect()
}

// Render the medium ASCII art logo and return lines as a Vec
pub fn get_medium_logo_lines() -> Vec<String> {
    let art = AsciiArt::new(ASCII_ART_MEDIUM, get_colors(), true);
    art.map(|line| line.to_string()).collect()
}

// Render the narrow ASCII art logo and return lines as a Vec
pub fn get_narrow_logo_lines() -> Vec<String> {
    let art = AsciiArt::new(ASCII_ART_NARROW, get_colors(), true);
    art.map(|line| line.to_string()).collect()
}

// Get OS-specific art if available, returns None if no match
pub fn get_os_logo_lines(os_name: &str) -> Option<Vec<String>> {
    let os_lower = os_name.to_lowercase();
    let art_str = if os_lower.contains("arch") {
        Some(ASCII_ART_ARCH)
    } else if os_lower.contains("cachyos") || os_lower.contains("cachy") {
        Some(ASCII_ART_CACHYOS)
    } else if os_lower.contains("fedora") {
        Some(ASCII_ART_FEDORA)
    } else if os_lower.contains("ubuntu") {
        Some(ASCII_ART_UBUNTU)
    } else if os_lower.contains("nixos") || os_lower.contains("nix") {
        Some(ASCII_ART_NIX)
    } else {
        None
    };

    art_str.map(|s| {
        let art = AsciiArt::new(s, get_colors(), true);
        art.map(|line| line.to_string()).collect()
    })
}

// Get smol OS-specific art if available, returns None if no match
pub fn get_os_logo_lines_smol(os_name: &str) -> Option<Vec<String>> {
    let os_lower = os_name.to_lowercase();
    let art_str = if os_lower.contains("arch") {
        Some(ASCII_ART_ARCH_SMOL)
    } else if os_lower.contains("cachyos") || os_lower.contains("cachy") {
        Some(ASCII_ART_CACHYOS_SMOL)
    } else if os_lower.contains("fedora") {
        Some(ASCII_ART_FEDORA_SMOL)
    } else if os_lower.contains("ubuntu") {
        Some(ASCII_ART_UBUNTU_SMOL)
    } else if os_lower.contains("nixos") || os_lower.contains("nix") {
        Some(ASCII_ART_NIX_SMOL)
    } else {
        None
    };

    art_str.map(|s| {
        let art = AsciiArt::new(s, get_colors(), true);
        art.map(|line| line.to_string()).collect()
    })
}