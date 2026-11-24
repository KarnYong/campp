// Modules
mod backup;
mod commands;
mod config;
mod database;
mod error;
mod process;
mod runtime;
mod sites;
mod terminal;

// Re-exports
pub use process::{ServiceInfo, ServiceMap, ServiceState, ServiceType};
pub use process::manager::ProcessManager;

use std::sync::{Arc, Mutex};
use tauri::{Manager, Emitter, AppHandle, menu::MenuEvent};
use tauri::tray::{TrayIconBuilder, TrayIconId};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::image::Image;

// Global state for the process manager
pub struct AppState {
    pub process_manager: Arc<Mutex<process::manager::ProcessManager>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            process_manager: Arc::new(Mutex::new(process::manager::ProcessManager::new())),
        }
    }
}

/// Headless entry point: download + extract the core runtime stack (Caddy +
/// PHP + default DB + phpMyAdmin) into the `runtime/` dir next to the exe, then
/// return a process exit code. Used by CI to build the offline "portable with
/// runtimes" release. Optional extras (PostgreSQL, Adminer, ngrok) stay
/// download-on-demand. ponytail: reuses RuntimeDownloader so the fragile
/// extract/marker/checksum logic lives in exactly one place instead of being
/// reimplemented in the CI YAML.
pub fn prepare_runtime(include: &[String]) -> i32 {
    if let Err(e) = headless_preflight() {
        eprintln!("prepare-runtime: {e}");
        return 1;
    }

    let downloader = match crate::runtime::downloader::RuntimeDownloader::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("prepare-runtime: cannot init downloader: {e}");
            return 1;
        }
    };

    // `--include <id>` pulls an optional component in (the Laravel e2e needs composer).
    let skip: Vec<&str> = ["postgresql", "adminer", "ngrok", "composer"]
        .into_iter()
        .filter(|id| !include.iter().any(|i| i == id))
        .collect();
    let cb: crate::runtime::downloader::ProgressCallback = Box::new(|_| {});
    match tauri::async_runtime::block_on(downloader.download_all_with_skip(cb, &skip)) {
        Ok(files) => {
            eprintln!("prepare-runtime: installed {} component archive(s)", files.len());
            0
        }
        Err(e) => {
            eprintln!("prepare-runtime failed: {e}");
            1
        }
    }
}

/// Shared setup for headless modes: a bad CAMPP_HOME fails up front, and
/// runtime-config.json is loaded. It ships next to the exe in a bundle, but CI and
/// the e2e run the raw target/ binary, so try the exe dir first, then the cwd.
fn headless_preflight() -> Result<(), String> {
    runtime::locator::base_dir()?;
    let dirs = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .into_iter()
        .chain(std::env::current_dir().ok());
    for dir in dirs {
        if dir.join("runtime-config.json").exists() {
            crate::runtime::packages::load_config_from_resource_dir(&dir);
            break;
        }
    }
    Ok(())
}

