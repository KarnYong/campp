use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::process::manager::configure_no_window;

/// Initialize PostgreSQL data directory using initdb
pub fn initialize_postgresql(pgsql_dir: &Path, data_dir: &Path, logs_dir: &Path) -> Result<(), String> {
    // Check if already initialized
    let pg_version = data_dir.join("PG_VERSION");
    if pg_version.exists() {
        tracing::info!("PostgreSQL data directory already initialized");
        return Ok(());
    }

    // Create data directory
    fs::create_dir_all(data_dir)
        .map_err(|e| format!("Failed to create PostgreSQL data directory: {}", e))?;

    // Locate initdb binary
    #[cfg(target_os = "windows")]
    let initdb = pgsql_dir.join("bin").join("initdb.exe");
    #[cfg(not(target_os = "windows"))]
    let initdb = pgsql_dir.join("bin").join("initdb");

    if !initdb.exists() {
        return Err(format!(
            "PostgreSQL initdb not found at: {}. Please ensure PostgreSQL runtime was downloaded correctly.",
            initdb.display()
        ));
    }

    let data_dir_str = data_dir.to_string_lossy().to_string();

    tracing::info!("Initializing PostgreSQL data directory at: {}", data_dir_str);

    let init_log_path = logs_dir.join("pgsql_init.log");
    let init_log_file = fs::File::create(&init_log_path)
        .map_err(|e| format!("Failed to create PostgreSQL init log file: {}", e))?;

    let mut cmd = configure_no_window(Command::new(&initdb));
    cmd.arg("-D").arg(&data_dir_str)
        .arg("--auth=trust")
        .arg("--encoding=UTF8")
        .arg("-U").arg("root")
        .stdout(Stdio::from(init_log_file.try_clone().map_err(|e| format!("Failed to clone stdout: {}", e))?))
        .stderr(Stdio::from(init_log_file));

    // On Unix, set library path so initdb can find shared libraries
    #[cfg(unix)]
    {
        let lib_dir = pgsql_dir.join("lib");
        let lib_path = lib_dir.to_string_lossy().to_string();
        if let Ok(existing) = std::env::var("LD_LIBRARY_PATH") {
            cmd.env("LD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
        } else {
            cmd.env("LD_LIBRARY_PATH", &lib_path);
        }
        if let Ok(existing) = std::env::var("DYLD_LIBRARY_PATH") {
            cmd.env("DYLD_LIBRARY_PATH", format!("{}:{}", lib_path, existing));
        } else {
            cmd.env("DYLD_LIBRARY_PATH", &lib_path);
        }
    }

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to start PostgreSQL initialization: {}", e))?;

    let timeout = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();

    let mut output = String::new();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = fs::read_to_string(&init_log_path).map(|s| output = s);
                break status.success();
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    tracing::warn!("PostgreSQL initialization timeout, killing process");
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::read_to_string(&init_log_path).map(|s| output = s);
                    break false;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(_) => {
                let _ = fs::read_to_string(&init_log_path).map(|s| output = s);
                break false;
            }
        }
    };

    if !success {
        tracing::error!("PostgreSQL initialization failed. Output:\n{}", output);
        return Err(format!(
            "PostgreSQL initialization failed. Check the log file at: {:?}",
            init_log_path
        ));
    }

    // Verify PG_VERSION was created
    if !pg_version.exists() {
        return Err(format!(
            "PostgreSQL initialization failed - PG_VERSION not created at {:?}. Check the log file at: {:?}",
            pg_version, init_log_path
        ));
    }

    tracing::info!("PostgreSQL initialization completed successfully");
    Ok(())
}
