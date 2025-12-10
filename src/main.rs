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

    // Spawn a thread for each individual info function for maximum parallelism
    // Core modules
    let os_handler = thread::spawn(modules::coremodules::os);
    let kernel_handler = thread::spawn(modules::coremodules::kernel);
    let uptime_handler = thread::spawn(modules::coremodules::uptime);

    // Hardware modules
    let cpu_handler = thread::spawn(modules::hardwaremodules::cpu);
    let gpu_handler = thread::spawn(modules::hardwaremodules::gpu);
    let memory_handler = thread::spawn(modules::hardwaremodules::memory);
    let storage_handler = thread::spawn(modules::hardwaremodules::storage);

    // Userspace modules
    let packages_handler = thread::spawn(modules::userspacemodules::packages);
    let terminal_handler = thread::spawn(modules::userspacemodules::terminal);
    let shell_handler = thread::spawn(modules::userspacemodules::shell);
    let wm_handler = thread::spawn(modules::userspacemodules::wm);
    let ui_handler = thread::spawn(modules::userspacemodules::ui);
    let editor_handler = thread::spawn(modules::userspacemodules::editor);
    let font_handler = thread::spawn(modules::fontmodule::find_font);

    // ASCII art (spawned after colors are initialized)
    let ascii_handler = thread::spawn(|| {
        (
            modules::asciimodule::get_wide_logo_lines(),
            modules::asciimodule::get_medium_logo_lines(),
            modules::asciimodule::get_narrow_logo_lines(),
        )
    });

    // Collect results and build sections
    let core = Section::new(
        "Core",
        vec![
            (
                "OS".to_string(),
                os_handler.join().unwrap_or_else(|_| "error".into()),
            ),
            (
                "Kernel".to_string(),
                kernel_handler.join().unwrap_or_else(|_| "error".into()),
            ),
            (
                "Uptime".to_string(),
                uptime_handler.join().unwrap_or_else(|_| "error".into()),
            ),
        ],
    );

    let hardware = Section::new(
        "Hardware",
        vec![
            (
                "CPU".to_string(),
                cpu_handler.join().unwrap_or_else(|_| "error".into()),
            ),
            (
                "GPU".to_string(),
                gpu_handler.join().unwrap_or_else(|_| "error".into()),
            ),
            (
                "Memory".to_string(),
                memory_handler.join().unwrap_or_else(|_| "error".into()),
            ),
            (
                "Storage".to_string(),
                storage_handler.join().unwrap_or_else(|_| "error".into()),
            ),
        ],
    );

    let editor_result = editor_handler.join().unwrap_or_else(|_| "error".into());

    let mut userspace_lines = vec![
        (
            "Packages".to_string(),
            packages_handler.join().unwrap_or_else(|_| "error".into()),
        ),
        (
            "Terminal".to_string(),
            terminal_handler.join().unwrap_or_else(|_| "error".into()),
        ),
        (
            "Shell".to_string(),
            shell_handler.join().unwrap_or_else(|_| "error".into()),
        ),
        (
            "WM".to_string(),
            wm_handler.join().unwrap_or_else(|_| "error".into()),
        ),
        (
            "UI".to_string(),
            ui_handler.join().unwrap_or_else(|_| "error".into()),
        ),
    ];

    if !editor_result.is_empty() {
        userspace_lines.push(("Editor".to_string(), editor_result));
    }

    userspace_lines.push((
        "Terminal Font".to_string(),
        font_handler.join().unwrap_or_else(|_| "error".into()),
    ));

    let userspace = Section::new("Userspace", userspace_lines);

    // Check if image mode is requested AND terminal supports it
    if args.image.is_some() && image::supports_kitty_graphics() {
        let image_arg = args.image.as_ref().unwrap();

        // Determine image path (expand ~ to home directory)
        let image_path = if image_arg.is_empty() {
            image::get_default_image_path()
        } else if image_arg.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                std::path::PathBuf::from(home).join(&image_arg[2..])
            } else {
                std::path::PathBuf::from(image_arg)
            }
        } else {
            std::path::PathBuf::from(image_arg)
        };

        // Draw image layout (imagerender handles all the logic)
        imagerender::draw_image_layout(&[core, hardware, userspace], &image_path);
    } else {
        // Standard ASCII art mode
        let (wide_logo, medium_logo, narrow_logo) =
            ascii_handler.join().expect("ASCII thread panicked");

        // Determine OS art setting: CLI args override config
        let os_art_setting = if let Some(ref os_override) = args.os_art {
            if os_override.is_empty() {
                OsArtSetting::Auto
            } else {
                OsArtSetting::Specific(os_override.clone())
            }
        } else {
            config.os_art
        };

        // Apply OS art setting
        let (wide, medium, narrow, smol) = match os_art_setting {
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
