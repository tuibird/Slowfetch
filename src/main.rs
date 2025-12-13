//Slowfetch by Tūī

mod cache;
mod colorcontrol;
mod configloader;
mod helpers;
mod image;
mod imagerender;
mod modules;
mod renderer;
mod terminalsize;

use clap::Parser;
use configloader::OsArtSetting;
use renderer::Section;
use std::thread;

// cmd line args, *claps*
#[derive(Parser)]
#[command(name = "slowfetch", about = "A slow system info fetcher")]
struct Args {
    // Display OS-specific art. Optionally specify OS name (example: --os arch)
    #[arg(short = 'o', long = "os", num_args = 0..=1, default_missing_value = "")]
    os_art: Option<String>,

    // Force refresh of cached values (OS name and GPU)
    #[arg(short = 'r', long = "refresh")]
    refresh: bool,

    // Display image instead of ASCII art (uses Kitty graphics protocol)
    #[arg(short = 'i', long = "image", num_args = 0..=1, default_missing_value = "")]
    image: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Set cache refresh flag if --refresh/-r was passed
    if args.refresh {
        cache::set_force_refresh(true);
    }

    // Load config first and initialize colors before spawning threads
    let config = configloader::load_config();
    colorcontrol::init_colors(config.colors.clone());

    // Only spawn threads for slow I/O operations (subprocesses)
    // These may run external commands like vulkaninfo, df, shell --version, etc.
    let gpu_handler = thread::spawn(modules::hardwaremodules::gpu);
    let storage_handler = thread::spawn(modules::hardwaremodules::storage);
    let packages_handler = thread::spawn(modules::userspacemodules::packages);
    let shell_handler = thread::spawn(modules::userspacemodules::shell);
    let font_handler = thread::spawn(modules::fontmodule::find_font);
    let screen_handler = thread::spawn(modules::hardwaremodules::screen);

    // Fast operations - just file reads or env var checks, no benefit from threading
    let os = modules::coremodules::os();
    let kernel = modules::coremodules::kernel();
    let uptime = modules::coremodules::uptime();
    let cpu = modules::hardwaremodules::cpu();
    let memory = modules::hardwaremodules::memory();
    let battery = modules::hardwaremodules::laptop_battery();
    let terminal = modules::userspacemodules::terminal();
    let wm = modules::userspacemodules::wm();
    let ui = modules::userspacemodules::ui();
    let editor = modules::userspacemodules::editor();

    // Load ASCII art synchronously - just reading static data
    let wide_logo = modules::asciimodule::get_wide_logo_lines();
    let medium_logo = modules::asciimodule::get_medium_logo_lines();
    let narrow_logo = modules::asciimodule::get_narrow_logo_lines();

    // Collect results and build sections
    let core = Section::new(
        "Core",
        vec![
            ("OS".to_string(), os),
            ("Kernel".to_string(), kernel),
            ("Uptime".to_string(), uptime),
        ],
    );

    let mut hardware_lines = vec![
        ("CPU".to_string(), cpu),
        ("GPU".to_string(), gpu_handler.join().unwrap_or_else(|_| "error".into())),
        ("Memory".to_string(), memory),
        ("Storage".to_string(), storage_handler.join().unwrap_or_else(|_| "error".into())),
    ];

    if battery != "unknown" {
        hardware_lines.push(("Battery".to_string(), battery));
    }

    let screen_entries = screen_handler.join().unwrap_or_else(|_| vec![]);
    hardware_lines.extend(screen_entries);

    let hardware = Section::new("Hardware", hardware_lines);

    let mut userspace_lines = vec![
        ("Packages".to_string(), packages_handler.join().unwrap_or_else(|_| "error".into())),
        ("Terminal".to_string(), terminal),
        ("Shell".to_string(), shell_handler.join().unwrap_or_else(|_| "error".into())),
        ("WM".to_string(), wm),
        ("UI".to_string(), ui),
    ];

