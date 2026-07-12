```rust
use librustdesk::*;

/// Точка входа для систем, отличных от Windows.
/// Бинарник агента предназначен только для Windows.
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("rustdesk-agent is supported only on Windows.");
    std::process::exit(1);
}

/// Точка входа RustDesk Enterprise Agent для Windows.
///
/// Режимы запуска:
///
/// rustdesk-agent.exe
///     Запуск через Windows Service Control Manager.
///
/// rustdesk-agent.exe --server
///     Запуск серверной части RustDesk в пользовательской сессии.
///
/// rustdesk-agent.exe --get-id
///     Вывод текущего RustDesk ID в stdout.
///
/// rustdesk-agent.exe --version
///     Вывод версии агента.
#[cfg(target_os = "windows")]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("--server") => run_agent_server(),
    
        Some("--get-id") => print_agent_id(),
    
        Some("--set-password") => {
            let Some(password) = args.get(2) else {
                eprintln!("Password is required.");
                std::process::exit(30);
            };
    
            set_agent_password(password);
        }
    
        Some("--version") => {
            println!("{}", VERSION);
        }
    
        Some("--help") | Some("-h") | Some("/?") => {
            print_help();
        }
    
        Some(unknown_argument) => {
            eprintln!("Unknown argument: {}", unknown_argument);
            eprintln!();
            print_help();
            std::process::exit(2);
        }
    
        None => run_agent_service(),
    }
}

/// Запускает бинарник как Windows-службу.
///
/// Эта функция подключается к Windows Service Control Manager.
/// Имя службы должно совпадать с именем, заданным в
/// start_agent_os_service():
///
/// RustDeskAgent
#[cfg(target_os = "windows")]
fn run_agent_service() {
    hbb_common::init_log(false, "agent-service");

    hbb_common::log::info!(
        "Starting RustDesk Enterprise Agent Windows service"
    );

    start_agent_os_service();
}

/// Запускает серверную часть RustDesk.
///
/// Этот режим автоматически запускается основной Windows-службой
/// в активной пользовательской сессии.
///
/// Здесь запускаются:
///
/// - IPC;
/// - захват экрана;
/// - управление клавиатурой и мышью;
/// - RendezvousMediator;
/// - регистрация на hbbs;
/// - соединения через hbbr.
#[cfg(target_os = "windows")]
fn run_agent_server() {
    if !common::global_init() {
        eprintln!("RustDesk Agent global initialization failed.");
        std::process::exit(10);
    }

    if !platform::windows::bootstrap() {
        hbb_common::log::error!(
            "RustDesk Agent Windows bootstrap failed"
        );

        common::global_clean();
        std::process::exit(11);
    }

    hbb_common::init_log(false, "agent-server");

    hbb_common::log::info!(
        "Starting RustDesk Enterprise Agent host server"
    );

    /*
     * is_server = true
     *
     * Указывает RustDesk, что этот процесс является основным
     * серверным процессом.
     *
     * no_server = false
     *
     * В данном режиме параметр фактически не используется,
     * поскольку is_server уже равен true.
     *
     * start_server() работает до завершения процесса или получения
     * команды закрытия через IPC.
     */
    start_server(true, false);

    hbb_common::log::info!(
        "RustDesk Enterprise Agent host server stopped"
    );

    common::global_clean();
}

/// Получает RustDesk ID и выводит его в stdout.
///
/// Использование:
///
/// rustdesk-agent.exe --get-id
///
/// Пример результата:
///
/// 123456789
///
/// Функцию следует запускать в той же пользовательской сессии,
/// в которой работает дочерний процесс rustdesk-agent.exe --server.
#[cfg(target_os = "windows")]
fn print_agent_id() {
    if !common::global_init() {
        eprintln!("RustDesk Agent global initialization failed.");
        std::process::exit(20);
    }

    /*
     * Сначала пробуем получить ID через IPC.
     *
     * Это предпочтительно, поскольку серверный процесс уже запущен
     * и именно он владеет актуальной конфигурацией.
     */
    let ipc_id = ipc::get_id();

    let id = if ipc_id.trim().is_empty() {
        /*
         * Если IPC ещё не готов, читаем ID напрямую из конфигурации.
         */
        hbb_common::config::Config::get_id()
    } else {
        ipc_id
    };

    common::global_clean();

    if id.trim().is_empty() {
        eprintln!("RustDesk Agent ID is not available yet.");
        std::process::exit(21);
    }

    println!("{}", id.trim());
}

/// Устанавливает пароль доступа.
#[cfg(target_os = "windows")]
fn set_agent_password(password: &str) {
    if password.trim().is_empty() {
        eprintln!("Password cannot be empty.");
        std::process::exit(31);
    }

    if !common::global_init() {
        eprintln!("RustDesk Agent global initialization failed.");
        std::process::exit(32);
    }

    match ipc::set_permanent_password(password.to_owned()) {
        Ok(_) => {
            println!("Permanent password configured successfully.");
        }
        Err(err) => {
            eprintln!("Failed to set permanent password: {}", err);
            common::global_clean();
            std::process::exit(33);
        }
    }

    common::global_clean();
}

/// Выводит справку по параметрам командной строки.
#[cfg(target_os = "windows")]
fn print_help() {
    println!("RustDesk Enterprise Agent");
    println!();
    println!("Usage:");
    println!("  rustdesk-agent.exe                          Run as Windows service");
    println!("  rustdesk-agent.exe --server                 Run the RustDesk host server");
    println!("  rustdesk-agent.exe --get-id                 Print the RustDesk ID");
    println!("  rustdesk-agent.exe --set-password PASSWORD  Set permanent password");
    println!("  rustdesk-agent.exe --version                Print the application version");
    println!("  rustdesk-agent.exe --help                   Show this help");
}

