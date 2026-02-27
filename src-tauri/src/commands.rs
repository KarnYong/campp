//! Tauri IPC Commands
//!
//! This module contains all Tauri commands that are exposed to the frontend.

use crate::process::{ServiceMap, ServiceType};

// Global process manager instance (using lazy_static or once_cell in production)
// For now, we'll use a simple approach

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
