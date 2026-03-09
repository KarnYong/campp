//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceState, ServiceType};
use crate::runtime::downloader::{DownloadProgress, RuntimeDownloader};
use crate::AppState;
use std::fs;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::State;

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
    let downloader = RuntimeDownloader::new();
    Ok(downloader.is_installed())
}

/// Reset installation (for testing/debug - deletes runtime directory)
#[tauri::command]
pub async fn reset_installation() -> Result<String, String> {
    let downloader = RuntimeDownloader::new();
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
    let downloader = RuntimeDownloader::new();
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

/// Open a folder in the system's file explorer
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    // Ensure folder exists before opening
    let path_obj = std::path::Path::new(&path);
    if !path_obj.exists() {
        fs::create_dir_all(path_obj)
            .map_err(|e| format!("Failed to create folder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

/// Open the user manual in the browser
#[tauri::command]
pub async fn open_manual(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let manual_path = resource_dir.join("MANUAL.html");

    // Open in default browser
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", &manual_path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open manual: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&manual_path)
            .spawn()
            .map_err(|e| format!("Failed to open manual: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&manual_path)
            .spawn()
            .map_err(|e| format!("Failed to open manual: {}", e))?;
    }

    Ok(())
}

/// Download and install runtime binaries
#[tauri::command]
pub async fn download_runtime(app: tauri::AppHandle) -> Result<String, String> {
    let downloader = RuntimeDownloader::new();
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
