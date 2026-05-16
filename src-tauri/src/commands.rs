//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceState, ServiceType};
use crate::runtime::deps::DependencyCheckResult;
use crate::runtime::downloader::{DownloadProgress, RuntimeDownloader};
use crate::runtime::packages::{PackageSelection, PackagesConfig};
use crate::config::AppSettings;
use crate::AppState;
use crate::ProcessManager;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

/// Open a folder in the system's file explorer using tauri-plugin-opener
///
/// Only allows opening known app directories (runtime, download, config, project root).
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    use tauri_plugin_opener::reveal_item_in_dir;

    let path_obj = std::path::Path::new(&path);

    // Build allowlist of known directories
    let mut allowed_dirs = Vec::new();

    let downloader = crate::runtime::downloader::RuntimeDownloader::new()?;
    if let Ok(runtime_dir) = downloader.get_runtime_dir() {
        allowed_dirs.push(runtime_dir);
    }
    allowed_dirs.push(std::env::temp_dir().join("campp-download"));

    let settings = crate::config::AppSettings::load();
    allowed_dirs.push(std::path::PathBuf::from(&settings.project_root));

    if let Some(data_dir) = dirs::data_local_dir() {
        allowed_dirs.push(data_dir.join("campp"));
    }

    // Canonicalize the requested path and check it's under an allowed directory
    let canonical = path_obj.canonicalize()
        .map_err(|e| format!("Path does not exist: {}", e))?;

    let is_allowed = allowed_dirs.iter().any(|dir| {
        dir.canonicalize().map(|d| canonical.starts_with(d)).unwrap_or(false)
    });

    if !is_allowed {
        return Err(format!("Access denied: path is not within an allowed directory"));
    }

    reveal_item_in_dir(&canonical)
        .map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(())
}

/// Open the user manual in the system's default application using tauri-plugin-opener
///
/// This command locates the MANUAL.html resource file and reveals it in the
/// file manager using tauri-plugin-opener for cross-platform compatibility.
/// Users can then open it with their preferred browser or HTML viewer.
#[tauri::command]
pub async fn open_manual(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::reveal_item_in_dir;

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let manual_path = resource_dir.join("MANUAL.html");

    // Ensure the manual exists
    if !manual_path.exists() {
        return Err(format!("Manual not found at: {}", manual_path.display()));
    }

    // Use tauri-plugin-opener to reveal the file in the file manager
    // This is cross-platform and lets the user choose how to open it
    reveal_item_in_dir(&manual_path)
        .map_err(|e| format!("Failed to open manual: {}", e))?;

    Ok(())
}

// Global state for download progress
static DOWNLOAD_PROGRESS: Mutex<Option<DownloadProgress>> = Mutex::new(None);

/// Start a service
#[tauri::command]
pub async fn start_service(
    service: ServiceType,
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let pm = state.process_manager.clone();

    tokio::task::spawn_blocking(move || {
        let mut manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        // Initialize if needed - propagate error if this fails
        manager.initialize()?;

        // Start the service
        let result = manager.start(service);

        // Update health and return statuses regardless of start result
        manager.update_health();
        let statuses = manager.get_all_statuses();

        result?;
        Ok(statuses)
    }).await.map_err(|e| format!("Task error: {}", e))?
}

/// Stop a service
#[tauri::command]
pub async fn stop_service(
    service: ServiceType,
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let pm = state.process_manager.clone();

    tokio::task::spawn_blocking(move || {
        let mut manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        // Stop the service
        manager.stop(service)?;

        // Update health and return statuses
        manager.update_health();
        Ok(manager.get_all_statuses())
    }).await.map_err(|e| format!("Task error: {}", e))?
}

/// Restart a service
#[tauri::command]
pub async fn restart_service(
    service: ServiceType,
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let pm = state.process_manager.clone();

    tokio::task::spawn_blocking(move || {
        let mut manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        // Initialize if needed
        manager.initialize()?;

        // Restart the service
        let result = manager.restart(service);

        manager.update_health();
        let statuses = manager.get_all_statuses();

        result?;
        Ok(statuses)
    }).await.map_err(|e| format!("Task error: {}", e))?
}

