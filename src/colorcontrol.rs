// Color control module for slowfetch
// Provides hex color support and centralized color definitions
// Colors are loaded from config.toml at runtime

use crate::configloader::ColorConfig;
use std::sync::OnceLock;
use tintify::{DynColors, TintColorize};

// Global color config, initialized once from config file
static COLORS: OnceLock<ColorConfig> = OnceLock::new();

// Initialize colors from config - call this once at startup
pub fn init_colors(colors: ColorConfig) {
    let _ = COLORS.set(colors);
}

// Get the current color config
fn colors() -> &'static ColorConfig {
    COLORS.get_or_init(ColorConfig::default)
}

// Get ASCII art colors as DynColors array for inkline
pub fn get_art_colors() -> Vec<DynColors> {
    let c = colors();
    vec![
        DynColors::Rgb(c.art_1.0, c.art_1.1, c.art_1.2),
        DynColors::Rgb(c.art_2.0, c.art_2.1, c.art_2.2),
        DynColors::Rgb(c.art_3.0, c.art_3.1, c.art_3.2),
        DynColors::Rgb(c.art_4.0, c.art_4.1, c.art_4.2),
        DynColors::Rgb(c.art_5.0, c.art_5.1, c.art_5.2),
        DynColors::Rgb(c.art_6.0, c.art_6.1, c.art_6.2),
        DynColors::Rgb(c.art_7.0, c.art_7.1, c.art_7.2),
        DynColors::Rgb(c.art_8.0, c.art_8.1, c.art_8.2),
        DynColors::Rgb(c.art_9.0, c.art_9.1, c.art_9.2),
    ]
}

// Color application functions
pub fn color_border(text: &str) -> String {
    let c = colors().border;
    text.truecolor(c.0, c.1, c.2).to_string()
}

pub fn color_title(text: &str) -> String {
    let c = colors().title;
    text.truecolor(c.0, c.1, c.2).to_string()
}

pub fn color_key(text: &str) -> String {
    let c = colors().key;
    text.truecolor(c.0, c.1, c.2).to_string()
}

pub fn color_value(text: &str) -> String {
    let c = colors().value;
    text.truecolor(c.0, c.1, c.2).to_string()
}
