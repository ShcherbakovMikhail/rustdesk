#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use librustdesk::*;

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("rustdesk-agent is supported only on Windows.");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() {
    #[cfg(not(feature = "inline"))]
    unsafe {
        winapi::um::shellscalingapi::SetProcessDpiAwareness(2);
    }

    if let Some(args) = core_main::core_main_with_options(true).as_mut() {
        if args.is_empty() {
            loop {
                std::thread::park();
            }
        }

        ui::start(args);
    }

    common::global_clean();
}
