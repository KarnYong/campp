use super::{ServiceInfo, ServiceMap, ServiceState, ServiceType};
use crate::runtime::locator::{locate_runtime_binaries, RuntimePaths};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// What older CAMPP versions wrote as index.php; replaced by the welcome page only if untouched.
const LEGACY_INDEX_PHP: &str = r#"<?php phpinfo(); ?>"#;
const DEFAULT_INFO_PHP: &str = "<?php phpinfo();\n";
const WELCOME_PHP: &str = include_str!("welcome.php");
const WELCOME_LOGO_B64: &str = include_str!("welcome_logo.b64");

/// Create the welcome index.php and info.php in the projects root when missing.
/// Never overwrites user files, except an untouched legacy phpinfo index.php.
fn ensure_default_pages(projects_dir: &Path) {
    let index_php = projects_dir.join("index.php");
    let is_legacy = fs::read_to_string(&index_php).map(|s| s.trim() == LEGACY_INDEX_PHP).unwrap_or(false);
    if !index_php.exists() || is_legacy {
        let _ = fs::write(&index_php, WELCOME_PHP.replace("{{LOGO}}", WELCOME_LOGO_B64.trim()));
    }
    let info_php = projects_dir.join("info.php");
    if !info_php.exists() {
        let _ = fs::write(&info_php, DEFAULT_INFO_PHP);
    }
}

// Windows-specific: Constant to hide console window
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Configure command to hide console window on Windows
#[cfg(target_os = "windows")]
pub(crate) fn configure_no_window(mut command: Command) -> Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn configure_no_window(command: Command) -> Command {
    command
}

/// Open a log file with retry logic for Windows file locking
fn open_log_file_with_retry(log_path: &PathBuf, service_name: &str) -> Result<File, String> {
    let max_retries = 5;
    let retry_delay_ms = 100;

    for attempt in 0..max_retries {
        // Try to open the file, truncating if it exists (for fresh logs)
        // On subsequent retries, try to append in case another process has it open
        let result = if attempt == 0 {
            File::create(log_path)
        } else {
            OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(log_path)
        };

        match result {
            Ok(file) => return Ok(file),
            Err(e) => {
                if e.raw_os_error() == Some(32) && attempt < max_retries - 1 {
                    // Windows error 32: file is being used by another process
                    // Wait and retry
                    std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms));
                } else {
                    return Err(format!(
                        "Failed to create {} log file after {} attempts: {}",
                        service_name,
                        attempt + 1,
                        e
                    ));
                }
            }
        }
    }

    Err(format!("Failed to create {} log file: maximum retries exceeded", service_name))
}

/// A running service process with its handle and configuration
pub struct ServiceProcess {
    pub name: ServiceType,
    pub child: Option<Child>,
    pub state: ServiceState,
    pub port: u16,
    /// Path to the log file for this service
    pub log_file: Option<PathBuf>,
    /// Error message if the service is in error state
    pub error_message: Option<String>,
    /// Tracked PID for targeted process killing
    pid: Option<u32>,
}