/// Get the status of all services
#[tauri::command]
pub async fn get_all_statuses(
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    manager.update_health();
    Ok(manager.get_all_statuses())
}

/// Get app settings
#[tauri::command]
pub async fn get_settings() -> Result<crate::config::AppSettings, String> {
    Ok(crate::config::AppSettings::load())
}

/// Save app settings
#[tauri::command]
pub async fn save_settings(settings: crate::config::AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let old_settings = crate::config::AppSettings::load();
    let mysql_changed = old_settings.mysql_root_password != settings.mysql_root_password;
    let postgres_changed = old_settings.postgres_root_password != settings.postgres_root_password;

    // Save the settings first
    settings.save()?;

    let pm = state.process_manager.clone();

    tokio::task::spawn_blocking(move || {
        let mut manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        // If PostgreSQL password changed, remove .password_set flag so it gets re-applied on start
        if postgres_changed {
            if let Some(paths) = manager.get_runtime_paths() {
                let flag = paths.pgsql_data_dir.join(".password_set");
                let _ = fs::remove_file(&flag);
            }
        }

        // Get the current running services before updating ports
        let running_services: Vec<ServiceType> = manager.get_all_statuses()
            .iter()
            .filter(|(_, s)| s.state == ServiceState::Running)
            .map(|(ty, _)| *ty)
            .collect();

        // Update ports in the process manager
        manager.update_ports(&settings);

        // If MySQL password changed and MySQL is running, apply inline before restart
        if mysql_changed && running_services.contains(&ServiceType::MySQL) {
            apply_mysql_password(&manager, &settings.mysql_root_password);
        }
        // If PostgreSQL password changed and PostgreSQL is running, apply inline
        if postgres_changed && running_services.contains(&ServiceType::PostgreSQL) {
            apply_postgres_password(&manager, &settings.postgres_root_password);
        }

        // Restart any running services with new configuration
        for service in running_services {
            let _ = manager.stop(service);
            let _ = manager.start(service);
        }

        Ok(())
    }).await.map_err(|e| format!("Task error: {}", e))?
}

/// Validate settings (check port conflicts, valid paths)
#[tauri::command]
pub async fn validate_settings(settings: crate::config::AppSettings) -> Result<Vec<String>, Vec<String>> {
    settings.validate()
}

/// Check if specific ports are available
#[tauri::command]
pub async fn check_ports(web_port: u16, php_port: u16, mysql_port: u16, postgres_port: u16) -> serde_json::Value {
    use crate::config::is_port_available;

    serde_json::json!({
        "web": {
            "port": web_port,
            "available": is_port_available(web_port)
        },
        "php": {
            "port": php_port,
            "available": is_port_available(php_port)
        },
        "mysql": {
            "port": mysql_port,
            "available": is_port_available(mysql_port)
        },
        "postgres": {
            "port": postgres_port,
            "available": is_port_available(postgres_port)
        }
    })
}

/// Check if runtime binaries are already installed
#[tauri::command]
pub async fn check_runtime_installed() -> Result<bool, String> {
    let downloader = RuntimeDownloader::new()?;
    Ok(downloader.is_installed())
}

/// Reset installation (for testing/debug - deletes runtime directory)
#[tauri::command]
pub async fn reset_installation(state: State<'_, AppState>) -> Result<String, String> {
    let pm = state.process_manager.clone();
    do_reset_installation(pm).await
}

