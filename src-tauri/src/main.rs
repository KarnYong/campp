// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Headless runtime pre-population for building the offline portable release.
    if std::env::args().any(|a| a == "--prepare-runtime") {
        std::process::exit(tauri_app_lib::prepare_runtime());
    }
    tauri_app_lib::run()
}
