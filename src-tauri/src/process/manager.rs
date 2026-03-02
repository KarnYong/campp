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
fn configure_no_window(mut command: Command) -> Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(target_os = "windows"))]
fn configure_no_window(command: Command) -> Command {
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
}

/// Process manager for CAMPP services
pub struct ProcessManager {
    services: HashMap<ServiceType, ServiceProcess>,
    runtime_paths: Option<RuntimePaths>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut services = HashMap::new();

        for service_type in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MariaDB] {
            services.insert(
                service_type,
                ServiceProcess {
                    name: service_type,
                    child: None,
                    state: ServiceState::Stopped,
                    port: service_type.default_port(),
                    log_file: None,
                    error_message: None,
                },
            );
        }

        Self {
            services,
            runtime_paths: None,
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
            ServiceType::Caddy => start_caddy(service_process, &paths),
            ServiceType::PhpFpm => start_php_fpm(service_process, &paths),
            ServiceType::MariaDB => start_mariadb(service_process, &paths),
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

        if !service_process.state.is_running() {
            return Ok(());
        }

        service_process.state = ServiceState::Stopping;

        // Terminate the child process if it exists
        if let Some(ref mut child) = service_process.child {
            #[cfg(unix)]
            {
                // On Unix, send SIGTERM first
                use std::os::unix::process::CommandExt;
                let _ = child.kill();
            }

            #[cfg(windows)]
            {
                // On Windows, just kill the process
                let _ = child.kill();
            }

            // Wait for the process to exit
            let _ = child.wait();
        }

        service_process.child = None;
        service_process.state = ServiceState::Stopped;

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
                    }
                    Ok(None) => {
                        // Still running
                        service_process.state = ServiceState::Running;
                        service_process.error_message = None;
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
fn start_caddy(service_process: &mut ServiceProcess, paths: &RuntimePaths) -> Result<(), String> {
    // Kill any existing Caddy processes to avoid port conflicts
    kill_existing_processes("caddy");

    // Generate phpMyAdmin config if needed
    generate_phpmyadmin_config(paths)?;

    // Generate Caddyfile if it doesn't exist
    let caddyfile_path = paths.config_dir.join("Caddyfile");
    if !caddyfile_path.exists() {
        generate_caddyfile(&caddyfile_path, paths, service_process.port)?;
    }

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
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check Caddy process: {}", e)),
    }
}

/// Start PHP-FPM (using PHP-CGI for simplicity in MVP)
fn start_php_fpm(service_process: &mut ServiceProcess, paths: &RuntimePaths) -> Result<(), String> {
    // Kill any existing PHP-CGI processes to avoid port conflicts
    kill_existing_processes("php-cgi");

    // Generate php.ini if it doesn't exist
    if !paths.php_ini.exists() {
        generate_php_ini(&paths.php_ini)?;
    }

    // Open log file with retry logic
    let log_path = paths.logs_dir.join("php-fpm.log");
    let log_file = open_log_file_with_retry(&log_path, "PHP-FPM")?;

    // For MVP, we'll use PHP-CGI in a simple mode
    // In production, you'd want to use php-fpm with a proper configuration
    let mut child = configure_no_window(Command::new(&paths.php_cgi))
        .arg("-b")
        .arg("127.0.0.1:9000")
        .arg("-c")
        .arg(&paths.php_ini)
        .current_dir(&paths.config_dir)
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start PHP-CGI: {}", e))?;

    // Give it a moment to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("PHP-CGI exited immediately with status: {:?}", status)),
        Ok(None) => {
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check PHP-CGI process: {}", e)),
    }
}

/// Start MariaDB database server
fn start_mariadb(service_process: &mut ServiceProcess, paths: &RuntimePaths) -> Result<(), String> {
    // Kill any existing MariaDB processes to avoid port conflicts
    kill_existing_processes("mysqld");

    // Convert path to forward slashes for Windows compatibility
    let data_dir_str = paths.mysql_data_dir.to_string_lossy()
        .replace('\\', "/");

    // Open log file with retry logic
    let log_path = paths.logs_dir.join("mariadb.log");
    let log_file = open_log_file_with_retry(&log_path, "MariaDB")?;

    // Start MariaDB with minimal options for testing
    let mut child = configure_no_window(Command::new(&paths.mariadb))
        .arg("--no-defaults")
        .arg("--datadir")
        .arg(&data_dir_str)
        .arg("--port")
        .arg(service_process.port.to_string())
        .arg("--bind-address=127.0.0.1")
        .arg("--skip-grant-tables")  // Allow without password for initial setup
        .arg("--console")
        .stdout(Stdio::from(log_file.try_clone().unwrap()))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start MariaDB: {}", e))?;

    // Give MariaDB more time to start (it's slower than other services)
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("MariaDB exited immediately with status: {:?}", status)),
        Ok(None) => {
            service_process.child = Some(child);
            service_process.log_file = Some(log_path);
            Ok(())
        }
        Err(e) => Err(format!("Failed to check MariaDB process: {}", e)),
    }
}

