//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceType};
use crate::runtime::downloader::{DownloadProgress, RuntimeDownloader};
use std::fs;
use std::sync::Mutex;
use tauri::Emitter;

// Global state for download progress
static DOWNLOAD_PROGRESS: Mutex<Option<DownloadProgress>> = Mutex::new(None);

/// Start a service
#[tauri::command]
pub async fn start_service(_service: ServiceType) -> Result<ServiceMap, String> {
    // TODO: Implement actual process control in Phase 3
    Ok(get_mock_statuses())
}

/// Stop a service
#[tauri::command]
pub async fn stop_service(_service: ServiceType) -> Result<ServiceMap, String> {
    // TODO: Implement actual process control in Phase 3
    Ok(get_mock_statuses())
}

/// Restart a service
#[tauri::command]
pub async fn restart_service(_service: ServiceType) -> Result<ServiceMap, String> {
    // TODO: Implement actual process control in Phase 3
    Ok(get_mock_statuses())
}

/// Get the status of all services
#[tauri::command]
pub async fn get_all_statuses() -> Result<ServiceMap, String> {
    Ok(get_mock_statuses())
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

// Helper function for mock data
fn get_mock_statuses() -> ServiceMap {
    use crate::process::{ServiceInfo, ServiceState};
    use std::collections::HashMap;

    let mut statuses = HashMap::new();

    for service_type in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MariaDB] {
        statuses.insert(
            service_type,
            ServiceInfo {
                service_type,
                state: ServiceState::Stopped,
                port: service_type.default_port(),
                error_message: None,
            },
        );
    }

    statuses
}
