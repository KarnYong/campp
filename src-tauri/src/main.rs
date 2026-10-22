// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux: draw the WebKitGTK window in software. GPU rendering gives a blank window
    // or a crash on VMs, WSL, some NVIDIA/Wayland setups and installs without Mesa;
    // CAMPP's simple UI doesn't need it. Set before any thread or window exists.
    // A user's own value wins (set it to 0 to get GPU rendering back).
    #[cfg(target_os = "linux")]
    for var in ["WEBKIT_DISABLE_DMABUF_RENDERER", "WEBKIT_DISABLE_COMPOSITING_MODE"] {
        if std::env::var_os(var).is_none() {
            std::env::set_var(var, "1");
        }
    }

    // Headless runtime pre-population for building the offline portable release.
    if std::env::args().any(|a| a == "--prepare-runtime") {
        std::process::exit(tauri_app_lib::prepare_runtime());
    }
    tauri_app_lib::run()
}
