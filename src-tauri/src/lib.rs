// Modules
mod commands;
mod config;
mod database;
mod process;
mod runtime;

// Re-exports
pub use process::{ServiceInfo, ServiceMap, ServiceState, ServiceType};
pub use process::manager::ProcessManager;

use std::sync::Mutex;

// Global state for the process manager
pub struct AppState {
    pub process_manager: Mutex<process::manager::ProcessManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            process_manager: Mutex::new(process::manager::ProcessManager::new()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                use tauri::menu::{Menu, MenuItem, Submenu};

                // Create menu items with IDs
                let open_download_folder = MenuItem::with_id(app, "open-download-folder", "View Download Folder (ZIP files)", true, None::<&str>)?;
                let open_runtime_folder = MenuItem::with_id(app, "open-runtime-folder", "Open Runtime Folder", true, None::<&str>)?;
                let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
                let reset_installation = MenuItem::with_id(app, "reset-installation", "Reset Installation", true, None::<&str>)?;
                let show_wizard = MenuItem::with_id(app, "show-wizard", "Show First-Run Wizard", true, None::<&str>)?;

                // Create submenu with items
                let debug_menu = Submenu::with_items(app, "Debug", true, &[&open_download_folder, &open_runtime_folder, &separator, &reset_installation, &show_wizard])?;
                let menu = Menu::with_items(app, &[&debug_menu])?;
                app.set_menu(menu)?;

                // Handle menu events
                app.on_menu_event(|app, event| {
                    use tauri::Emitter;
                    match event.id.as_ref() {
                        "open-download-folder" => {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Ok(download_dir) = commands::get_download_dir().await {
                                    let _ = commands::open_folder(download_dir).await;
                                }
                            });
                        }
                        "open-runtime-folder" => {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Ok(runtime_dir) = commands::get_runtime_dir().await {
                                    let _ = tauri_plugin_opener::reveal_item_in_dir(runtime_dir);
                                }
                            });
                        }
                        "reset-installation" => {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Ok(_) = commands::reset_installation().await {
                                    let _ = app.emit("show-wizard", ());
                                }
                            });
                        }
                        "show-wizard" => {
                            let _ = app.emit("show-wizard", ());
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Service management commands
            commands::start_service,
            commands::stop_service,
            commands::restart_service,
            commands::get_all_statuses,
            // Settings commands
            commands::get_settings,
            commands::save_settings,
            commands::validate_settings,
            commands::check_ports,
            // Runtime download commands
            commands::check_runtime_installed,
            commands::download_runtime,
            commands::get_runtime_dir,
            commands::get_download_dir,
            commands::open_folder,
            commands::reset_installation,
            commands::cleanup_all_services,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

