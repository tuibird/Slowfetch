//Slowfetch by Tūī

mod asciimodule;
mod coremodules;
mod hardwaremodules;
mod helpers;
mod renderer;
mod terminalsize;
mod userspacemodules;

use renderer::Section;

fn main() {
    let core = Section::new(
        "Core",
        vec![
            ("OS".to_string(), coremodules::os()),
            ("Kernel".to_string(), coremodules::kernel()),
            ("Uptime".to_string(), coremodules::uptime()),
        ],
    );

    let hardware = Section::new(
        "Hardware",
        vec![
            ("CPU".to_string(), hardwaremodules::cpu()),
            ("GPU".to_string(), hardwaremodules::gpu()),
            ("Memory".to_string(), hardwaremodules::memory()),
            ("Storage".to_string(), hardwaremodules::storage()),
        ],
    );

    let userspace = Section::new(
        "Userspace",
        vec![
            ("Packages".to_string(), userspacemodules::packages()),
            ("Terminal".to_string(), userspacemodules::terminal()),
            ("Shell".to_string(), userspacemodules::shell()),
            ("WM".to_string(), userspacemodules::wm()),
            ("UI".to_string(), userspacemodules::ui()),
        ],
    );

    print!(
        "{}",
        renderer::draw_layout(
            &asciimodule::get_wide_logo_lines(),
            &asciimodule::get_narrow_logo_lines(),
            &[core, hardware, userspace]
        )
    );
}
