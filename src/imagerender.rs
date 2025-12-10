// Image rendering module for Slowfetch
// Handles layout and display of images using the Kitty graphics protocol

use crate::renderer::{build_box, build_sections_lines, repeat_char, visible_len, Section};
use crate::terminalsize::get_terminal_size;

// Draw a side by side layout, or vertical stack
// Handles the full image rendering including cursor positioning.

pub fn draw_image_layout(sections: &[Section], image_path: &std::path::Path) {
    use std::io::Write;

    let (term_width, term_height) = get_terminal_size()
        .map(|(cols, rows)| (cols as usize, rows as usize))
        .unwrap_or((80, 24));

    // Calculate sections dimensions
    let sections_width = sections
        .iter()
        .flat_map(|section| {
            std::iter::once(section.title.chars().count()).chain(
                section
                    .lines
                    .iter()
                    .map(|(key, value)| visible_len(key) + 2 + visible_len(value)),
            )
        })
        .max()
        .unwrap_or(0);

    let sections_box_width = sections_width + 4; // borders + margins

    // Calculate sections height
    let sections_height: usize = sections
        .iter()
        .map(|section| section.lines.len() + 2)
        .sum();

    // Image box should be roughly square-ish based on the sections height
    // Terminal cells are typically ~2:1 height:width ratio
    let image_box_content_width = (sections_height as f64 * 2.0) as usize;
    let image_box_width = image_box_content_width + 4; // borders + margins

    let side_by_side_width = image_box_width + 1 + sections_box_width;

    if term_width >= side_by_side_width {
        // Wide layout: side-by-side
        let sections_box = build_sections_lines(sections, None);
        let target_height = sections_box.len();

        // Create empty content for the image box
        let empty_lines: Vec<String> = Vec::new();
        let image_box = build_box(
            &empty_lines,
            None,
            Some(image_box_content_width),
            Some(target_height),
            true,
        );

        let max_lines = image_box.len().max(sections_box.len());

        // Build the layout string
        let mut output = String::new();
        for index in 0..max_lines {
            if index < image_box.len() {
                output.push_str(&image_box[index]);
            } else {
                let box_width = visible_len(&image_box[0]);
                output.push_str(&repeat_char(' ', box_width));
            }

            output.push(' ');

            if index < sections_box.len() {
                output.push_str(&sections_box[index]);
            }

            output.push('\n');
        }

        let total_lines = output.lines().count();
        let box_cols = image_box_content_width;
        let box_rows = target_height.saturating_sub(2);

        // Print the layout first otherwise things get freaky
        print!("{}", output);
        let _ = std::io::stdout().flush();

        // Move cursor up to position inside the image box
        print!("\x1b[{}A", total_lines - 1);
        print!("\x1b[2C");
        let _ = std::io::stdout().flush();

        // Display the image scaled to fit the box
        match crate::image::display_image(image_path, box_cols as u16, box_rows as u16) {
            Ok(img_output) => {
                print!("{}", img_output);
                let _ = std::io::stdout().flush();
            }
            Err(image_error) => eprintln!("Image error: {}", image_error),
        }

        // Move cursor back down to after the layout
        println!("\x1b[{}B", total_lines);
        let _ = std::io::stdout().flush();
    } else {
        // Narrow layout: stacked (image on top, sections below)
        // Image box maintains 1:1 aspect ratio based on sections_box_width
        // Terminal cells are ~2:1 height:width ratio
        // Box visual width = sections_width + 6 (content + 2 borders + 2 internal padding)
        // Box visual height = (sections_width + 6) / 2 for 1:1 aspect
        let image_content_width = sections_width;
        let image_box_total_height = ((sections_width + 6) as f64 / 2.0).ceil() as usize;
        let image_content_height = image_box_total_height.saturating_sub(2); // content area for image

        // Check if we have enough vertical space for stacked layout
        let stacked_height = image_box_total_height + sections_height;

        if term_height >= stacked_height && image_content_width > 8 {
            // Build image box (target_height is total box height)
            let empty_lines: Vec<String> = Vec::new();
            let image_box = build_box(
                &empty_lines,
                None,
                Some(image_content_width),
                Some(image_box_total_height),
                true,
            );

            // Build sections with matching width
            let sections_box = build_sections_lines(sections, Some(image_content_width));

            // Print image box
            let mut output = String::new();
            for line in &image_box {
                output.push_str(line);
                output.push('\n');
            }

            // Print sections
            for line in &sections_box {
                output.push_str(line);
                output.push('\n');
            }

            let total_lines = output.lines().count();

            // Print the layout
            print!("{}", output);
            let _ = std::io::stdout().flush();

            // Move cursor up to position inside the image box (at top)
            print!("\x1b[{}A", total_lines - 1);
            print!("\x1b[2C");
            let _ = std::io::stdout().flush();

            // Display the image
            match crate::image::display_image(image_path, image_content_width as u16, image_content_height as u16) {
                Ok(img_output) => {
                    print!("{}", img_output);
                    let _ = std::io::stdout().flush();
                }
                Err(image_error) => eprintln!("Image error: {}", image_error),
            }

            // Move cursor back down
            println!("\x1b[{}B", total_lines);
            let _ = std::io::stdout().flush();
        } else {
            // not enough terminal space, just draw fetch content
            let sections_box = build_sections_lines(sections, None);

            for line in &sections_box {
                println!("{}", line);
            }
        }
    }
}