/// Initialize MariaDB data directory
fn initialize_mariadb_data_dir(paths: &RuntimePaths) -> Result<(), String> {
    // Check if already initialized by looking for mysql system tables
    let mysql_dir = paths.mysql_data_dir.join("mysql");
    if mysql_dir.exists() {
        // Check if system tables exist
        let user_mrj_file = mysql_dir.join("user.MRJ");
        let db_file = mysql_dir.join("db.MYI");  // MyISAM index
        let db_innodb = mysql_dir.join("db.ibd");  // InnoDB table

        if user_mrj_file.exists() || db_file.exists() || db_innodb.exists() {
            // Already initialized
            return Ok(());
        }
    }

    // Create the data directory if it doesn't exist
    fs::create_dir_all(&paths.mysql_data_dir)
        .map_err(|e| format!("Failed to create MySQL data directory: {}", e))?;

    // Get clean path with forward slashes (Windows fix)
    // MariaDB on Windows has issues with backslashes, convert to forward slashes
    let data_dir_str = paths.mysql_data_dir.to_string_lossy()
        .replace('\\', "/");

    // For some MariaDB versions, we use --bootstrap instead of --initialize-insecure
    // Note: This may take 30+ seconds on first run
    let mut child = Command::new(&paths.mariadb)
        .arg("--bootstrap")
        .arg("--datadir")
        .arg(&data_dir_str)
        .arg("--lc-messages-dir")
        .arg(paths.mariadb.parent().unwrap().join("share"))  // Messages directory
        .current_dir(&paths.config_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run MariaDB initialization: {}", e))?;

    // Wait for initialization to complete (can take up to 60 seconds)
    let timeout = std::time::Duration::from_secs(60);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                } else {
                    // Bootstrap failure is OK - try without bootstrap
                    // The data directory might be ready to use
                    return Ok(());
                }
            }
            Ok(None) => {
                // Still running
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    // Timeout might mean it's stuck, but try to continue anyway
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(e) => {
                // Error waiting, but try to continue
                return Ok(());
            }
        }
    }
}

