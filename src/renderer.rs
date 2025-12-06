// slowfetch rendering system
// prioritises speed and readability, expect to break things.

use crate::terminalsize::get_terminal_size;
use tintify::TintColorize;

// Box drawing characters
const BOX_TOP_LEFT: char = '╭';
const BOX_TOP_RIGHT: char = '╮';
const BOX_BOTTOM_LEFT: char = '╰';
const BOX_BOTTOM_RIGHT: char = '╯';
const BOX_HORIZONTAL: char = '─';
const BOX_VERTICAL: char = '│';

// Strip ANSI codes to get visible width.
// Because colored text is a liar about its actual length!
fn visible_len(text: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for character in text.chars() {
        if character == '\x1b' {
            in_escape = true; // Oh no, ANSI escape sequence incoming!
        } else if in_escape {
            if character == 'm' {
                in_escape = false; // Phew, we made it through the escape sequence
            }
        } else {
            len += 1; // This one actually counts!
        }
    }
    len
}

// A section of system info with a title and content lines (key, value).
pub struct Section {
    pub title: String,
    pub lines: Vec<(String, String)>,
}

impl Section {
    pub fn new(title: &str, lines: Vec<(String, String)>) -> Self {
        Self {
            title: title.to_string(),
            lines,
        }
    }
}

// Helper to create a string of repeated characters
// Like hitting your head against a wall
fn repeat_char(character: char, count: usize) -> String {
    let mut result = String::with_capacity(count * character.len_utf8());
    for _ in 0..count {
        result.push(character); // Push it real good
    }
    result
}

// Function to build a box around content
fn build_box(
    lines: &[String],
    title: Option<&str>,
    target_width: Option<usize>,
    target_height: Option<usize>,
    center_content: bool,
) -> Vec<String> {
    // Find the widest line in our content ignoring those sneaky ANSI codes
    let content_width = lines
        .iter()
        .map(|line| visible_len(line))
        .max()
        .unwrap_or(0);
    // If title is present, ensure width accommodates it
    let title_len = title.map_or(0, |title_text| title_text.chars().count());
    // Make sure the box is wide enough for both content AND title (no squishing allowed!)
    let min_width = content_width.max(title_len);

    let max_width = target_width.unwrap_or(min_width).max(min_width);

    let content_height = lines.len();
    let min_height = content_height + 2; // +2 for top and bottom borders (they need love too)
    let max_height = target_height.unwrap_or(min_height).max(min_height);

    // If we have extra vertical space, let's split it evenly top and bottom
    let total_v_padding = max_height.saturating_sub(min_height);
    let top_v_padding = total_v_padding / 2; // Half goes up here
    let bottom_v_padding = total_v_padding - top_v_padding; // The rest goes down there

    let mut result = Vec::with_capacity(max_height);

    // Top Border - Let's make it fancy!
    let mut top = String::with_capacity(max_width + 32); // Extra space for ANSI codes
    top.push_str(&BOX_TOP_LEFT.to_string().bright_magenta().to_string());
    if let Some(title_text) = title {
        // Center the title by calculating how many dashes go on each side
        let total_dashes = max_width.saturating_sub(title_len);
        let left = total_dashes / 2; // Left side gets half
        let right = total_dashes - left; // Right side gets the rest (handles odd numbers)

        top.push_str(
            &repeat_char(BOX_HORIZONTAL, left)
                .bright_magenta()
                .to_string(),
        );

        top.push(' ');
        //
        // Section title colorising done here idiot
        //
        top.push_str(&title_text.bright_cyan().to_string());
        top.push(' ');

        top.push_str(
            &repeat_char(BOX_HORIZONTAL, right)
                .bright_magenta()
                .to_string(),
        );
    } else {
        top.push_str(
            &repeat_char(BOX_HORIZONTAL, max_width + 2)
                .bright_magenta()
                .to_string(),
        );
    }
    top.push_str(&BOX_TOP_RIGHT.to_string().bright_magenta().to_string());
    result.push(top);

    // Vertical Padding top
    if top_v_padding > 0 {
        let border = BOX_VERTICAL.to_string().bright_magenta().to_string();
        let empty_row = format!("{}{}{}", border, repeat_char(' ', max_width + 2), border);
        for _ in 0..top_v_padding {
            result.push(empty_row.clone());
        }
    }

    // Content
    for line in lines {
        let line_len = visible_len(line);
        let padding = max_width.saturating_sub(line_len);

        // Should we center this line or push it to the left?
        let (left_pad, right_pad) = if center_content {
            let left_padding = padding / 2;
            (left_padding, padding - left_padding) // Split padding evenly-ish
        } else {
            (0, padding) // All padding goes to the right (left-aligned gang)
        };

        let mut row = String::with_capacity(max_width + 32);
        row.push_str(&BOX_VERTICAL.to_string().bright_magenta().to_string());

        row.push(' '); // Left margin
        row.push_str(&repeat_char(' ', left_pad));
        row.push_str(line);
        row.push_str(&repeat_char(' ', right_pad));
        row.push(' '); // Right margin

        row.push_str(&BOX_VERTICAL.to_string().bright_magenta().to_string());
        result.push(row);
    }

    // Vertical Padding bottom
    if bottom_v_padding > 0 {
        let border = BOX_VERTICAL.to_string().bright_magenta().to_string();
        let empty_row = format!("{}{}{}", border, repeat_char(' ', max_width + 2), border);
        for _ in 0..bottom_v_padding {
            result.push(empty_row.clone());
        }
    }

    // Bottom Border
    let mut bottom = String::with_capacity(max_width + 32);
    bottom.push_str(&BOX_BOTTOM_LEFT.to_string().bright_magenta().to_string());
    bottom.push_str(
        &repeat_char(BOX_HORIZONTAL, max_width + 2)
            .bright_magenta()
            .to_string(),
    );
    bottom.push_str(&BOX_BOTTOM_RIGHT.to_string().bright_magenta().to_string());
    result.push(bottom);

    result
}

