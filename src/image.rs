// Image handling module for Slowfetch
// Uses the Kitty graphics protocol to display images in the terminal

use std::path::Path;

// Display an image using the Kitty graphics protocol.
// Kitty handles the scaling - we just tell it the target dimensions in terminal cells.
// arguments:
// `path` - Path to the image file (PNG, JPEG, etc.)
//  `box_cols` - Width of the box in terminal columns
//  `box_rows` - Height of the box in terminal rows\
//
// currently hardcoded image path
//
// returns the escape sequence string to display the image or an error message dun dun duuuun

pub fn display_image(path: &Path, box_cols: u16, box_rows: u16) -> Result<String, String> {
    // Ensure we have an absolute path for Kitty to read
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {}", e))?
            .join(path)
    };

    // Verify file exists
    if !abs_path.exists() {
        return Err(format!("Image file not found: {}", abs_path.display()));
    }

    // Create the kitty graphics command - let Kitty handle the scaling
    let action = kitty_image::Action::TransmitAndDisplay(
        kitty_image::ActionTransmission {
            format: kitty_image::Format::Png,
            medium: kitty_image::Medium::File,
            ..Default::default()
        },
        kitty_image::ActionPut {
            columns: box_cols as u32,
            rows: box_rows as u32,
            ..Default::default()
        },
    );

    let command = kitty_image::Command::with_payload_from_path(action, &abs_path);
    let wrapped = kitty_image::WrappedCommand::new(command);

    Ok(wrapped.to_string())
}

/// Check if the terminal supports the Kitty graphics protocol
pub fn supports_kitty_graphics() -> bool {
    // Check for Kitty
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }

    // Check TERM for kitty or ghostty
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") || term.contains("ghostty") {
            return true;
        }
    }

    // Check TERM_PROGRAM for ghostty
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        if term_program.to_lowercase().contains("ghostty") {
            return true;
        }
    }

    false
}

/// Returns the path to the default slowfetch image
pub fn get_default_image_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/home/tui/Rice/Rust Projects/SlowfetchV2/src/assets/default/slowfetch.png")
}
