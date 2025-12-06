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
    // Spawn all threads concurrently - sections and ASCII art
    let core_handler = thread::spawn(|| {
        Section::new(
            "Core",
            vec![
                ("OS".to_string(), coremodules::os()),
                ("Kernel".to_string(), coremodules::kernel()),
                ("Uptime".to_string(), coremodules::uptime()),
            ],
        )
    });

    let hardware_handler = thread::spawn(|| {
        Section::new(
            "Hardware",
            vec![
                ("CPU".to_string(), hardwaremodules::cpu()),
                ("GPU".to_string(), hardwaremodules::gpu()),
                ("Memory".to_string(), hardwaremodules::memory()),
                ("Storage".to_string(), hardwaremodules::storage()),
            ],
        )
    });

    let userspace_handler = thread::spawn(|| {
        Section::new(
            "Userspace",
            vec![
                ("Packages".to_string(), userspacemodules::packages()),
                ("Terminal".to_string(), userspacemodules::terminal()),
                ("Shell".to_string(), userspacemodules::shell()),
                ("WM".to_string(), userspacemodules::wm()),
                ("UI".to_string(), userspacemodules::ui()),
            ],
        )
    });

    let ascii_handler = thread::spawn(|| {
        (
            asciimodule::get_wide_logo_lines(),
            asciimodule::get_narrow_logo_lines(),
        )
    });

    // Wait for all threads to complete
    let core = core_handler.join().expect("Core thread panicked");
    let hardware = hardware_handler.join().expect("Hardware thread panicked");
    let userspace = userspace_handler.join().expect("Userspace thread panicked");
    let (wide_logo, narrow_logo) = ascii_handler.join().expect("ASCII thread panicked");

    print!(
        "{}",
        renderer::draw_layout(&wide_logo, &narrow_logo, &[core, hardware, userspace])
    );
}
