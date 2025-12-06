//Slowfetch by Tūī

mod asciimodule;
mod coremodules;
mod hardwaremodules;
mod helpers;
mod renderer;
mod terminalsize;
mod userspacemodules;

use renderer::Section;
use std::thread;

fn main() {
    // Spawn a thread for each individual info function for maximum parallelism
    // Core modules
    let os_handler = thread::spawn(coremodules::os);
    let kernel_handler = thread::spawn(coremodules::kernel);
    let uptime_handler = thread::spawn(coremodules::uptime);

    // Hardware modules
    let cpu_handler = thread::spawn(hardwaremodules::cpu);
    let gpu_handler = thread::spawn(hardwaremodules::gpu);
    let memory_handler = thread::spawn(hardwaremodules::memory);
    let storage_handler = thread::spawn(hardwaremodules::storage);

    // Userspace modules
    let packages_handler = thread::spawn(userspacemodules::packages);
    let terminal_handler = thread::spawn(userspacemodules::terminal);
    let shell_handler = thread::spawn(userspacemodules::shell);
    let wm_handler = thread::spawn(userspacemodules::wm);
    let ui_handler = thread::spawn(userspacemodules::ui);

    // ASCII art
    let ascii_handler = thread::spawn(|| {
        (
            asciimodule::get_wide_logo_lines(),
            asciimodule::get_narrow_logo_lines(),
        )
    });

    // Collect results and build sections
    let core = Section::new(
        "Core",
        vec![
            ("OS".to_string(), os_handler.join().unwrap_or_else(|_| "error".into())),
            ("Kernel".to_string(), kernel_handler.join().unwrap_or_else(|_| "error".into())),
            ("Uptime".to_string(), uptime_handler.join().unwrap_or_else(|_| "error".into())),
        ],
    );

    let hardware = Section::new(
        "Hardware",
        vec![
            ("CPU".to_string(), cpu_handler.join().unwrap_or_else(|_| "error".into())),
            ("GPU".to_string(), gpu_handler.join().unwrap_or_else(|_| "error".into())),
            ("Memory".to_string(), memory_handler.join().unwrap_or_else(|_| "error".into())),
            ("Storage".to_string(), storage_handler.join().unwrap_or_else(|_| "error".into())),
        ],
    );

    let userspace = Section::new(
        "Userspace",
        vec![
            ("Packages".to_string(), packages_handler.join().unwrap_or_else(|_| "error".into())),
            ("Terminal".to_string(), terminal_handler.join().unwrap_or_else(|_| "error".into())),
            ("Shell".to_string(), shell_handler.join().unwrap_or_else(|_| "error".into())),
            ("WM".to_string(), wm_handler.join().unwrap_or_else(|_| "error".into())),
            ("UI".to_string(), ui_handler.join().unwrap_or_else(|_| "error".into())),
        ],
    );

    let (wide_logo, narrow_logo) = ascii_handler.join().expect("ASCII thread panicked");

    print!(
        "{}",
        renderer::draw_layout(&wide_logo, &narrow_logo, &[core, hardware, userspace])
    );
}
