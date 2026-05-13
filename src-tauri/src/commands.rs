//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceState, ServiceType};
use crate::runtime::deps::DependencyCheckResult;
use crate::runtime::downloader::{DownloadProgress, RuntimeDownloader};
use crate::runtime::packages::{PackageSelection, PackagesConfig};
use crate::config::AppSettings;
use crate::AppState;
use std::fs;
use std::sync::Mutex;
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
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    // Initialize if needed - propagate error if this fails
    manager.initialize()?;

    // Start the service
    let result = manager.start(service);

    // Update health and return statuses regardless of start result
    // This ensures the frontend sees the error state
    manager.update_health();
    let statuses = manager.get_all_statuses();

    // Return error after getting statuses so frontend can see the error state
    result?;
    Ok(statuses)
}

/// Stop a service
#[tauri::command]
pub async fn stop_service(
    service: ServiceType,
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    // Stop the service
    manager.stop(service)?;

    // Update health and return statuses
    manager.update_health();
    Ok(manager.get_all_statuses())
}

/// Restart a service
#[tauri::command]
pub async fn restart_service(
    service: ServiceType,
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    // Initialize if needed - propagate error if this fails
    manager.initialize()?;

    // Restart the service
    let result = manager.restart(service);

    // Update health and return statuses regardless of restart result
    manager.update_health();
    let statuses = manager.get_all_statuses();

    // Return error after getting statuses so frontend can see the error state
    result?;
    Ok(statuses)
}

/// Get the status of all services
#[tauri::command]
pub async fn get_all_statuses(
    state: State<'_, AppState>,
) -> Result<ServiceMap, String> {
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    // Update health before returning statuses
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
    // Save the settings first
    settings.save()?;

    // Update the ProcessManager with new settings
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    // Get the current running services before updating ports
    let running_services: Vec<ServiceType> = manager.get_all_statuses()
        .iter()
        .filter(|(_, s)| s.state == ServiceState::Running)
        .map(|(ty, _)| *ty)
        .collect();

    // Update ports in the process manager
    manager.update_ports(&settings);

    // Restart any running services with new configuration
    for service in running_services {
        // Stop the service
        let _ = manager.stop(service);
        // Start it again with new port settings
        let _ = manager.start(service);
    }

    Ok(())
}

/// Validate settings (check port conflicts, valid paths)
#[tauri::command]
pub async fn validate_settings(settings: crate::config::AppSettings) -> Result<Vec<String>, Vec<String>> {
    settings.validate()
}

/// Check if specific ports are available
#[tauri::command]
pub async fn check_ports(web_port: u16, php_port: u16, mysql_port: u16) -> serde_json::Value {
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
pub async fn reset_installation() -> Result<String, String> {
    let downloader = RuntimeDownloader::new()?;
    let runtime_dir = downloader.get_runtime_dir().map_err(|e| e.to_string())?;

    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir)
            .map_err(|e| format!("Failed to remove runtime directory: {}", e))?;
    }

    Ok("Installation reset. Run the app again to see first-run wizard.".to_string())
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
    let mut manager = state.process_manager.lock()
        .map_err(|e| format!("Failed to acquire process manager lock: {}", e))?;

    manager.stop_all()?;

    Ok("All services stopped".to_string())
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
    for component in ["caddy", "php", "mysql", "mariadb", "phpmyadmin"] {
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
    let valid_components = ["caddy", "php", "mysql", "mariadb", "phpmyadmin"];
    if !valid_components.contains(&component.as_str()) {
        return Err(format!("Invalid component: {}", component));
    }

    // Stop the corresponding service if it maps to one
    let service_type = match component.as_str() {
        "caddy" => Some(ServiceType::Caddy),
        "php" => Some(ServiceType::PhpFpm),
        "mysql" | "mariadb" => Some(ServiceType::MySQL),
        _ => None,
    };

    if let Some(st) = service_type {
        let mut manager = state.process_manager.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        let _ = manager.stop(st);
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