    if !editor.is_empty() {
        userspace_lines.push(("Editor".to_string(), editor));
    }

    userspace_lines.push((
        "Terminal Font".to_string(),
        font_handler.join().unwrap_or_else(|_| "error".into()),
    ));

    let userspace = Section::new("Userspace", userspace_lines);

    // Check if image mode is requested (CLI arg or config) AND terminal supports it
    let use_image = args.image.is_some() || config.image;

    if use_image && image::supports_kitty_graphics() {
        // Determine image path:
        // 1. CLI arg with explicit path takes highest priority
        // 2. CLI arg empty (-i/--image) uses config.image_path if set, else default
        // 3. Config image=true uses config.image_path if set, else default
        let image_path = if let Some(ref image_arg) = args.image {
            if image_arg.is_empty() {
                // CLI flag without path - use config image_path if available
                if let Some(ref config_path) = config.image_path {
                    std::path::PathBuf::from(config_path)
                } else {
                    image::get_default_image_path()
                }
            } else if image_arg.starts_with("~/") {
                // CLI flag with explicit path (expand ~)
                if let Some(home) = std::env::var_os("HOME") {
                    std::path::PathBuf::from(home).join(&image_arg[2..])
                } else {
                    std::path::PathBuf::from(image_arg)
                }
            } else {
                // CLI flag with explicit path
                std::path::PathBuf::from(image_arg)
            }
        } else {
            // Config image=true, use config image_path if set, else default
            if let Some(ref config_path) = config.image_path {
                std::path::PathBuf::from(config_path)
            } else {
                image::get_default_image_path()
            }
        };

        // Draw image layout (imagerender handles all the logic)
        imagerender::draw_image_layout(&[core, hardware, userspace], &image_path);
    } else {
        // Standard ASCII art mode
        // Check for custom art first (overrides everything else)
        let (wide, medium, narrow, smol) = if let Some(ref custom_path) = config.custom_art {
            if let Some(custom_art) = modules::asciimodule::get_custom_art_lines(custom_path) {
                (custom_art.clone(), custom_art.clone(), custom_art, None)
            } else {
                // Custom art file not found, fall back to default
                (
                    wide_logo.clone(),
                    medium_logo.clone(),
                    narrow_logo.clone(),
                    None,
                )
            }
        } else {
            // Determine OS art setting: CLI args override config
            let os_art_setting = if let Some(ref os_override) = args.os_art {
                if os_override.is_empty() {
                    OsArtSetting::Auto
                } else {
                    OsArtSetting::Specific(os_override.clone())
                }
            } else {
                config.os_art.clone()
            };

            // Apply OS art setting
            match os_art_setting {
                OsArtSetting::Disabled => (wide_logo, medium_logo, narrow_logo, None),
                OsArtSetting::Auto => {
                    let os_name = core
                        .lines
                        .iter()
                        .find(|(k, _)| k == "OS")
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("");
                    if let Some(os_logo) = modules::asciimodule::get_os_logo_lines(os_name) {
                        let smol_logo = modules::asciimodule::get_os_logo_lines_smol(os_name);
                        (os_logo.clone(), os_logo.clone(), os_logo, smol_logo)
                    } else {
                        (wide_logo, medium_logo, narrow_logo, None)
                    }
                }
                OsArtSetting::Specific(ref os_name) => {
                    if let Some(os_logo) = modules::asciimodule::get_os_logo_lines(os_name) {
                        let smol_logo = modules::asciimodule::get_os_logo_lines_smol(os_name);
                        (os_logo.clone(), os_logo.clone(), os_logo, smol_logo)
                    } else {
                        (wide_logo, medium_logo, narrow_logo, None)
                    }
                }
            }
        };

        print!(
            "{}",
            renderer::draw_layout(
                &wide,
                &medium,
                &narrow,
                &[core, hardware, userspace],
                smol.as_deref()
            )
        );
    }
}
