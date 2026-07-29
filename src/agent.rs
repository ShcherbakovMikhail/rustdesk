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

    let is_connection_manager =
        std::env::args().nth(1).as_deref() == Some("--cm");

    if is_connection_manager && !acquire_connection_manager_mutex() {
        return;
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

#[cfg(target_os = "windows")]
fn acquire_connection_manager_mutex() -> bool {
    use std::os::windows::ffi::OsStrExt;
    use winapi::{
        shared::winerror::ERROR_ALREADY_EXISTS,
        um::{
            errhandlingapi::GetLastError,
            synchapi::CreateMutexW,
        },
    };

    let mutex_name: Vec<u16> =
        std::ffi::OsStr::new("Global\\RustDeskAgentConnectionManager")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

    unsafe {
        let handle = CreateMutexW(
            std::ptr::null_mut(),
            0,
            mutex_name.as_ptr(),
        );

        if handle.is_null() {
            // Не блокируем работу CM, если Windows не позволила
            // создать mutex. Ошибка будет видна в последующих логах.
            return true;
        }

        if GetLastError() == ERROR_ALREADY_EXISTS {
            // Другой процесс --cm уже работает.
            // Текущий процесс должен завершиться.
            return false;
        }
    }

    /*
     * HANDLE намеренно остаётся открытым до завершения процесса.
     * Windows автоматически освободит mutex при завершении CM.
     */
    true
}
