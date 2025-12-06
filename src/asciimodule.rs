//! ASCII art module for Slowfetch.
//! Uses inkline to render colorized ASCII art.

use inkline::AsciiArt;
use tintify::{AnsiColors, DynColors};

/// The ASCII art for the Slowfetch logo (wide version).
const ASCII_ART_WIDE: &str = include_str!("assets/wide.txt");

/// The ASCII art for the Slowfetch logo (narrow version).
const ASCII_ART_NARROW: &str = include_str!("assets/narrow.txt");

/// Get the rainbow color palette for the logo.
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

/// Render the wide ASCII art logo and return lines as a Vec.
pub fn get_wide_logo_lines() -> Vec<String> {
    let art = AsciiArt::new(ASCII_ART_WIDE, get_colors(), true);
    art.map(|line| line.to_string()).collect()
}

/// Render the narrow ASCII art logo and return lines as a Vec.
pub fn get_narrow_logo_lines() -> Vec<String> {
    let art = AsciiArt::new(ASCII_ART_NARROW, get_colors(), true);
    art.map(|line| line.to_string()).collect()
}
