#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use librustdesk::*;

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("rustdesk-agent is supported only on Windows.");
}

#[cfg(target_os = "windows")]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--server") {
        run_agent_server();
    } else if args.get(1).map(String::as_str) == Some("--get-id") {
        write_agent_id();
    } else {
        run_agent_service();
    }
}

#[cfg(target_os = "windows")]
fn run_agent_service() {
    hbb_common::init_log(false, "agent-service");
    start_agent_os_service();
}

#[cfg(target_os = "windows")]
fn run_agent_server() {
    if !common::global_init() {
        hbb_common::log::error!("RustDesk Agent global initialization failed");
        return;
    }

    if !platform::windows::bootstrap() {
        hbb_common::log::error!("RustDesk Agent Windows bootstrap failed");
        common::global_clean();
        return;
    }

    hbb_common::init_log(false, "agent-server");
    hbb_common::log::info!("Starting RustDesk Agent host server");

    start_server(true, false);

    common::global_clean();
}

#[cfg(target_os = "windows")]
fn write_agent_id() {
    if !common::global_init() {
        return;
    }

    let id = hbb_common::config::Config::get_id();

    let path = std::env::temp_dir().join("rustdesk-agent-id.txt");

    let content = format!("RustDesk Agent ID: {}\r\n", id);

    let _ = std::fs::write(&path, content);

    common::global_clean();
}