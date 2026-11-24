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

    let args: Vec<String> = std::env::args().collect();
    // Headless runtime pre-population for building the offline portable release.
    // `--include <id>` (repeatable) also installs an optional component.
    if args.iter().any(|a| a == "--prepare-runtime") {
        let include: Vec<String> = args
            .windows(2)
            .filter(|w| w[0] == "--include")
            .map(|w| w[1].clone())
            .collect();
        std::process::exit(tauri_app_lib::prepare_runtime(&include));
    }
    // Headless services for the isolated Laravel e2e (run with CAMPP_HOME set).
    if args.iter().any(|a| a == "--serve") {
        std::process::exit(tauri_app_lib::serve_headless());
    }
    tauri_app_lib::run()
}
