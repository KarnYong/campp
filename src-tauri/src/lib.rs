// Modules
mod commands;
mod config;
mod database;
mod process;
mod runtime;

// Re-exports
pub use process::{ServiceInfo, ServiceMap, ServiceState, ServiceType};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Service management commands
            commands::start_service,
            commands::stop_service,
            commands::restart_service,
            commands::get_all_statuses,
            // Settings commands
            commands::get_settings,
            commands::save_settings,
            // Runtime download commands
            commands::check_runtime_installed,
            commands::download_runtime,
            commands::get_runtime_dir,
            commands::reset_installation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