// turning boring data into pretty boxes
fn build_sections_lines(sections: &[Section], target_width: Option<usize>) -> Vec<String> {
    // 1. Format info lines with colors (make it pretty
    let formatted_sections: Vec<Vec<String>> = sections
        .iter()
        .map(|section| {
            section
                .lines
                .iter()
                .map(|(key, value)| format!("{}: {}", key.bright_cyan(), value.bright_white()))
                .collect()
        })
        .collect();

    // 2. Calculate content width based on formatted lines
    let content_width = sections
        .iter()
        .zip(formatted_sections.iter())
        .flat_map(|(section, formatted_lines)| {
            std::iter::once(section.title.chars().count()) // Don't forget the title!
                .chain(formatted_lines.iter().map(|line| visible_len(line))) // And all the lines
        })
        .max()
        .unwrap_or(0); // Just in case we have no sections (sad!)

    let max_width = target_width.unwrap_or(content_width).max(content_width);

    let mut result = Vec::new();

    // Build a box for each section and stack em
    for (index, section) in sections.iter().enumerate() {
        let box_lines = build_box(
            &formatted_sections[index],
            Some(&section.title),
            Some(max_width),
            None,
            false, // Left aligned content
        );
        result.extend(box_lines);
    }

    result
}

// Draw ASCII art and sections with adaptive layout.
// Wide art side-by-side, medium art side-by-side, narrow art stacked, or no art if too short.
pub fn draw_layout(
    wide_art: &[String],
    medium_art: &[String],
    narrow_art: &[String],
    sections: &[Section],
) -> String {
    // Calculate widths beforehand (measure twice, cut once!)
    let wide_art_width = wide_art
        .iter()
        .map(|line| visible_len(line))
        .max()
        .unwrap_or(0);
    let medium_art_width = medium_art
        .iter()
        .map(|line| visible_len(line))
        .max()
        .unwrap_or(0);
    let narrow_art_width = narrow_art
        .iter()
        .map(|line| visible_len(line))
        .max()
        .unwrap_or(0);

    // Calculate sections width using the key-value structure
    // this is too account for "Key: Value" format, using +2 for ": "
    let sections_width = sections
        .iter()
        .flat_map(|section| {
            std::iter::once(section.title.chars().count()).chain(section.lines.iter().map(
                |(key, value)| {
                    // Key + ": " + Value (don't forget the colon and space!)
                    visible_len(key) + 2 + visible_len(value)
                },
            ))
        })
        .max()
        .unwrap_or(0);

    // Time for some box math! Each box needs borders (2) and margins (2) = +4
    // Then we need a gap between them (+1)
    let wide_box_width = wide_art_width + 4;
    let medium_box_width = medium_art_width + 4;
    let sections_box_width = sections_width + 4;
    let wide_side_by_side = wide_box_width + 1 + sections_box_width;
    let medium_side_by_side = medium_box_width + 1 + sections_box_width;

    // How big is your terminal, really though i need to know.
    let (term_width, term_height) = get_terminal_size()
        .map(|(cols, rows)| (cols as usize, rows as usize))
        .unwrap_or((80, 24)); // Default to 80x24 if i can't figure it out

    // Calculate sections height: each section has content lines + 2 for borders
    let sections_height: usize = sections
        .iter()
        .map(|section| section.lines.len() + 2)
        .sum();

    // Calculate narrow art height (content + 2 for borders)
    let narrow_art_height = narrow_art.len() + 2;

    let mut output = String::new();

    if term_width >= wide_side_by_side {
        // We have room! Let's go side-by-side with the WIDE art (fancy mode activated)
        let sections_box = build_sections_lines(sections, None);
        let target_height = sections_box.len();
        let wide_art_box = build_box(wide_art, None, None, Some(target_height), true);

        let max_lines = wide_art_box.len().max(sections_box.len());

        for index in 0..max_lines {
            if index < wide_art_box.len() {
                output.push_str(&wide_art_box[index]);
            } else {
                let box_width = visible_len(&wide_art_box[0]);
                output.push_str(&repeat_char(' ', box_width));
            }

            output.push(' ');

            if index < sections_box.len() {
                output.push_str(&sections_box[index]);
            }

            output.push('\n');
        }
    } else if term_width >= medium_side_by_side {
        // Medium art side-by-side (not wide enough for full art, but not narrow either)
        let sections_box = build_sections_lines(sections, None);
        let target_height = sections_box.len();
        let medium_art_box = build_box(medium_art, None, None, Some(target_height), true);

        let max_lines = medium_art_box.len().max(sections_box.len());

        for index in 0..max_lines {
            if index < medium_art_box.len() {
                output.push_str(&medium_art_box[index]);
            } else {
                let box_width = visible_len(&medium_art_box[0]);
                output.push_str(&repeat_char(' ', box_width));
            }

            output.push(' ');

            if index < sections_box.len() {
                output.push_str(&sections_box[index]);
            }

            output.push('\n');
        }
    } else if term_height >= sections_height + narrow_art_height {
        // Stacked layout with narrow art on top
        let max_width = narrow_art_width.max(sections_width);

        let narrow_art_box = build_box(narrow_art, None, Some(max_width), None, true);
        let sections_box = build_sections_lines(sections, Some(max_width));

        for line in &narrow_art_box {
            output.push_str(line);
            output.push('\n');
        }
        for line in &sections_box {
            output.push_str(line);
            output.push('\n');
        }
    } else {
        // Terminal too short, skip the art
        let sections_box = build_sections_lines(sections, None);

        for line in &sections_box {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}