/// Core reset logic, usable from both Tauri commands and menu handlers
pub async fn do_reset_installation(pm: Arc<Mutex<ProcessManager>>) -> Result<String, String> {
    // Stop all managed services
    let pm_clone = pm.clone();
    tokio::task::spawn_blocking(move || {
        let mut manager = pm_clone.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;
        let _ = manager.stop_all();
        Ok::<(), String>(())
    }).await.map_err(|e| format!("Task error: {}", e))??;

    // Kill any lingering processes that may still hold file locks
    kill_runtime_processes();

    let paths = crate::runtime::locator::get_app_data_paths()?;

    // Delete runtime binaries, config, data, and logs — but preserve projects
    for dir in [&paths.runtime_dir, &paths.config_dir, &paths.mysql_data_dir, &paths.pgsql_data_dir, &paths.logs_dir] {
        if dir.exists() {
            remove_dir_all_with_retry(dir)?;
        }
    }

    // Also delete the settings file so the wizard starts fresh
    let settings_path = paths.config_dir.join("settings.json");
    if settings_path.exists() {
        let _ = fs::remove_file(&settings_path);
    }

    Ok("Installation reset. Run the app again to see first-run wizard.".to_string())
}

fn kill_runtime_processes() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "caddy.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "php-cgi.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "mysqld.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "postgres.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
    }

    #[cfg(unix)]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-9", "caddy"])
            .output();
        let _ = std::process::Command::new("pkill")
            .args(["-9", "php-cgi"])
            .output();
        let _ = std::process::Command::new("pkill")
            .args(["-9", "mysqld"])
            .output();
        let _ = std::process::Command::new("pkill")
            .args(["-9", "postgres"])
            .output();
    }

    // Give processes time to fully exit and release file locks
    std::thread::sleep(std::time::Duration::from_millis(1000));
}

fn remove_dir_all_with_retry(path: &std::path::Path) -> Result<(), String> {
    let mut attempts = 0;
    let max_attempts = 5;

    loop {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(500 * attempts as u64));
                // Re-kill processes in case they restarted or are slow to release locks
                kill_runtime_processes();
            }
            Err(e) => return Err(format!(
                "Failed to remove runtime directory after {} attempts: {}. \
                 Please stop all services manually and try again.",
                max_attempts, e
            )),
        }
    }
}

/// Get the runtime directory path
#[tauri::command]
pub async fn get_runtime_dir() -> Result<String, String> {
    let downloader = RuntimeDownloader::new()?;
    downloader
        .get_runtime_dir()
        .map(|p| p.to_string_lossy().to_string())
}

/// Get the installation directory (where the exe is located)
#[tauri::command]
pub async fn get_install_dir() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        let install_dir = exe_path.parent()
            .ok_or("Failed to get installation directory")?;
        Ok(install_dir.to_string_lossy().to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows, return the project root as the install dir concept doesn't apply
        let settings = crate::config::AppSettings::load();
        Ok(settings.project_root)
    }
}

/// Get the download directory path (where ZIP files are stored)
#[tauri::command]
pub async fn get_download_dir() -> Result<String, String> {
    let temp_dir = std::env::temp_dir().join("campp-download");
    Ok(temp_dir.to_string_lossy().to_string())
}

/// Download and install runtime binaries
#[tauri::command]
pub async fn download_runtime(app: tauri::AppHandle) -> Result<String, String> {
    // Ensure config is loaded from Tauri's resource directory
    if let Ok(resource_dir) = app.path().resource_dir() {
        crate::runtime::packages::load_config_from_resource_dir(&resource_dir);
    }
    let downloader = RuntimeDownloader::new()?;
    let app_clone = app.clone();

    // Emit progress updates via Tauri events
    downloader
        .download_all(Box::new(move |progress| {
            let _ = app_clone.emit("download-progress", &progress);

            // Store latest progress
            if let Ok(mut p) = DOWNLOAD_PROGRESS.lock() {
                *p = Some(progress);
            }
        }))
        .await?;

    Ok("Runtime binaries installed successfully".to_string())
}

/// Stop all running services (for cleanup on app exit)
#[tauri::command]
pub async fn cleanup_all_services(state: State<'_, AppState>) -> Result<String, String> {
    let pm = state.process_manager.clone();

    tokio::task::spawn_blocking(move || {
        let mut manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        manager.stop_all()?;
        Ok("All services stopped".to_string())
    }).await.map_err(|e| format!("Task error: {}", e))?
}

