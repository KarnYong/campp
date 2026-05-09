use super::{ServiceInfo, ServiceMap, ServiceState, ServiceType};
use crate::runtime::locator::{locate_runtime_binaries, RuntimePaths};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

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

        for service_type in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MySQL] {
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

    fn port_for_service(service_type: ServiceType, settings: &crate::config::AppSettings) -> u16 {
        match service_type {
            ServiceType::Caddy => settings.web_port,
            ServiceType::PhpFpm => settings.php_port,
            ServiceType::MySQL => settings.mysql_port,
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
            fs::create_dir_all(&paths.projects_dir)
                .map_err(|e| format!("Failed to create projects dir: {}", e))?;
        }

        Ok(())
    }

    /// Start a service
    pub fn start(&mut self, service: ServiceType) -> Result<(), String> {
        // Ensure we have runtime paths
        if self.runtime_paths.is_none() {
            self.initialize()?;
        }

        // Clone the paths we need before the mutable borrow
        let paths = self.runtime_paths.as_ref().ok_or("Runtime paths not initialized")?.clone();

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
            ServiceType::Caddy => start_caddy(service_process, &paths, self.settings.php_port, self.settings.mysql_port),
            ServiceType::PhpFpm => start_php_fpm(service_process, &paths),
            ServiceType::MySQL => start_mysql(service_process, &paths),
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

        // Kill the tracked child process by handle
        if let Some(ref mut child) = service_process.child {
            let _ = child.kill();
            let _ = child.wait();
        }

        // For MySQL/MariaDB on Windows, also kill by PID to handle spawned child processes
        #[cfg(target_os = "windows")]
        if let Some(pid) = service_process.pid {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output();

            std::thread::sleep(std::time::Duration::from_millis(500));
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
fn start_caddy(service_process: &mut ServiceProcess, paths: &RuntimePaths, php_port: u16, mysql_port: u16) -> Result<(), String> {
    // Kill any existing Caddy processes to avoid port conflicts
    kill_existing_processes("caddy");

    // Generate phpMyAdmin config if needed
    crate::config::generator::generate_phpmyadmin_config(paths, mysql_port)?;
    // Always regenerate Caddyfile with current port settings
    let caddyfile_path = paths.config_dir.join("Caddyfile");
    crate::config::generator::generate_caddyfile(&caddyfile_path, paths, service_process.port, php_port)?;

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
    kill_existing_processes("php-fpm");
    kill_existing_processes("php-cgi");

    // Generate php.ini if it doesn't exist
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
fn start_mysql(service_process: &mut ServiceProcess, paths: &RuntimePaths) -> Result<(), String> {
    // Kill any existing database server processes to avoid port conflicts
    #[cfg(target_os = "linux")]
    {
        // Linux: Kill MariaDB processes (mariadbd)
        // Also kill mysqld in case of mixed installations
        kill_existing_processes("mariadbd");
        kill_existing_processes("mysqld");
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Windows/macOS: Kill MySQL processes (mysqld)
        kill_existing_processes("mysqld");
    }

    // Initialize MySQL data directory if needed
    initialize_mysql_data_dir(paths)?;

    // Clean path and use proper Windows format for MySQL
    let data_dir_str = paths.mysql_data_dir.to_string_lossy().to_string();
    let data_dir_str = data_dir_str.trim_end_matches('\\').trim_end_matches('/');

    // Check if we need to create 127.0.0.1 user (first run)
    let user_created_flag = paths.mysql_data_dir.join(".user_127_0_0_1_created");
    let needs_init_file = !user_created_flag.exists();

    let init_file_path = if needs_init_file {
        // Create init file to add root@127.0.0.1 user
        let init_file = paths.logs_dir.join("mysql_init_user.sql");
        fs::write(&init_file, "CREATE USER IF NOT EXISTS 'root'@'127.0.0.1' IDENTIFIED BY '';\n\
            GRANT ALL PRIVILEGES ON *.* TO 'root'@'127.0.0.1' WITH GRANT OPTION;\n\
            FLUSH PRIVILEGES;\n")
            .map_err(|e| format!("Failed to create init file: {}", e))?;
        Some(init_file)
    } else {
        None
    };

    // Open log file with retry logic
    let log_path = paths.logs_dir.join("mysql.log");
    let log_file = open_log_file_with_retry(&log_path, "MariaDB")?;

    // Build MySQL command with optional init file
    let mut cmd = configure_no_window(Command::new(&paths.mysql));
    cmd.arg("--datadir")
        .arg(&data_dir_str)
        .arg("--port")
        .arg(service_process.port.to_string())
        .arg("--bind-address=127.0.0.1")
        .arg("--console")
        .arg("--skip-name-resolve");

    // Add init file on first run
    if let Some(ref init_file) = init_file_path {
        cmd.arg("--init-file")
            .arg(init_file);
    }

    let mut child = cmd
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| {
            let log_content = fs::read_to_string(&log_path).unwrap_or_else(|_| String::from("Could not read log"));
            format!("Failed to start MariaDB: {}\n\nMariaDB log:\n{}", e, log_content)
        })?;

    // Give MariaDB more time to start (it's slower than other services)
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            // Clean up init file if it exists
            if let Some(init_file) = init_file_path {
                let _ = fs::remove_file(&init_file);
            }
            Err(format!(
                "MariaDB exited immediately with status: {:?}\n\nCheck logs at: {:?}",
                status, log_path
            ))
        }
        Ok(None) => {
            // Mark user as created after successful start
            if needs_init_file {
                let _ = fs::write(&user_created_flag, "done");
                tracing::info!("MariaDB root@127.0.0.1 user created during startup");
            }
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            service_process.pid = service_process.child.as_ref().map(|c| c.id());
            Ok(())
        }
        Err(e) => {
            if let Some(init_file) = init_file_path {
                let _ = fs::remove_file(&init_file);
            }
            Err(format!("Failed to check MariaDB process: {}", e))
        }
    }
}

fn initialize_mysql_data_dir(paths: &RuntimePaths) -> Result<(), String> {
    crate::database::mysql::initialize_mysql(paths)
}

/// Kill any existing processes with the given name to avoid port conflicts
fn kill_existing_processes(process_name: &str) {
    super::killer::kill_existing_processes(process_name)
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
    fn test_process_manager_new() {
        let manager = ProcessManager::new();

        assert_eq!(manager.services.len(), 3);

        let caddy = manager.services.get(&ServiceType::Caddy).unwrap();
        assert_eq!(caddy.name, ServiceType::Caddy);
        assert_eq!(caddy.state, ServiceState::Stopped);
        assert_eq!(caddy.port, 8080);
        assert!(caddy.child.is_none());
    }

    #[test]
    fn test_process_manager_default() {
        let manager = ProcessManager::default();
        assert_eq!(manager.services.len(), 3);
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

        assert_eq!(statuses.len(), 3);

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
        is_port_available(8080) && is_port_available(9000) && is_port_available(3307)
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
        for service in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MySQL] {
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
