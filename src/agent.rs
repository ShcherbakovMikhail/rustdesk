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

    let command_args: Vec<String> = std::env::args().collect();
    let is_connection_manager =
        command_args.get(1).map(String::as_str) == Some("--cm");
    let is_get_id = command_args.get(1).map(String::as_str) == Some("--get-id");

    if is_connection_manager && !acquire_connection_manager_mutex() {
        return;
    }

    let mut ui_args = core_main::core_main_with_options(true);

    if is_get_id {
        let result = write_agent_id(command_args.get(2).map(String::as_str));
        if let Err(error) = &result {
            hbb_common::log::error!("Failed to write agent ID: {}", error);
        }
        common::global_clean();
        if result.is_err() {
            std::process::exit(40);
        }
        return;
    }

    if let Some(args) = ui_args.as_mut() {
        if args.is_empty() {
            loop {
                std::thread::park();
            }
        }

        if args.first().map(String::as_str) == Some("--cm") {
            run_headless_connection_manager();
        } else {
            /*
             * Остальные внутренние режимы RustDesk сохраняют
             * штатную обработку.
             */
            ui::start(args);
        }
    }

    common::global_clean();
}

#[cfg(target_os = "windows")]
fn write_agent_id(output_path: Option<&str>) -> std::io::Result<()> {
    let path = match output_path {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let program_data = std::env::var_os("ProgramData").ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "ProgramData environment variable is missing",
                )
            })?;
            std::path::PathBuf::from(program_data)
                .join("RustDesk Agent")
                .join("agent-id.txt")
        }
    };
    // Discard a previous result so a failed IPC query cannot return stale ID.
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    // Read the active server via IPC, not this user's local configuration.
    let id = ipc::get_id();
    let id = id.trim();
    if id.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotConnected,
            "The running host returned no ID through IPC",
        ));
    }
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format!("{}\r\n", id))?;
    hbb_common::log::info!("Agent ID written to {}", path.display());
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_headless_connection_manager() {
    let connection_manager = ui_cm_interface::ConnectionManager {
        ui_handler: HeadlessConnectionManagerHandler,
    };

    /*
     * start_ipc содержит штатную логику RustDesk для:
     *
     * - входящих подключений;
     * - файловой передачи;
     * - clipboard;
     * - терминала;
     * - разрешений;
     * - управления состоянием соединения.
     *
     * Отличается только UI handler: события интерфейса игнорируются.
     */
    ui_cm_interface::start_ipc(connection_manager);
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct HeadlessConnectionManagerHandler;

#[cfg(target_os = "windows")]
impl ui_cm_interface::InvokeUiCM for HeadlessConnectionManagerHandler {
    fn add_connection(&self, client: &ui_cm_interface::Client) {
        hbb_common::log::info!(
            "Headless CM: connection added, id={}, peer={}",
            client.id,
            client.peer_id
        );
    }

    fn remove_connection(&self, id: i32, close: bool) {
        hbb_common::log::info!(
            "Headless CM: connection removed, id={}, close={}",
            id,
            close
        );
    }

    fn new_message(&self, id: i32, _text: String) {
        hbb_common::log::debug!(
            "Headless CM: message received, id={}",
            id
        );
    }

    fn change_theme(&self, _dark: String) {}

    fn change_language(&self) {}

    fn show_elevation(&self, show: bool) {
        hbb_common::log::debug!(
            "Headless CM: elevation state changed, show={}",
            show
        );
    }

    fn update_voice_call_state(
        &self,
        client: &ui_cm_interface::Client,
    ) {
        hbb_common::log::debug!(
            "Headless CM: voice-call state changed, id={}",
            client.id
        );
    }

    fn file_transfer_log(&self, action: &str, log: &str) {
        hbb_common::log::debug!(
            "Headless CM file transfer: action={}, log={}",
            action,
            log
        );
    }
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
            return true;
        }

        if GetLastError() == ERROR_ALREADY_EXISTS {
            return false;
        }
    }

    true
}