/// Get all available runtime packages
#[tauri::command]
pub async fn get_available_packages_cmd() -> Result<PackagesConfig, String> {
    Ok(crate::runtime::packages::get_available_packages())
}

/// Download and install runtime binaries with custom package selection
#[tauri::command]
pub async fn download_runtime_with_packages(
    package_selection: PackageSelection,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Ensure config is loaded from Tauri's resource directory
    if let Ok(resource_dir) = app.path().resource_dir() {
        crate::runtime::packages::load_config_from_resource_dir(&resource_dir);
    }
    let downloader = RuntimeDownloader::with_packages(package_selection)?;
    let app_clone = app.clone();

    // Emit progress updates via Tauri events
    downloader
        .download_all(Box::new(move |progress| {
            let _ = app_clone.emit("download-progress", &progress);

            // Store latest progress
            if let Ok(mut p) = DOWNLOAD_PROGRESS.lock() {
                *p = Some(progress);
            }
        }))
        .await?;

    Ok("Runtime binaries installed successfully".to_string())
}

/// Get the current package selection from settings
#[tauri::command]
pub async fn get_package_selection() -> Result<PackageSelection, String> {
    let settings = AppSettings::load();
    Ok(settings.package_selection)
}

/// Update package selection in settings (without downloading)
#[tauri::command]
pub async fn update_package_selection(
    package_selection: PackageSelection,
) -> Result<(), String> {
    let mut settings = AppSettings::load();
    settings.package_selection = package_selection;
    settings.save()?;
    Ok(())
}

/// Update database root passwords in settings and apply to running databases
#[tauri::command]
pub async fn update_db_passwords(
    state: State<'_, AppState>,
    mysql_password: String,
    postgres_password: String,
) -> Result<(), String> {
    let old_settings = AppSettings::load();

    let mysql_changed = old_settings.mysql_root_password != mysql_password;
    let postgres_changed = old_settings.postgres_root_password != postgres_password;

    // Save new passwords to settings file
    let mut settings = old_settings;
    settings.mysql_root_password = mysql_password.clone();
    settings.postgres_root_password = postgres_password.clone();
    settings.save()?;

    // Apply password changes to running databases
    let pm = state.process_manager.clone();
    tokio::task::spawn_blocking(move || {
        let manager = pm.lock()
            .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

        if mysql_changed {
            apply_mysql_password(&manager, &mysql_password);
        }
        if postgres_changed {
            apply_postgres_password(&manager, &postgres_password);
        }

        Ok::<(), String>(())
    }).await.map_err(|e| format!("Task error: {}", e))?
}

fn apply_mysql_password(manager: &crate::ProcessManager, password: &str) {
    let paths = match manager.get_runtime_paths() {
        Some(p) => p,
        None => return,
    };

    if manager.status(ServiceType::MySQL) != ServiceState::Running {
        return;
    }

    let password_sql = if password.is_empty() {
        "''".to_string()
    } else {
        format!("'{}'", password.replace('\'', "\\'"))
    };

    // mysql client is in the same bin/ directory as mysqld
    let mysql_bin_dir = paths.mysql.parent().unwrap_or_else(|| std::path::Path::new(""));
    #[cfg(target_os = "windows")]
    let mysql_client = mysql_bin_dir.join("mysql.exe");
    #[cfg(not(target_os = "windows"))]
    let mysql_client = mysql_bin_dir.join("mysql");

    if !mysql_client.exists() {
        tracing::warn!("MySQL client not found at {:?}, password will apply on next restart", mysql_client);
        return;
    }

    let port_str = manager.get_service_port(ServiceType::MySQL)
        .unwrap_or(crate::config::settings::DEFAULT_PORTS.mysql)
        .to_string();

    let old_password = manager.get_settings().mysql_root_password.clone();

    let sql = format!(
        "ALTER USER 'root'@'127.0.0.1' IDENTIFIED BY {}; FLUSH PRIVILEGES;",
        password_sql
    );

    let mut cmd = crate::process::manager::configure_no_window(Command::new(&mysql_client));
    cmd.arg("-h").arg("127.0.0.1")
        .arg("-P").arg(&port_str)
        .arg("-u").arg("root");

    if !old_password.is_empty() {
        cmd.arg(format!("-p{}", old_password));
    }

    cmd.arg("-e").arg(&sql)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    match cmd.status() {
        Ok(s) if s.success() => tracing::info!("MySQL password updated successfully"),
        _ => tracing::warn!("Failed to update MySQL password inline, will apply on next restart"),
    }
}