/// Process manager for CAMPP services
pub struct ProcessManager {
    services: HashMap<ServiceType, ServiceProcess>,
    runtime_paths: Option<RuntimePaths>,
    settings: crate::config::AppSettings,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self::with_settings(crate::config::AppSettings::load())
    }

    pub fn with_settings(settings: crate::config::AppSettings) -> Self {
        let mut services = HashMap::new();

        for service_type in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MySQL, ServiceType::PostgreSQL, ServiceType::Ngrok] {
            services.insert(
                service_type,
                ServiceProcess {
                    name: service_type,
                    child: None,
                    state: ServiceState::Stopped,
                    port: Self::port_for_service(service_type, &settings),
                    log_file: None,
                    error_message: None,
                    pid: None,
                },
            );
        }

        Self {
            services,
            runtime_paths: None,
            settings,
        }
    }

    pub fn get_runtime_paths(&self) -> Option<RuntimePaths> {
        self.runtime_paths.clone()
    }

    pub fn get_settings(&self) -> &crate::config::AppSettings {
        &self.settings
    }

    pub fn get_service_port(&self, service: ServiceType) -> Option<u16> {
        self.services.get(&service).map(|s| s.port)
    }

    fn port_for_service(service_type: ServiceType, settings: &crate::config::AppSettings) -> u16 {
        match service_type {
            ServiceType::Caddy => settings.web_port,
            ServiceType::PhpFpm => settings.php_port,
            ServiceType::MySQL => settings.mysql_port,
            ServiceType::PostgreSQL => settings.postgres_port,
            ServiceType::Ngrok => 4040,
        }
    }

    pub fn update_ports(&mut self, settings: &crate::config::AppSettings) {
        self.settings = settings.clone();
        for (service_type, service_process) in self.services.iter_mut() {
            service_process.port = Self::port_for_service(*service_type, settings);
        }
    }

    /// Initialize the process manager with runtime paths
    pub fn initialize(&mut self) -> Result<(), String> {
        let paths = locate_runtime_binaries()?;
        self.runtime_paths = Some(paths);

        // Ensure all required directories exist
        if let Some(ref paths) = self.runtime_paths {
            fs::create_dir_all(&paths.config_dir)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
            fs::create_dir_all(&paths.logs_dir)
                .map_err(|e| format!("Failed to create logs dir: {}", e))?;

            // Create PHP sessions directory for session storage
            let php_sessions_dir = paths.logs_dir.join("php-sessions");
            fs::create_dir_all(&php_sessions_dir)
                .map_err(|e| format!("Failed to create PHP sessions dir: {}", e))?;

            #[cfg(target_os = "linux")]
            fs::create_dir_all(&paths.mysql_data_dir)
                .map_err(|e| format!("Failed to create MariaDB data dir: {}", e))?;
            #[cfg(not(target_os = "linux"))]
            fs::create_dir_all(&paths.mysql_data_dir)
                .map_err(|e| format!("Failed to create MySQL data dir: {}", e))?;

            // Create PostgreSQL data dir only if PostgreSQL binaries are present
            #[cfg(target_os = "windows")]
            let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl.exe");
            #[cfg(not(target_os = "windows"))]
            let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl");
            if pg_ctl.exists() {
                fs::create_dir_all(&paths.pgsql_data_dir)
                    .map_err(|e| format!("Failed to create PostgreSQL data dir: {}", e))?;
            }

            fs::create_dir_all(&paths.projects_dir)
                .map_err(|e| format!("Failed to create projects dir: {}", e))?;
        }

        // Self-heal: ensure downloaded runtime binaries carry the execute bit on
        // Unix. This fixes installs created before execute-bit handling covered
        // versioned extraction trees (mariadb-*/mysql-*/postgresql-*), so a
        // MariaDB/MySQL "Permission denied" startup failure is resolved without
        // forcing the user to re-download the runtime. No-op on Windows.
        if let Ok(app_paths) = crate::runtime::locator::get_app_data_paths() {
            crate::runtime::locator::ensure_runtime_executables(&app_paths.runtime_dir);
        }

        Ok(())
    }

    /// Start a service
    pub fn start(&mut self, service: ServiceType) -> Result<(), String> {
        // Reload settings from disk to pick up any password/port changes
        self.settings = crate::config::AppSettings::load();

        // Ensure we have runtime paths
        if self.runtime_paths.is_none() {
            self.initialize()?;
        }

        // Clone the paths we need before the mutable borrow
        let paths = self.runtime_paths.as_ref().ok_or("Runtime paths not initialized")?.clone();

        ensure_default_pages(&paths.projects_dir);

        // Pre-check: ngrok requires Caddy to be running
        if service == ServiceType::Ngrok {
            let caddy_running = self.services.get(&ServiceType::Caddy)
                .map(|s| s.state.is_running())
                .unwrap_or(false);
            if !caddy_running {
                return Err("Caddy must be running before starting ngrok. Start Caddy first.".to_string());
            }
        }

        let service_process = self
            .services
            .get_mut(&service)
            .ok_or_else(|| format!("Service {:?} not found", service))?;

        // Check if already running
        if service_process.state.is_running() {
            return Ok(());
        }

        service_process.state = ServiceState::Starting;

        // Spawn the appropriate service
        let result = match service {
            ServiceType::Caddy => start_caddy(service_process, &paths, self.settings.php_port, self.settings.mysql_port, &self.settings),
            ServiceType::PhpFpm => start_php_fpm(service_process, &paths),
            ServiceType::MySQL => start_mysql(service_process, &paths, &self.settings),
            ServiceType::PostgreSQL => start_postgresql(service_process, &paths, &self.settings),
            ServiceType::Ngrok => start_ngrok(service_process, &paths, &self.settings),
        };

        match result {
            Ok(_) => {
                service_process.state = ServiceState::Running;
                service_process.error_message = None;
                Ok(())
            }
            Err(e) => {
                service_process.state = ServiceState::Error;
                service_process.error_message = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Stop a service
    pub fn stop(&mut self, service: ServiceType) -> Result<(), String> {
        let service_process = self
            .services
            .get_mut(&service)
            .ok_or_else(|| format!("Service {:?} not found", service))?;

        if !service_process.state.is_running() && service_process.state != ServiceState::Error {
            return Ok(());
        }

        service_process.state = ServiceState::Stopping;

        // PostgreSQL must shut down gracefully via pg_ctl BEFORE we force-kill
        // the child handle — a SIGKILL'd postmaster forces crash recovery on the
        // next start. Wait for it to actually stop, then the child.kill() below
        // is just a fallback for the (rare) case pg_ctl couldn't.
        if service == ServiceType::PostgreSQL {
            if let Some(ref paths) = self.runtime_paths {
                #[cfg(target_os = "windows")]
                let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl.exe");
                #[cfg(not(target_os = "windows"))]
                let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl");

                if pg_ctl.exists() {
                    let _ = configure_no_window(Command::new(&pg_ctl))
                        .arg("stop")
                        .arg("-D")
                        .arg(&paths.pgsql_data_dir)
                        .arg("-m")
                        .arg("fast")
                        .arg("-w") // wait for shutdown to complete
                        .arg("-t")
                        .arg("30") // ...up to 30s
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .output();
                }
            }
        }

        // Unix: ask politely first. child.kill() is SIGKILL, and a SIGKILLed php-fpm
        // master leaves its workers running and holding the port, so PHP-FPM couldn't
        // start again after Stop. SIGTERM also gives MariaDB/MySQL a clean shutdown.
        #[cfg(unix)]
        if let Some(ref mut child) = service_process.child {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = Command::new("kill")
                    .args(["-TERM", &child.id().to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                for _ in 0..100 {
                    // up to 10s
                    if !matches!(child.try_wait(), Ok(None)) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        // Windows: kill the whole process tree, before the parent is killed below (taskkill /T
        // finds children through the parent). MySQL 8.4 runs as two processes (the one we
        // spawn starts a second mysqld that does the work); without /T that one kept
        // running and holding port 3307 after Stop, so backups copied live MySQL files.
        #[cfg(target_os = "windows")]
        if let Some(pid) = service_process.pid {
            let _ = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output();

            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Force-kill the tracked child by handle (fallback, and the Windows path).
        if let Some(ref mut child) = service_process.child {
            let _ = child.kill();
            let _ = child.wait();
        }

        service_process.child = None;
        service_process.pid = None;
        service_process.state = ServiceState::Stopped;
        service_process.error_message = None;

        Ok(())
    }

    /// Restart a service
    pub fn restart(&mut self, service: ServiceType) -> Result<(), String> {
        self.stop(service)?;
        self.start(service)?;
        Ok(())
    }

    /// Get the status of a service
    pub fn status(&self, service: ServiceType) -> ServiceState {
        self.services
            .get(&service)
            .map(|s| s.state.clone())
            .unwrap_or(ServiceState::Stopped)
    }

    /// Get all service statuses
    pub fn get_all_statuses(&self) -> ServiceMap {
        self.services
            .iter()
            .map(|(ty, proc)| {
                (
                    *ty,
                    ServiceInfo {
                        service_type: *ty,
                        state: proc.state.clone(),
                        port: proc.port,
                        error_message: proc.error_message.clone(),
                    },
                )
            })
            .collect()
    }

    /// Update process health (check if processes are still running)
    pub fn update_health(&mut self) {
        for (_service_type, service_process) in self.services.iter_mut() {
            if let Some(ref mut child) = service_process.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process has exited
                        service_process.state = ServiceState::Error;
                        service_process.error_message = Some(format!(
                            "Process exited unexpectedly with status: {:?}",
                            status
                        ));
                        service_process.child = None;
                        service_process.pid = None;
                    }
                    Ok(None) => {
                        // Still running — only update state from Starting, preserve Error
                        if service_process.state == ServiceState::Starting {
                            service_process.state = ServiceState::Running;
                        }
                    }
                    Err(_) => {
                        // Error checking status
                        service_process.state = ServiceState::Error;
                        service_process.error_message = Some(
                            "Failed to check process status".to_string()
                        );
                    }
                }
            }
        }
    }

    /// Stop all running services (called on app shutdown)
    pub fn stop_all(&mut self) -> Result<(), String> {
        let services_to_stop: Vec<ServiceType> = self
            .services
            .iter()
            .filter(|(_, s)| s.state.is_running())
            .map(|(ty, _)| *ty)
            .collect();

        for service in services_to_stop {
            // Ignore errors during shutdown, just try to stop everything
            let _ = self.stop(service);
        }

        Ok(())
    }
}

/// Start Caddy web server
fn start_caddy(service_process: &mut ServiceProcess, paths: &RuntimePaths, php_port: u16, mysql_port: u16, settings: &crate::config::AppSettings) -> Result<(), String> {
    // Kill any existing Caddy processes to avoid port conflicts
    kill_existing_processes(&paths.caddy);

    // Generate phpMyAdmin config only if phpMyAdmin is installed
    if paths.phpmyadmin.join("index.php").exists() {
        crate::config::generator::generate_phpmyadmin_config(paths, mysql_port)?;
    }
    // Generate Adminer launcher only if Adminer is installed
    if paths.adminer.join("adminer.php").exists() {
        crate::config::generator::generate_adminer_config(paths, mysql_port, settings.postgres_port)?;
    }
    // Always regenerate Caddyfile with current port settings
    let caddyfile_path = paths.config_dir.join("Caddyfile");
    crate::config::generator::generate_caddyfile(&caddyfile_path, paths, service_process.port, php_port, settings.bind_address())?;

    // Open log file with retry logic for Windows file locking
    let log_path = paths.logs_dir.join("caddy.log");
    let log_file = open_log_file_with_retry(&log_path, "Caddy")?;

    // Start Caddy
    let mut child = configure_no_window(Command::new(&paths.caddy))
        .arg("run")
        .arg("--config")
        .arg(&caddyfile_path)
        .current_dir(&paths.config_dir)
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start Caddy: {}", e))?;

    // Give it a moment to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("Caddy exited immediately with status: {:?}", status)),
        Ok(None) => {
            let pid = child.id();
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = Some(pid);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check Caddy process: {}", e)),
    }
}

/// Start PHP-FPM (using PHP-CGI for simplicity in MVP)
fn start_php_fpm(service_process: &mut ServiceProcess, paths: &RuntimePaths) -> Result<(), String> {
    // Kill any existing PHP processes to avoid port conflicts
    kill_existing_processes(&paths.php_cgi);
    // php-fpm (macOS/Linux) rewrites its process title to "php-fpm: master process
    // (<conf>)", so the binary path no longer appears. Our conf path does, and is
    // just as CAMPP-specific. Without this, a php-fpm left over from a crashed or
    // force-quit CAMPP keeps port 9000 and PHP-FPM can't start.
    kill_existing_processes(&paths.config_dir.join("php-fpm.conf"));

    // Generate php.ini only if it doesn't exist (first run)
    // Users can manually edit php.ini in the runtime directory
    if !paths.php_ini.exists() {
        crate::config::generator::generate_php_ini(&paths.php_ini, paths)?;
    }

    // Open log file with retry logic
    let log_path = paths.logs_dir.join("php-fpm.log");
    let log_file = open_log_file_with_retry(&log_path, "PHP-FPM")?;

    // Check if we have php-fpm (static-php on Linux/macOS) or php-cgi (Windows)
    let is_fpm = paths.php_cgi.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "php-fpm")
        .unwrap_or(false);

    let mut child = if is_fpm {
        // Generate php-fpm.conf if it doesn't exist
        let fpm_conf_path = paths.config_dir.join("php-fpm.conf");
        if !fpm_conf_path.exists() {
            crate::config::generator::generate_php_fpm_conf(&fpm_conf_path, paths, service_process.port)?;
        } else {
            // Regenerate with current port
            crate::config::generator::generate_php_fpm_conf(&fpm_conf_path, paths, service_process.port)?;
        }

        // PHP-FPM requires -F to run in foreground and -y for config
        configure_no_window(Command::new(&paths.php_cgi))
            .arg("-F")  // Don't daemonize
            .arg("-y")
            .arg(&fpm_conf_path)
            .arg("-c")
            .arg(&paths.php_ini)
            .current_dir(&paths.config_dir)
            .stdout(Stdio::from(log_file.try_clone().unwrap()))
            .stderr(Stdio::from(log_file))
            .spawn()
            .map_err(|e| format!("Failed to start PHP-FPM: {}", e))?
    } else {
        // PHP-CGI (Windows) uses -b for FastCGI mode
        configure_no_window(Command::new(&paths.php_cgi))
            .env("PHP_FCGI_MAX_REQUESTS", "0") // 0 = unlimited; nothing respawns php-cgi, default 500 kills it
            .arg("-b")
            .arg(format!("127.0.0.1:{}", service_process.port))
            .arg("-c")
            .arg(&paths.php_ini)
            .current_dir(&paths.config_dir)
            .stdout(Stdio::from(log_file.try_clone().unwrap()))
            .stderr(Stdio::from(log_file))
            .spawn()
            .map_err(|e| format!("Failed to start PHP-CGI: {}", e))?
    };

    // Give it a moment to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("PHP exited immediately with status: {:?}", status)),
        Ok(None) => {
            let pid = child.id();
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = Some(pid);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check PHP process: {}", e)),
    }
}

/// Start MySQL/MariaDB database server
///
/// **IMPORTANT Platform Differences:**
/// - **Linux**: Uses MariaDB 12.x (binary: mariadbd)
/// - **Windows/macOS**: Uses MySQL 8.x (binary: mysqld)
///
/// These are drop-in replacements for each other, but have different
/// initialization requirements and binary names.
fn start_mysql(service_process: &mut ServiceProcess, paths: &RuntimePaths, settings: &crate::config::AppSettings) -> Result<(), String> {
    // Kill any leftover CAMPP database server (mariadbd/mysqld) to avoid port
    // conflicts. Path-scoped: never touches a system-wide MySQL/MariaDB.
    kill_existing_processes(&paths.mysql);

    // Initialize MySQL data directory if needed
    initialize_mysql_data_dir(paths)?;

    // Clean path and use proper Windows format for MySQL
    let data_dir_str = paths.mysql_data_dir.to_string_lossy().to_string();
    let data_dir_str = data_dir_str.trim_end_matches('\\').trim_end_matches('/');

    // Check if we need to create 127.0.0.1 user (first run)
    let user_created_flag = paths.mysql_data_dir.join(".user_127_0_0_1_created");
    let needs_init_file = !user_created_flag.exists();

    // Always create init file to keep password in sync with settings
    let init_file = paths.logs_dir.join("mysql_init_user.sql");
    let init_sql = mysql_init_sql(&settings.mysql_root_password, settings.listen_all_interfaces);
    fs::write(&init_file, init_sql)
        .map_err(|e| format!("Failed to create init file: {}", e))?;

    // Open log file with retry logic
    let log_path = paths.logs_dir.join("mysql.log");
    #[cfg(target_os = "linux")]
    let log_file = open_log_file_with_retry(&log_path, "MariaDB")?;
    #[cfg(not(target_os = "linux"))]
    let log_file = open_log_file_with_retry(&log_path, "MySQL")?;

    // Build MySQL command — always pass init file to keep password synced.
    // Ensure the binary is executable (tar extraction drops the exec bit on
    // Unix); without this the spawn fails with EACCES on macOS/Linux.
    crate::runtime::locator::ensure_executable(&paths.mysql);

    // Use a custom socket path to avoid conflicts with system MySQL/MariaDB
    // On macOS/Linux, default is /tmp/mysql.sock which conflicts with any
    // system MySQL installation. We place it in the CAMPP logs directory.
    let socket_path = paths.logs_dir.join("mysql.sock");
    let socket_str = socket_path.to_string_lossy().to_string();

    let mut cmd = configure_no_window(Command::new(&paths.mysql));
    cmd.arg("--datadir")
        .arg(&data_dir_str)
        .arg("--port")
        .arg(service_process.port.to_string())
        .arg(format!("--bind-address={}", settings.bind_address()))
        .arg("--socket")
        .arg(&socket_str);
    // MySQL-specific: Disable X Protocol to avoid port 33060 and mysqlx.sock conflicts
    // MariaDB (used on Linux) doesn't have X Protocol, so this is MySQL-only
    #[cfg(not(target_os = "linux"))]
    cmd.arg("--mysqlx=0");
    cmd.arg("--console")
        .arg("--skip-name-resolve")
        .arg("--init-file")
        .arg(&init_file);

    let mut child = cmd
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| {
            let log_content = fs::read_to_string(&log_path).unwrap_or_else(|_| String::from("Could not read log"));
            #[cfg(target_os = "linux")]
            let db_name = "MariaDB";
            #[cfg(not(target_os = "linux"))]
            let db_name = "MySQL";
            format!("Failed to start {}: {}\n\n{} log:\n{}", db_name, e, db_name, log_content)
        })?;

    // Give database more time to start (it's slower than other services)
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            let _ = fs::remove_file(&init_file);
            #[cfg(target_os = "linux")]
            let db_name = "MariaDB";
            #[cfg(not(target_os = "linux"))]
            let db_name = "MySQL";
            Err(format!(
                "{} exited immediately with status: {:?}\n\nCheck logs at: {:?}",
                db_name, status, log_path
            ))
        }
        Ok(None) => {
            // Mark user as created after successful start
            if needs_init_file {
                let _ = fs::write(&user_created_flag, "done");
                #[cfg(target_os = "linux")]
                tracing::info!("MariaDB root@127.0.0.1 user created during startup");
                #[cfg(not(target_os = "linux"))]
                tracing::info!("MySQL root@127.0.0.1 user created during startup");
            }
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = service_process.child.as_ref().map(|c| c.id());
            remove_init_file_when_ready(init_file, service_process.port);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&init_file);
            Err(format!("Failed to check MariaDB process: {}", e))
        }
    }
}