/// Headless `--serve` (spec 006): start PHP, the database and Caddy with the saved
/// settings, print `CAMPP_READY {json}`, and run until stdin closes or Ctrl+C/SIGTERM.
/// Used by the Laravel e2e with CAMPP_HOME so it never touches an installed CAMPP.
/// Closing stdin as the stop signal means a crashed test runner can't leave services
/// running. Release builds on Windows have no console (windows_subsystem), so this is
/// meant for debug builds and CI.
pub fn serve_headless() -> i32 {
    use std::io::Write;
    let fail = |msg: String| {
        println!("CAMPP_ERROR {}", msg.replace('\n', " "));
        let _ = std::io::stdout().flush();
        1
    };
    if let Err(e) = headless_preflight() {
        return fail(e);
    }

    let mut pm = ProcessManager::with_settings(config::AppSettings::load());
    if let Err(e) = pm.initialize() {
        return fail(e);
    }
    // Database before PHP-FPM/Caddy, so a page never hits a missing DB.
    for service in [ServiceType::MySQL, ServiceType::PhpFpm, ServiceType::Caddy] {
        if let Err(e) = pm.start(service) {
            let _ = pm.stop_all();
            return fail(format!("{service:?}: {e}"));
        }
    }
    let bin = match terminal::prepare_launchers() {
        Ok((bin, _)) => bin,
        Err(e) => {
            let _ = pm.stop_all();
            return fail(e);
        }
    };

    let s = pm.get_settings();
    let ready = serde_json::json!({
        "web": s.web_port, "php": s.php_port, "mysql": s.mysql_port,
        "bin": bin.to_string_lossy(),
    });
    println!("CAMPP_READY {ready}");
    let _ = std::io::stdout().flush();

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let on_eof = tx.clone();
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
        let _ = on_eof.send(());
    });
    std::thread::spawn(move || {
        tauri::async_runtime::block_on(async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
        });
        let _ = tx.send(());
    });
    let _ = rx.recv();

    let _ = pm.stop_all();
    println!("CAMPP_STOPPED");
    let _ = std::io::stdout().flush();
    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    // A CAMPP_HOME (test/dev) build must run beside the installed CAMPP, so it skips
    // the one-instance lock; normal installs keep it.
    if runtime::locator::home_override().is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Another instance was launched — show and focus the existing window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .manage(AppState::new())
        .on_menu_event(handle_menu_event)
        .setup(|app| {
            // Load runtime config from Tauri's resource directory
            if let Ok(resource_dir) = app.path().resource_dir() {
                crate::runtime::packages::load_config_from_resource_dir(&resource_dir);
            }

            #[cfg(debug_assertions)]
            {
                use tauri::menu::{Menu, MenuItem, Submenu};

                // Create debug menu items with IDs
                let open_download_folder = MenuItem::with_id(app, "open-download-folder", "View Download Folder (ZIP files)", true, None::<&str>)?;
                let open_runtime_folder = MenuItem::with_id(app, "open-runtime-folder", "Open Runtime Folder", true, None::<&str>)?;

                // Create submenu with items
                let debug_menu = Submenu::with_items(app, "Debug", true, &[&open_download_folder, &open_runtime_folder])?;
                let menu = Menu::with_items(app, &[&debug_menu])?;
                app.set_menu(menu)?;
            }

            // Setup system tray
            setup_system_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle window close event - minimize to tray instead of closing
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
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
            commands::check_system_dependencies,
            commands::download_runtime,
            commands::download_runtime_with_packages,
            commands::download_runtime_with_skip,
            commands::get_available_packages_cmd,
            commands::get_package_selection,
            commands::update_package_selection,
            commands::update_db_passwords,
            commands::get_selected_package_ids,
            commands::reload_runtime_config,
            commands::get_installed_versions,
            commands::check_existing_components,
            commands::get_runtime_dir,
            commands::get_download_dir,
            commands::get_logs_dir,
            commands::get_install_dir,
            commands::open_folder,
            commands::open_manual,
            commands::reset_installation,
            commands::set_theme,
            commands::get_caddy_version,
            commands::list_backups,
            commands::create_backup,
            commands::restore_backup,
            commands::delete_backup,
            commands::delete_all_data,
            commands::open_backups_folder,
            commands::cleanup_all_services,
            commands::uninstall_component,
            commands::open_terminal,
            commands::list_sites,
            commands::get_debug_info,
            commands::get_ngrok_public_url,
            commands::add_windows_defender_exclusion,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        // Debug menu items
        "open-download-folder" => {
            tauri::async_runtime::spawn(async move {
                if let Ok(download_dir) = commands::get_download_dir().await {
                    let _ = commands::open_folder(download_dir).await;
                }
            });
        }
        "open-runtime-folder" => {
            tauri::async_runtime::spawn(async move {
                if let Ok(runtime_dir) = commands::get_runtime_dir().await {
                    let _ = commands::open_folder(runtime_dir).await;
                }
            });
        }
        // Tray menu items
        "tray-show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "tray-hide" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        "tray-quit" => {
            // Cleanup services before quitting
            if let Some(state) = app.try_state::<AppState>() {
                let pm = state.process_manager.clone();
                let _ = pm.lock().unwrap().stop_all();
            }
            std::process::exit(0);
        }
        _ => {}
    }
}

fn setup_system_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Embed the icon directly in the binary to ensure it's always available
    #[cfg(target_os = "windows")]
    let icon_bytes = include_bytes!("../icons/icon.ico");
    #[cfg(not(target_os = "windows"))]
    let icon_bytes = include_bytes!("../icons/32x32.png");

    // Load and decode the embedded image
    let img = image::load_from_memory(icon_bytes)?;
    let rgba = img.to_rgba8();
    let dimensions = rgba.dimensions();
    let raw_bytes = rgba.as_raw().to_vec();

    // Create Tauri Image from raw RGBA bytes
    let tray_icon = Image::new_owned(raw_bytes, dimensions.0, dimensions.1);

    // Create tray menu items
    let show_item = MenuItem::with_id(app, "tray-show", "Show CAMPP", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "tray-hide", "Hide to Tray", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "tray-quit", "Quit CAMPP", true, None::<&str>)?;

    // Create tray menu
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;

    // Build and set tray icon
    let tray_id = TrayIconId::new("main-tray");
    TrayIconBuilder::with_id(tray_id)
        .menu(&menu)
        .tooltip("CAMPP - Local Web Stack")
        .icon(tray_icon)
        .build(app)?;

    Ok(())
}