fn apply_postgres_password(manager: &crate::ProcessManager, password: &str) {
    let paths = match manager.get_runtime_paths() {
        Some(p) => p,
        None => return,
    };

    if manager.status(ServiceType::PostgreSQL) != ServiceState::Running {
        // Service not running — remove flag so password gets set on next start
        let flag = paths.pgsql_data_dir.join(".password_set");
        let _ = fs::remove_file(&flag);
        tracing::info!("PostgreSQL not running, password will be applied on next start");
        return;
    }

    #[cfg(target_os = "windows")]
    let psql_bin = paths.pgsql_dir.join("bin").join("psql.exe");
    #[cfg(not(target_os = "windows"))]
    let psql_bin = paths.pgsql_dir.join("bin").join("psql");

    if !psql_bin.exists() {
        return;
    }

    let port_str = manager.get_service_port(ServiceType::PostgreSQL)
        .unwrap_or(crate::config::settings::DEFAULT_PORTS.postgres)
        .to_string();

    let escaped_pw = password.replace('\'', "''");
    let sql = format!("ALTER USER root PASSWORD '{}';", escaped_pw);

    let mut cmd = crate::process::manager::configure_no_window(Command::new(&psql_bin));
    cmd.arg("-h").arg("127.0.0.1")
        .arg("-p").arg(&port_str)
        .arg("-U").arg("root")
        .arg("-d").arg("postgres")
        .arg("-c").arg(&sql)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let old_password = manager.get_settings().postgres_root_password.clone();
    if !old_password.is_empty() {
        cmd.env("PGPASSWORD", &old_password);
    }
    cmd.env("PGCONNECT_TIMEOUT", "5");

    #[cfg(unix)]
    {
        let lib_dir = paths.pgsql_dir.join("lib");
        let lib_path = lib_dir.to_string_lossy().to_string();
        if let Ok(existing) = std::env::var("LD_LIBRARY_PATH") {
            cmd.env("LD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
        } else {
            cmd.env("LD_LIBRARY_PATH", &lib_path);
        }
    }

    match cmd.output() {
        Ok(output) if output.status.success() => {
            tracing::info!("PostgreSQL password updated successfully");
        }
        Ok(output) => {
            tracing::warn!("Failed to update PostgreSQL password: {}",
                String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            tracing::warn!("Failed to run psql to update password: {}", e);
        }
    }
}

/// Get the selected package IDs from runtime-config.json
#[tauri::command]
pub async fn get_selected_package_ids() -> Result<PackageSelection, String> {
    Ok(crate::runtime::packages::get_selected_package_ids())
}

/// Reload the runtime configuration from runtime-config.json
#[tauri::command]
pub async fn reload_runtime_config() -> Result<String, String> {
    crate::runtime::packages::reload_runtime_config();
    Ok("Runtime configuration reloaded successfully".to_string())
}

/// Get the installed runtime versions
#[tauri::command]
pub async fn get_installed_versions() -> Result<std::collections::HashMap<String, String>, String> {
    let downloader = RuntimeDownloader::new()?;
    let runtime_dir = downloader.get_runtime_dir()?;

    let mut versions = std::collections::HashMap::new();

    // Read version from marker files
    for component in ["caddy", "php", "mysql", "mariadb", "phpmyadmin", "postgresql", "adminer"] {
        let marker_file = runtime_dir.join(format!("{}_installed.txt", component));
        if let Ok(content) = fs::read_to_string(&marker_file) {
            // Parse version from format: "version=1.2.3\ninstalled_at=..."
            for line in content.lines() {
                if let Some(version) = line.strip_prefix("version=") {
                    versions.insert(component.to_string(), version.to_string());
                    break;
                }
            }
        }
    }

    Ok(versions)
}

/// Check for existing components before download
#[tauri::command]
pub async fn check_existing_components() -> Result<std::collections::HashMap<String, String>, String> {
    let downloader = RuntimeDownloader::new()?;
    Ok(downloader.get_installed_components())
}

/// Download and install runtime binaries with option to skip existing components
#[tauri::command]
pub async fn download_runtime_with_skip(
    package_selection: PackageSelection,
    skip_list: Vec<String>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Ensure config is loaded from Tauri's resource directory
    if let Ok(resource_dir) = app.path().resource_dir() {
        crate::runtime::packages::load_config_from_resource_dir(&resource_dir);
    }
    let downloader = RuntimeDownloader::with_packages(package_selection)?;
    let app_clone = app.clone();

    // Convert Vec<String> to Vec<&str> for the skip_list
    let skip_refs: Vec<&str> = skip_list.iter().map(|s| s.as_str()).collect();

    // Emit progress updates via Tauri events
    downloader
        .download_all_with_skip(Box::new(move |progress| {
            let _ = app_clone.emit("download-progress", &progress);

            // Store latest progress
            if let Ok(mut p) = DOWNLOAD_PROGRESS.lock() {
                *p = Some(progress);
            }
        }), &skip_refs)
        .await?;

    Ok("Runtime binaries installed successfully".to_string())
}

/// Check system dependencies (libraries required by runtime binaries)
#[tauri::command]
pub async fn check_system_dependencies() -> DependencyCheckResult {
    crate::runtime::deps::check_system_dependencies()
}

/// Uninstall a specific component (stops service if running, removes binary files)
#[tauri::command]
pub async fn uninstall_component(
    component: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let valid_components = ["caddy", "php", "mysql", "mariadb", "phpmyadmin", "postgresql", "adminer"];
    if !valid_components.contains(&component.as_str()) {
        return Err(format!("Invalid component: {}", component));
    }

    // Stop the corresponding service if it maps to one
    let service_type = match component.as_str() {
        "caddy" => Some(ServiceType::Caddy),
        "php" => Some(ServiceType::PhpFpm),
        "mysql" | "mariadb" => Some(ServiceType::MySQL),
        "postgresql" => Some(ServiceType::PostgreSQL),
        _ => None,
    };

    if let Some(st) = service_type {
        let pm = state.process_manager.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let mut manager = pm.lock()
                .map_err(|e| format!("Failed to acquire lock: {}", e))?;
            let _ = manager.stop(st);
            Ok::<(), String>(())
        }).await;
    }

    let downloader = RuntimeDownloader::new()?;
    downloader.uninstall_component(&component)?;

    Ok(())
}

/// Get debug info for troubleshooting (version, paths, config status)
#[tauri::command]
pub async fn get_debug_info(app: tauri::AppHandle) -> serde_json::Value {
    use serde_json::json;

    let version = env!("CARGO_PKG_VERSION").to_string();

    let resource_dir = app.path().resource_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|e| format!("ERROR: {}", e));
    let config_in_resource = std::path::Path::new(&resource_dir).join("runtime-config.json").exists();

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|e| format!("ERROR: {}", e));

    let mut runtime_dir = "ERROR".to_string();
    if let Ok(dl) = RuntimeDownloader::new() {
        runtime_dir = dl.get_runtime_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|e| e);
    }

    let config_loaded = crate::runtime::packages::get_config().is_some();

    let resource_files: Vec<String> = std::fs::read_dir(&resource_dir)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();

    json!({
        "version": version,
        "exePath": exe_path,
        "resourceDir": resource_dir,
        "configInResourceDir": config_in_resource,
        "resourceDirFiles": resource_files,
        "runtimeDir": runtime_dir,
        "configLoaded": config_loaded,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}
