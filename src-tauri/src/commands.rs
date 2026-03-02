//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceType};
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

    // Initialize if needed
    let _ = manager.initialize();

    // Start the service
    manager.start(service)?;

    // Update health and return statuses
    manager.update_health();
    Ok(manager.get_all_statuses())
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

    // Initialize if needed
    let _ = manager.initialize();

    // Restart the service
    manager.restart(service)?;

    // Update health and return statuses
    manager.update_health();
    Ok(manager.get_all_statuses())
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
    Ok(crate::config::AppSettings::default())
}

/// Save app settings
#[tauri::command]
pub async fn save_settings(_settings: crate::config::AppSettings) -> Result<(), String> {
    // TODO: Implement settings persistence in Phase 4
    Ok(())
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