/// Kill any existing processes with the given name to avoid port conflicts
fn kill_existing_processes(process_name: &str) {
    #[cfg(windows)]
    {
        // Use taskkill on Windows to forcefully terminate processes by name
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", &format!("{}.exe", process_name)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        // Give the process time to terminate and release ports
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    #[cfg(unix)]
    {
        // Use pkill on Unix to forcefully terminate processes by name
        let _ = Command::new("pkill")
            .args(["-9", process_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        // Give the process time to terminate and release ports
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Generate a basic Caddyfile
fn generate_caddyfile(path: &PathBuf, paths: &RuntimePaths, port: u16) -> Result<(), String> {
    // Convert paths to use forward slashes for Caddyfile (cross-platform compatibility)
    let projects = paths.projects_dir
        .to_str()
        .ok_or("Invalid project path")?
        .replace('\\', "/");
    let log_file = paths.logs_dir.join("caddy-access.log")
        .to_str()
        .ok_or("Invalid log path")?
        .replace('\\', "/");
    let phpmyadmin = paths.phpmyadmin
        .to_str()
        .ok_or("Invalid phpMyAdmin path")?
        .replace('\\', "/");

    // Build the Caddyfile content
    let mut content = String::new();
    content.push_str(&format!(":{} {{\n", port));
    content.push_str("    # phpMyAdmin - must come before global directives\n");
    content.push_str("    # Redirect /phpmyadmin to /phpmyadmin/\n");
    content.push_str("    redir /phpmyadmin /phpmyadmin/\n");
    content.push_str("\n");
    content.push_str("    # Handle phpMyAdmin requests - handle_path strips the /phpmyadmin prefix\n");
    content.push_str("    handle_path /phpmyadmin/* {\n");
    content.push_str(&format!("        root * \"{}\"\n", phpmyadmin));
    content.push_str("        php_fastcgi 127.0.0.1:9000\n");
    content.push_str("        file_server browse\n");
    content.push_str("    }\n");
    content.push_str("\n");
    content.push_str("    # Root directory for serving files (default project root)\n");
    content.push_str(&format!("    root * \"{}\"\n", projects));
    content.push_str("\n");
    content.push_str("    # Enable PHP for all other requests\n");
    content.push_str("    php_fastcgi 127.0.0.1:9000\n");
    content.push_str("\n");
    content.push_str("    # File server for project files\n");
    content.push_str("    file_server browse\n");
    content.push_str("\n");
    content.push_str("    # Logging\n");
    content.push_str("    log {\n");
    content.push_str(&format!("        output file \"{}\"\n", log_file));
    content.push_str("        format json\n");
    content.push_str("    }\n");
    content.push_str("\n");
    content.push_str("    # Encode responses\n");
    content.push_str("    encode gzip\n");
    content.push_str("\n");
    content.push_str("    # Security headers\n");
    content.push_str("    header {\n");
    content.push_str("        X-Content-Type-Options nosniff\n");
    content.push_str("        X-Frame-Options SAMEORIGIN\n");
    content.push_str("        Referrer-Policy no-referrer\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create Caddyfile: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write Caddyfile: {}", e))?;

    Ok(())
}

/// Generate a basic php.ini
fn generate_php_ini(path: &PathBuf) -> Result<(), String> {
    let php_ini_content = r#"; CAMPP PHP Configuration
; Basic PHP settings for development

[PHP]
; Error reporting - suppress deprecation warnings for phpMyAdmin compatibility with PHP 8.3
error_reporting = E_ALL & ~E_DEPRECATED
display_errors = On
display_startup_errors = On
log_errors = On
error_log = ""

; Maximum execution time
max_execution_time = 300

; Memory limit
memory_limit = 256M

; POST data limit
post_max_size = 100M
upload_max_filesize = 100M

; Date timezone
date.timezone = UTC

; Extensions
extension_dir = "ext"
extension=curl
extension=mbstring
extension=mysqli
extension=openssl
extension=pdo_mysql
extension=zlib

; Session settings
session.save_path = "/tmp"

; CGI settings
cgi.force_redirect = 0
cgi.fix_pathinfo = 1
"#;

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create php.ini: {}", e))?;
    file.write_all(php_ini_content.as_bytes())
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok(())
}

/// Generate phpMyAdmin config.inc.php
fn generate_phpmyadmin_config(paths: &RuntimePaths) -> Result<(), String> {
    let config_path = paths.phpmyadmin.join("config.inc.php");

    // Only generate if it doesn't exist (don't overwrite user customizations)
    if config_path.exists() {
        return Ok(());
    }

    // Generate a 32-byte blowfish secret for cookie encryption
    let blowfish_secret: String = (0..32)
        .map(|_| {
            const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let idx = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as usize % CHARSET.len();
            CHARSET[idx] as char
        })
        .collect();

    let config_content = format!(r#"<?php
/**
 * CAMPP phpMyAdmin Configuration
 * Generated automatically - you can customize this file
 */

// Cookie encryption key (exactly 32 bytes)
$cfg['blowfish_secret'] = '{}';

// Server configuration
$i = 0;
$i++;
$cfg['Servers'][$i]['auth_type'] = 'config';
$cfg['Servers'][$i]['user'] = 'root';
$cfg['Servers'][$i]['password'] = '';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '3307';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = true;

// Upload directory
$cfg['UploadDir'] = '';
$cfg['SaveDir'] = '';

// Temp directory
$cfg['TempDir'] = './tmp/';

// Disable configuration storage warning (optional advanced features)
$cfg['PmaNoRelation_DisableWarning'] = true;

// Default language
$cfg['DefaultLang'] = 'en';

// Theme
$cfg['ThemeDefault'] = 'pmahomme';
"#, blowfish_secret);

    let mut file = File::create(&config_path)
        .map_err(|e| format!("Failed to create phpMyAdmin config: {}", e))?;
    file.write_all(config_content.as_bytes())
        .map_err(|e| format!("Failed to write phpMyAdmin config: {}", e))?;

    // Create temp directory for phpMyAdmin if it doesn't exist
    let tmp_dir = paths.phpmyadmin.join("tmp");
    if !tmp_dir.exists() {
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("Failed to create phpMyAdmin tmp directory: {}", e))?;
    }

    Ok(())
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
        assert_eq!(manager.status(ServiceType::MariaDB), ServiceState::Stopped);
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

        let service = manager.services.get_mut(&ServiceType::MariaDB).unwrap();
        service.state = ServiceState::Error;
        service.error_message = Some("Test error".to_string());

        assert_eq!(manager.status(ServiceType::MariaDB), ServiceState::Error);

        let statuses = manager.get_all_statuses();
        let mariadb_info = statuses.get(&ServiceType::MariaDB).unwrap();
        assert_eq!(mariadb_info.state, ServiceState::Error);
        assert_eq!(mariadb_info.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_update_health_with_no_processes() {
        let mut manager = ProcessManager::new();

        manager.update_health();

        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::PhpFpm), ServiceState::Stopped);
        assert_eq!(manager.status(ServiceType::MariaDB), ServiceState::Stopped);
    }

    #[test]
    fn test_port_assignment_for_services() {
        let manager = ProcessManager::new();

        let caddy = manager.services.get(&ServiceType::Caddy).unwrap();
        assert_eq!(caddy.port, 8080);

        let php = manager.services.get(&ServiceType::PhpFpm).unwrap();
        assert_eq!(php.port, 9000);

        let mariadb = manager.services.get(&ServiceType::MariaDB).unwrap();
        assert_eq!(mariadb.port, 3307);
    }

    #[test]
    fn test_multiple_services_have_independent_states() {
        let mut manager = ProcessManager::new();

        let caddy = manager.services.get_mut(&ServiceType::Caddy).unwrap();
        caddy.state = ServiceState::Running;

        let php = manager.services.get_mut(&ServiceType::PhpFpm).unwrap();
        php.state = ServiceState::Starting;

        let mariadb = manager.services.get_mut(&ServiceType::MariaDB).unwrap();
        mariadb.state = ServiceState::Stopped;

        assert_eq!(manager.status(ServiceType::Caddy), ServiceState::Running);
        assert_eq!(manager.status(ServiceType::PhpFpm), ServiceState::Starting);
        assert_eq!(manager.status(ServiceType::MariaDB), ServiceState::Stopped);
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
            paths.caddy.exists() && paths.php_cgi.exists() && paths.mariadb.exists()
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
        for service in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MariaDB] {
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
                ServiceType::MariaDB => "mariadb.log",
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
    fn test_integration_start_stop_mariadb() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let mut manager = match setup_test() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping: {}", e);
                return;
            }
        };

        // Start MariaDB
        let result = manager.start(ServiceType::MariaDB);
        if let Err(e) = &result {
            let logs = read_log_file(&manager, ServiceType::MariaDB);
            panic!("MariaDB failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for MariaDB to be running (longer timeout)
        let is_running = wait_for_state(&mut manager, ServiceType::MariaDB, ServiceState::Running, 15);
        assert!(is_running, "MariaDB should be in Running state");

        // Stop MariaDB
        manager.stop(ServiceType::MariaDB).expect("MariaDB should stop");

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
        if let Err(e) = manager.start(ServiceType::MariaDB) {
            let logs = read_log_file(&manager, ServiceType::MariaDB);
            panic!("MariaDB failed to start: {}\n\nLogs:\n{}", e, logs);
        }

        // Wait for all to be running
        let caddy_running = wait_for_state(&mut manager, ServiceType::Caddy, ServiceState::Running, 10);
        let php_running = wait_for_state(&mut manager, ServiceType::PhpFpm, ServiceState::Running, 10);
        let mariadb_running = wait_for_state(&mut manager, ServiceType::MariaDB, ServiceState::Running, 20);

        if !caddy_running {
            let logs = read_log_file(&manager, ServiceType::Caddy);
            panic!("Caddy not running. Logs:\n{}", logs);
        }
        if !php_running {
            let logs = read_log_file(&manager, ServiceType::PhpFpm);
            panic!("PHP not running. Logs:\n{}", logs);
        }
        if !mariadb_running {
            let logs = read_log_file(&manager, ServiceType::MariaDB);
            panic!("MariaDB not running. Logs:\n{}", logs);
        }

        // Stop all services
        manager.stop(ServiceType::MariaDB).ok();
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