fn initialize_mysql_data_dir(paths: &RuntimePaths) -> Result<(), String> {
    crate::database::mysql::initialize_mysql(paths)
}

/// Start PostgreSQL database server
fn start_postgresql(service_process: &mut ServiceProcess, paths: &RuntimePaths, settings: &crate::config::AppSettings) -> Result<(), String> {
    // Kill any leftover CAMPP PostgreSQL processes (postgres + pg_ctl, all under
    // pgsql_dir). Path-scoped: never touches a system-wide PostgreSQL.
    kill_existing_processes(&paths.pgsql_dir);

    // Check that PostgreSQL binaries exist
    #[cfg(target_os = "windows")]
    let postgres_bin = paths.pgsql_dir.join("bin").join("postgres.exe");
    #[cfg(not(target_os = "windows"))]
    let postgres_bin = paths.pgsql_dir.join("bin").join("postgres");

    if !postgres_bin.exists() {
        return Err(format!(
            "PostgreSQL binary not found at: {}. Please ensure PostgreSQL runtime was downloaded.",
            postgres_bin.display()
        ));
    }

    // Initialize data directory if needed
    crate::database::postgres::initialize_postgresql(
        &paths.pgsql_dir,
        &paths.pgsql_data_dir,
        &paths.logs_dir,
    )?;

    // Generate postgresql.conf and pg_hba.conf
    crate::config::generator::generate_postgresql_conf(
        &paths.pgsql_data_dir,
        service_process.port,
        settings.listen_all_interfaces,
    )?;

    // Use "trust" auth on first run (password not yet set) so psql can connect
    // without a password. After setting the password, we switch to scram-sha-256.
    let password_flag = paths.pgsql_data_dir.join(".password_set");
    let needs_password_setup = !settings.postgres_root_password.is_empty() && !password_flag.exists();
    let pg_auth_method = if needs_password_setup || settings.postgres_root_password.is_empty() {
        "trust"
    } else {
        "scram-sha-256"
    };
    crate::config::generator::generate_pg_hba_conf(
        &paths.pgsql_data_dir,
        pg_auth_method,
        settings.listen_all_interfaces,
    )?;

    // Open log file
    let log_path = paths.logs_dir.join("postgresql.log");
    let log_file = open_log_file_with_retry(&log_path, "PostgreSQL")?;

    let data_dir_str = paths.pgsql_data_dir.to_string_lossy().to_string();

    // Start postgres directly in foreground (like mysqld --console)
    let mut cmd = configure_no_window(Command::new(&postgres_bin));
    cmd.arg("-D").arg(&data_dir_str)
        .arg("-p").arg(service_process.port.to_string());

    // On Unix, set library path so postgres can find shared libraries
    #[cfg(unix)]
    {
        let lib_dir = paths.pgsql_dir.join("lib");
        let lib_path = lib_dir.to_string_lossy().to_string();
        if let Ok(existing) = std::env::var("LD_LIBRARY_PATH") {
            cmd.env("LD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
        } else {
            cmd.env("LD_LIBRARY_PATH", &lib_path);
        }
        if let Ok(existing) = std::env::var("DYLD_LIBRARY_PATH") {
            cmd.env("DYLD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
        } else {
            cmd.env("DYLD_LIBRARY_PATH", &lib_path);
        }
    }

    let mut child = cmd
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start PostgreSQL: {}", e))?;

    // Give PostgreSQL time to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    match child.try_wait() {
        Ok(Some(status)) => Err(format!(
            "PostgreSQL exited immediately with status: {:?}\n\nCheck logs at: {:?}",
            status, log_path
        )),
        Ok(None) => {
            let pid = child.id();
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = Some(pid);

            // Set PostgreSQL root password on first run if configured
            if !settings.postgres_root_password.is_empty() {
                let password_flag_path = paths.pgsql_data_dir.join(".password_set");
                if !password_flag_path.exists() {
                    #[cfg(target_os = "windows")]
                    let psql_bin = paths.pgsql_dir.join("bin").join("psql.exe");
                    #[cfg(not(target_os = "windows"))]
                    let psql_bin = paths.pgsql_dir.join("bin").join("psql");

                    if psql_bin.exists() {
                        // Wait for PostgreSQL to be ready (with polling instead of blind sleep)
                        let ready = wait_for_postgres_ready(&psql_bin, service_process.port, &paths.pgsql_dir, 15);
                        if !ready {
                            tracing::warn!("PostgreSQL not ready in time, skipping password setup");
                        } else {
                            let escaped_pw = settings.postgres_root_password.replace('\'', "''");
                            let sql = format!("ALTER USER root PASSWORD '{}';", escaped_pw);
                            let port_str = service_process.port.to_string();

                            let mut psql_cmd = configure_no_window(Command::new(&psql_bin));
                            psql_cmd
                                .arg("-h").arg("127.0.0.1")
                                .arg("-p").arg(&port_str)
                                .arg("-U").arg("root")
                                .arg("-d").arg("postgres")
                                .arg("-c").arg(&sql)
                                .arg("-w") // Never prompt for password
                                .stdin(Stdio::null()) // Prevent stdin reads from blocking
                                .env("PGCONNECT_TIMEOUT", "5");

                            // On Unix, set library path
                            #[cfg(unix)]
                            {
                                let lib_dir = paths.pgsql_dir.join("lib");
                                let lib_path = lib_dir.to_string_lossy().to_string();
                                if let Ok(existing) = std::env::var("LD_LIBRARY_PATH") {
                                    psql_cmd.env("LD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
                                } else {
                                    psql_cmd.env("LD_LIBRARY_PATH", &lib_path);
                                }
                                if let Ok(existing) = std::env::var("DYLD_LIBRARY_PATH") {
                                    psql_cmd.env("DYLD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
                                } else {
                                    psql_cmd.env("DYLD_LIBRARY_PATH", &lib_path);
                                }
                            }

                            match psql_cmd.output() {
                                Ok(output) if output.status.success() => {
                                    let _ = fs::write(&password_flag_path, "done");
                                    tracing::info!("PostgreSQL root password set successfully");

                                    // Switch to scram-sha-256 now that password is set
                                    if let Err(e) = crate::config::generator::generate_pg_hba_conf(
                                        &paths.pgsql_data_dir, "scram-sha-256", settings.listen_all_interfaces,
                                    ) {
                                        tracing::warn!("Failed to update pg_hba.conf: {}", e);
                                    } else {
                                        // Signal PostgreSQL to reload config
                                        reload_postgresql_conf(&paths);
                                    }
                                }
                                Ok(output) => {
                                    tracing::warn!("Failed to set PostgreSQL password: {}",
                                        String::from_utf8_lossy(&output.stderr));
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to run psql to set password: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        }
        Err(e) => Err(format!("Failed to check PostgreSQL process: {}", e)),
    }
}

/// Start ngrok tunnel service
fn start_ngrok(
    service_process: &mut ServiceProcess,
    paths: &RuntimePaths,
    settings: &crate::config::AppSettings,
) -> Result<(), String> {
    // Check authtoken is configured
    if settings.ngrok_authtoken.is_empty() {
        return Err("ngrok authtoken not configured. Set it in Settings.".to_string());
    }

    // Check that the ngrok binary exists
    if !paths.ngrok.exists() {
        return Err(format!(
            "ngrok binary not found at: {}. Install ngrok from Settings.",
            paths.ngrok.display()
        ));
    }

    // Generate ngrok.yml config with authtoken
    let ngrok_config_path = paths.config_dir.join("ngrok.yml");
    crate::config::generator::generate_ngrok_config(
        &ngrok_config_path,
        &settings.ngrok_authtoken,
        settings.web_port,
    )?;

    // Kill any existing ngrok processes
    kill_existing_processes(&paths.ngrok);

    // Open log file
    let log_path = paths.logs_dir.join("ngrok.log");
    let log_file = open_log_file_with_retry(&log_path, "ngrok")?;

    // Spawn ngrok process
    let mut child = configure_no_window(Command::new(&paths.ngrok))
        .arg("http")
        .arg("--config")
        .arg(&ngrok_config_path)
        .arg(format!("{}", settings.web_port))
        .current_dir(&paths.config_dir)
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start ngrok: {}", e))?;

    // Give ngrok a moment to start
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("ngrok exited immediately with status: {:?}", status)),
        Ok(None) => {
            let pid = child.id();
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = Some(pid);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check ngrok process: {}", e)),
    }
}

/// Wait for PostgreSQL to accept connections by polling with psql
fn wait_for_postgres_ready(psql_bin: &Path, port: u16, pgsql_dir: &Path, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let port_str = port.to_string();

    while start.elapsed() < timeout {
        let mut cmd = configure_no_window(Command::new(psql_bin));
        cmd.arg("-h").arg("127.0.0.1")
            .arg("-p").arg(&port_str)
            .arg("-U").arg("root")
            .arg("-d").arg("postgres")
            .arg("-c").arg("SELECT 1;")
            .arg("-w")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("PGCONNECT_TIMEOUT", "2");

        #[cfg(unix)]
        {
            let lib_dir = pgsql_dir.join("lib");
            let lib_path = lib_dir.to_string_lossy().to_string();
            cmd.env("LD_LIBRARY_PATH", &lib_path);
            cmd.env("DYLD_LIBRARY_PATH", &lib_path);
        }

        match cmd.output() {
            Ok(output) if output.status.success() => return true,
            _ => std::thread::sleep(std::time::Duration::from_millis(500)),
        }
    }
    false
}

/// Quote a value as a MySQL/MariaDB string literal. Backslash is an escape
/// character in MySQL strings, so it is escaped along with the quote;
/// otherwise a password ending in `\` breaks the init file and the server won't start.
pub(crate) fn mysql_string_literal(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// The init file holds the root password in plain text. MySQL/MariaDB runs it before
/// accepting connections, so once the port answers it has been read and can go.
/// Runs in the background so starting the service isn't delayed.
fn remove_init_file_when_ready(init_file: PathBuf, port: u16) {
    std::thread::spawn(move || {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        for _ in 0..120 {
            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        // ponytail: ~2 min ceiling; if the server never answers, the next start overwrites the file anyway.
        if let Err(e) = fs::remove_file(&init_file) {
            tracing::warn!("Could not remove MySQL init file {:?}: {}", init_file, e);
        }
    });
}

/// SQL run via `--init-file` on every MySQL/MariaDB start, keeping every root
/// account on the saved password.
fn mysql_init_sql(password: &str, listen_all_interfaces: bool) -> String {
    let pw = mysql_string_literal(password);
    let mut sql = format!(
        "CREATE USER IF NOT EXISTS 'root'@'127.0.0.1' IDENTIFIED BY {pw};\n\
        ALTER USER 'root'@'127.0.0.1' IDENTIFIED BY {pw};\n\
        GRANT ALL PRIVILEGES ON *.* TO 'root'@'127.0.0.1' WITH GRANT OPTION;\n"
    );
    // root@localhost is the socket/named-pipe account. Both initializers create it
    // with no usable password (MySQL --initialize-insecure: empty; mariadb-install-db:
    // unix_socket, so any process running as this OS user gets in). Put it on the
    // same password. MariaDB spells it out so unix_socket is replaced, not kept as an OR.
    #[cfg(target_os = "linux")]
    sql.push_str(&format!(
        "ALTER USER IF EXISTS 'root'@'localhost' IDENTIFIED VIA mysql_native_password USING PASSWORD({pw});\n"
    ));
    #[cfg(not(target_os = "linux"))]
    sql.push_str(&format!("ALTER USER IF EXISTS 'root'@'localhost' IDENTIFIED BY {pw};\n"));
    if listen_all_interfaces {
        sql.push_str(&format!(
            "CREATE USER IF NOT EXISTS 'root'@'%' IDENTIFIED BY {pw};\n\
            ALTER USER 'root'@'%' IDENTIFIED BY {pw};\n\
            GRANT ALL PRIVILEGES ON *.* TO 'root'@'%' WITH GRANT OPTION;\n"
        ));
    } else {
        // Left over from earlier LAN mode: keep it in sync so an old password never lingers.
        sql.push_str(&format!("ALTER USER IF EXISTS 'root'@'%' IDENTIFIED BY {pw};\n"));
    }
    sql.push_str("FLUSH PRIVILEGES;\n");
    sql
}

/// Signal PostgreSQL to reload its configuration files
fn reload_postgresql_conf(paths: &RuntimePaths) {
    #[cfg(target_os = "windows")]
    let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl.exe");
    #[cfg(not(target_os = "windows"))]
    let pg_ctl = paths.pgsql_dir.join("bin").join("pg_ctl");

    if pg_ctl.exists() {
        let data_dir_str = paths.pgsql_data_dir.to_string_lossy().to_string();
        let mut cmd = configure_no_window(Command::new(&pg_ctl));
        cmd.arg("reload")
            .arg("-D").arg(&data_dir_str)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // pg_ctl links the bundled libpq. Without it the reload fails on Linux/macOS and
        // the server keeps the "trust" pg_hba.conf (no password) until its next restart.
        #[cfg(unix)]
        {
            let lib_path = paths.pgsql_dir.join("lib").to_string_lossy().to_string();
            cmd.env("LD_LIBRARY_PATH", &lib_path);
            cmd.env("DYLD_LIBRARY_PATH", &lib_path);
        }
        let _ = cmd.status();
        tracing::info!("PostgreSQL config reloaded (switched to scram-sha-256)");
    }
}

/// Kill any leftover CAMPP runtime processes running from `path_marker` (a
/// runtime binary or subdirectory) to avoid port conflicts. Path-scoped so a
/// user's system-wide DB/web server is never touched.
fn kill_existing_processes(path_marker: &std::path::Path) {
    super::killer::kill_existing_processes(path_marker)
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_string_literal_escapes_quote_and_backslash() {
        assert_eq!(mysql_string_literal(""), "''");
        assert_eq!(mysql_string_literal("abc"), "'abc'");
        assert_eq!(mysql_string_literal("a'b"), r"'a\'b'");
        assert_eq!(mysql_string_literal(r"ends\"), r"'ends\\'");
    }

    #[test]
    fn init_file_with_password_is_removed_once_the_server_answers() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("mysql_init_user.sql");
        std::fs::write(&file, "ALTER USER ... IDENTIFIED BY 'secret';").unwrap();

        // Reserve a free port, then release it so nothing is listening yet.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        remove_init_file_when_ready(file.clone(), port);
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(file.exists(), "kept while the server isn't accepting connections yet");

        let _server = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        for _ in 0..40 {
            if !file.exists() { break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(!file.exists(), "removed once the port answers");
    }

    #[test]
    fn mysql_init_sql_sets_password_on_every_root_account() {
        let sql = mysql_init_sql("s3cret", false);
        for host in ["127.0.0.1", "localhost", "%"] {
            assert!(sql.contains(&format!("'root'@'{host}'")), "missing root@{host}");
        }
        assert_eq!(sql.matches("'s3cret'").count(), 4);
        assert!(!sql.contains("CREATE USER IF NOT EXISTS 'root'@'%'"), "LAN off must not create root@%");
        assert!(mysql_init_sql("s3cret", true).contains("CREATE USER IF NOT EXISTS 'root'@'%'"));
    }

    #[test]
    fn test_process_manager_new() {
        let manager = ProcessManager::new();

        assert_eq!(manager.services.len(), 5);

        let caddy = manager.services.get(&ServiceType::Caddy).unwrap();
        assert_eq!(caddy.name, ServiceType::Caddy);
        assert_eq!(caddy.state, ServiceState::Stopped);
        assert_eq!(caddy.port, 8080);
        assert!(caddy.child.is_none());
    }

    #[test]
    fn test_process_manager_default() {
        let manager = ProcessManager::default();
        assert_eq!(manager.services.len(), 5);
        assert!(manager.runtime_paths.is_none());
    }

    #[test]
    fn test_status_of_service() {
        let manager = ProcessManager::new();

        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::PhpFpm), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::MySQL), ServiceState::Stopped);
    }

    #[test]
    fn test_get_all_statuses() {
        let manager = ProcessManager::new();
        let statuses = manager.get_all_statuses();

        assert_eq!(statuses.len(), 5);

        let caddy_info = statuses.get(&ServiceType::Caddy).unwrap();
        assert_eq!(caddy_info.service_type, ServiceType::Caddy);
        assert_eq!(caddy_info.state, ServiceState::Stopped);
        assert_eq!(caddy_info.port, 8080);
    }

    #[test]
    fn test_stop_already_stopped_service() {
        let mut manager = ProcessManager::new();

        let result = manager.stop(ServiceType::Caddy);
        assert!(result.is_ok());
        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Stopped);
    }

    #[test]
    fn test_service_error_state_handling() {
        let mut manager = ProcessManager::new();

        let service = manager.services.get_mut(&ServiceType::MySQL).unwrap();
        service.state = ServiceState::Error;
        service.error_message = Some("Test error".to_string());

        assert_eq!(manager.status(ServiceType::MySQL), ServiceState::Error);

        let statuses = manager.get_all_statuses();
        let mysql_info = statuses.get(&ServiceType::MySQL).unwrap();
        assert_eq!(mysql_info.state, ServiceState::Error);
        assert_eq!(mysql_info.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_update_health_with_no_processes() {
        let mut manager = ProcessManager::new();

        manager.update_health();

        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::PhpFpm), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::MySQL), ServiceState::Stopped);
    }

    #[test]
    fn test_port_assignment_for_services() {
        let manager = ProcessManager::new();

        let caddy = manager.services.get(&ServiceType::Caddy).unwrap();
        assert_eq!(caddy.port, 8080);

        let php = manager.services.get(&ServiceType::PhpFpm).unwrap();
        assert_eq!(php.port, 9000);

        let mysql = manager.services.get(&ServiceType::MySQL).unwrap();
        assert_eq!(mysql.port, 3307);

        let pgsql = manager.services.get(&ServiceType::PostgreSQL).unwrap();
        assert_eq!(pgsql.port, 5433);
    }

    #[test]
    fn test_multiple_services_have_independent_states() {
        let mut manager = ProcessManager::new();

        let caddy = manager.services.get_mut(&ServiceType::Caddy).unwrap();
        caddy.state = ServiceState::Running;

        let php = manager.services.get_mut(&ServiceType::PhpFpm).unwrap();
        php.state = ServiceState::Starting;

        let mysql = manager.services.get_mut(&ServiceType::MySQL).unwrap();
        mysql.state = ServiceState::Stopped;

        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Running);
        assert_eq!(manager.status(ServiceType::PhpFpm), ServiceState::Starting);
        assert_eq!(manager.status(ServiceType::MySQL), ServiceState::Stopped);
    }

    #[test]
    fn test_all_services_use_correct_binary_names() {
        let manager = ProcessManager::new();

        for (service_type, process) in &manager.services {
            assert_eq!(process.name, *service_type);
            assert_eq!(process.name.binary_name(), service_type.binary_name());
        }
    }
}

// Integration tests - require actual runtime binaries installed
// Run with: cargo test --lib -- --ignored --test-threads=1
// IMPORTANT: Run with --test-threads=1 to prevent port conflicts
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Mutex;

    // Global mutex to ensure tests run serially even if run with multiple threads
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Check if runtime binaries are available for integration testing
    fn has_runtime_binaries() -> bool {
        if let Ok(paths) = locate_runtime_binaries() {
            paths.caddy.exists() && paths.php_cgi.exists() && paths.mysql.exists()
        } else {
            false
        }
    }

    /// Check if a port is available
    fn is_port_available(port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
    }

    /// Check if all required ports are available
    fn are_ports_available() -> bool {
        is_port_available(8080) && is_port_available(9000) && is_port_available(3307) && is_port_available(5433)
    }

    /// Wait for a service to reach a specific state, with timeout
    fn wait_for_state(manager: &mut ProcessManager, service: ServiceType, expected_state: ServiceState, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let current_state = manager.status(service);
            if current_state == expected_state {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            manager.update_health();
        }
        false
    }

    /// Clean up any running services after test
    fn cleanup_services(manager: &mut ProcessManager) {
        for service in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MySQL, ServiceType::PostgreSQL, ServiceType::Ngrok] {
            let _ = manager.stop(service);
        }
        // Give processes time to fully exit
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    /// Read log file contents for debugging
    fn read_log_file(manager: &ProcessManager, service: ServiceType) -> String {
        // First try to get the log file from the service process
        if let Some(process) = manager.services.get(&service) {
            if let Some(ref log_path) = process.log_file {
                return std::fs::read_to_string(log_path)
                    .unwrap_or_else(|e| format!("Could not read log: {}", e));
            }
        }

        // If not available, try to read from the expected location
        if let Some(ref paths) = manager.runtime_paths {
            let log_name = match service {
                ServiceType::Caddy => "caddy.log",
                ServiceType::PhpFpm => "php-fpm.log",
                ServiceType::MySQL => "mysql.log",
                ServiceType::PostgreSQL => "postgresql.log",
                ServiceType::Ngrok => "ngrok.log",
            };
            let log_path = paths.logs_dir.join(log_name);
            if log_path.exists() {
                return std::fs::read_to_string(&log_path)
                    .unwrap_or_else(|e| format!("Log exists but could not read: {}", e));
            }
        }

        "No log file available".to_string()
    }

    /// Setup test with proper checks, returns error message if setup fails
    fn setup_test() -> Result<ProcessManager, String> {
        if !has_runtime_binaries() {
            return Err("Runtime binaries not found. Run download_runtime first.".to_string());
        }

        // Kill any lingering processes from previous tests
        kill_lingering_processes();

        // Wait a bit for ports to be released
        std::thread::sleep(std::time::Duration::from_millis(500));

        if !are_ports_available() {
            return Err("Required ports (8080, 9000, 3307) are not available. \
                       Please stop any services using these ports.".to_string());
        }

        let mut manager = ProcessManager::new();
        manager.initialize()?;

        Ok(manager)
    }

    /// Kill any lingering service processes from previous test runs
    fn kill_lingering_processes() {
        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "caddy.exe"])
                .output();
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "php-cgi.exe"])
                .output();
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "mysqld.exe"])
                .output();
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "ngrok.exe"])
                .output();
        }

        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("pkill")
                .args(&["-9", "caddy"])
                .output();
            let _ = Command::new("pkill")
                .args(&["-9", "php-cgi"])
                .output();
            let _ = Command::new("pkill")
                .args(&["-9", "mysqld"])
                .output();
            let _ = Command::new("pkill")
                .args(&["-9", "ngrok"])
                .output();
        }
    }

    #[test]
    #[ignore]
    fn test_integration_check_binaries_and_ports() {
        // This test checks prerequisites without starting services
        match setup_test() {
            Ok(_) => println!("SUCCESS: All binaries found and ports available"),
            Err(e) => println!("PREREQUISITE FAILED: {}", e),
        }
    }

    #[test]
    #[ignore]
    fn test_integration_initialize_and_directories() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Verify directories were created
        assert!(manager.runtime_paths.is_some(), "Runtime paths should be set");

        if let Some(ref paths) = manager.runtime_paths {
            assert!(paths.config_dir.exists(), "Config directory should exist");
            assert!(paths.logs_dir.exists(), "Logs directory should exist");
            assert!(paths.mysql_data_dir.exists(), "MySQL data directory should exist");
            assert!(paths.projects_dir.exists(), "Projects directory should exist");
        }

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_start_stop_caddy() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start Caddy
        let result = manager.start(ServiceType::Caddy);
        if let Err(e) = &result {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for Caddy to be running
        let is_running = wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 5);
        assert!(is_running, "Caddy should be in Running state");

        // Stop Caddy
        manager.stop(ServiceType::Caddy).expect("Caddy should stop");

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_start_stop_php() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start PHP
        let result = manager.start(ServiceType::PhpFpm);
        if let Err(e) = &result {
            let logs = read_log_file(&manager, ServiceType::PhpFpm);
            panic!("PHP failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for PHP to be running
        let is_running = wait_for_state(&mut manager, ServiceType::PhpFpm, ServiceState::Running, 5);
        assert!(is_running, "PHP should be in Running state");

        // Stop PHP
        manager.stop(ServiceType::PhpFpm).expect("PHP should stop");

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_start_stop_mysql() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start MySQL
        let result = manager.start(ServiceType::MySQL);
        if let Err(e) = &result {
            let logs = read_log_file(&manager, ServiceType::MySQL);
            panic!("MySQL failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for MySQL to be running (longer timeout)
        let is_running = wait_for_state(&mut manager, ServiceType::MySQL, ServiceState::Running, 15);
        assert!(is_running, "MySQL should be in Running state");

        // Stop MySQL
        manager.stop(ServiceType::MySQL).expect("MySQL should stop");

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_restart_caddy() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start Caddy
        if let Err(e) = manager.start(ServiceType::Caddy) {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy failed to start: {}\n\nLogs:\n{}", e, logs);
        }
        wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 5);

        // Restart Caddy
        let result = manager.restart(ServiceType::Caddy);
        assert!(result.is_ok(), "Restart should succeed");

        // Should be running again after restart
        let is_running = wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 5);
        assert!(is_running, "Caddy should be running after restart");

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_all_services_concurrent() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start all services
        if let Err(e) = manager.start(ServiceType::Caddy) {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy failed to start: {}\n\nLogs:\n{}", e, logs);
        }
        if let Err(e) = manager.start(ServiceType::PhpFpm) {
            let logs = read_log_file(&manager, ServiceType::PhpFpm);
            panic!("PHP failed to start: {}\n\nLogs:\n{}", e, logs);
        }
        if let Err(e) = manager.start(ServiceType::MySQL) {
            let logs = read_log_file(&manager, ServiceType::MySQL);
            panic!("MySQL failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for all to be running
        let caddy_running = wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 10);
        let php_running = wait_for_state(&mut manager, ServiceType::PhpFpm, ServiceState::Running, 10);
        let mysql_running = wait_for_state(&mut manager, ServiceType::MySQL, ServiceState::Running, 20);

        if !caddy_running {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy not running. Logs:\n{}", logs);
        }
        if !php_running {
            let logs = read_log_file(&manager, ServiceType::PhpFpm);
            panic!("PHP not running. Logs:\n{}", logs);
        }
        if !mysql_running {
            let logs = read_log_file(&manager, ServiceType::MySQL);
            panic!("MySQL not running. Logs:\n{}", logs);
        }

        // Stop all services
        manager.stop(ServiceType::MySQL).ok();
        manager.stop(ServiceType::PhpFpm).ok();
        manager.stop(ServiceType::Caddy).ok();

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_health_check() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start Caddy
        if let Err(e) = manager.start(ServiceType::Caddy) {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy failed to start: {}\n\nLogs:\n{}", e, logs);
        }
        wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 5);

        // Update health should maintain Running state
        manager.update_health();
        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Running);

        // Kill the process and check health detects it
        if let Some(ref mut child) = manager.services.get_mut(&ServiceType::Caddy).unwrap().child {
            let _ = child.kill();
            let _ = child.wait();
        }

        manager.update_health();

        // Health check should detect process is gone
        let state = manager.status(ServiceType::Caddy);
        assert!(state == ServiceState::Error || state == ServiceState::Stopped,
                "State should be Error or Stopped after process dies, got {:?}", state);

        cleanup_services(&mut manager);
    }

    #[test]
    #[ignore]
    fn test_integration_log_files_created() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start Caddy
        if let Err(e) = manager.start(ServiceType::Caddy) {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy failed to start: {}\n\nLogs:\n{}", e, logs);
        }
        wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 5);

        // Check log file was created
        let caddy_process = manager.services.get(&ServiceType::Caddy).unwrap();
        if let Some(ref log_path) = caddy_process.log_file {
            assert!(log_path.exists(), "Log file should exist at {:?}", log_path);
        } else {
            panic!("Log file path should be set");
        }

        cleanup_services(&mut manager);
    }
}
