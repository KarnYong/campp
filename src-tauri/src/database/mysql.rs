use crate::runtime::locator::RuntimePaths;
use std::fs;
use std::process::{Command, Stdio};

use crate::process::manager::configure_no_window;

pub fn initialize_mysql(paths: &RuntimePaths) -> Result<(), String> {
    let mysql_dir = paths.mysql_data_dir.join("mysql");
    if mysql_dir.exists() {
        let entries: Vec<_> = mysql_dir.read_dir()
            .and_then(|e| e.collect::<Result<_, _>>())
            .unwrap_or_default();

        let has_sdi_files = entries.iter().any(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sdi"))
                .unwrap_or(false)
        });

        if has_sdi_files {
            #[cfg(target_os = "linux")]
            tracing::info!("MariaDB data directory already initialized");
            #[cfg(not(target_os = "linux"))]
            tracing::info!("MySQL data directory already initialized");
            return Ok(());
        }
    }

    #[cfg(target_os = "linux")]
    fs::create_dir_all(&paths.mysql_data_dir)
        .map_err(|e| format!("Failed to create MariaDB data directory: {}", e))?;
    #[cfg(not(target_os = "linux"))]
    fs::create_dir_all(&paths.mysql_data_dir)
        .map_err(|e| format!("Failed to create MySQL data directory: {}", e))?;

    let data_dir_str = paths.mysql_data_dir.to_string_lossy().replace('\\', "/");

    #[cfg(target_os = "linux")]
    {
        tracing::info!("MariaDB 12.x: Initializing data directory using mariadb-install-db");

        let mariadbd_dir = paths.mysql.parent()
            .ok_or("Failed to get MariaDB binary directory")?;

        let install_db_script = mariadbd_dir.parent()
            .ok_or("Failed to get MariaDB base directory")?
            .join("scripts")
            .join("mariadb-install-db");

        if !install_db_script.exists() {
            let install_db_script_fallback = mariadbd_dir.parent()
                .ok_or("Failed to get MariaDB base directory")?
                .join("scripts")
                .join("mysql_install_db");

            if !install_db_script_fallback.exists() {
                return Err(format!(
                    "MariaDB installation script not found. Tried:\n  - {}\n  - {}\n\
                    Please ensure the MariaDB runtime was downloaded correctly.",
                    install_db_script.display(),
                    install_db_script_fallback.display()
                ));
            }
        }

        let init_log_path = paths.logs_dir.join("mysql_init.log");
        let init_log_file = fs::File::create(&init_log_path)
            .map_err(|e| format!("Failed to create init log file: {}", e))?;

        let mut cmd = configure_no_window(Command::new(&install_db_script));
        cmd.arg(format!("--datadir={}", data_dir_str))
            .arg(format!("--basedir={}", mariadbd_dir.parent().unwrap().display()))
            .arg("--user=")
            .stdout(Stdio::from(init_log_file.try_clone().unwrap()))
            .stderr(Stdio::from(init_log_file));

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to start MariaDB initialization: {}", e))?;

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
                        tracing::warn!("MariaDB initialization timeout, killing process");
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
            tracing::error!("MariaDB initialization failed. Output:\n{}", output);
            return Err(format!(
                "MariaDB initialization failed. Check the log file at: {:?}",
                init_log_path
            ));
        }

        tracing::info!("MariaDB initialization completed successfully");

        if !mysql_dir.exists() {
            return Err(format!(
                "MariaDB initialization failed - mysql directory not created at {:?}. \
                 Check the log file at: {:?}",
                mysql_dir, init_log_path
            ));
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        tracing::info!("MySQL 8.x: Initializing data directory at: {}", data_dir_str);

        let mysqld = &paths.mysql;

        let init_log_path = paths.logs_dir.join("mysql_init.log");
        let init_log_file = fs::File::create(&init_log_path)
            .map_err(|e| format!("Failed to create init log file: {}", e))?;

        let mut child = configure_no_window(Command::new(mysqld))
            .arg("--initialize-insecure")
            .arg("--datadir")
            .arg(&data_dir_str)
            .arg("--console")
            .stdout(Stdio::from(init_log_file.try_clone().unwrap()))
            .stderr(Stdio::from(init_log_file))
            .spawn()
            .map_err(|e| format!("Failed to start MySQL initialization: {}", e))?;

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
                        tracing::warn!("MySQL initialization timeout, killing process");
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
            tracing::error!("MySQL initialization failed. Output:\n{}", output);
            return Err(format!(
                "MySQL initialization failed. Check the log file at: {:?}",
                init_log_path
            ));
        }

        tracing::info!("MySQL initialization completed successfully");

        if !mysql_dir.exists() {
            return Err(format!(
                "MySQL initialization failed - mysql directory not created at {:?}. \
                 Check the log file at: {:?}",
                mysql_dir, init_log_path
            ));
        }
    }

    Ok(())
}

pub fn create_database(name: &str) -> Result<(), String> {
    let _ = name;
    // TODO: Implement database creation
    Ok(())
}

pub fn drop_database(name: &str) -> Result<(), String> {
    let _ = name;
    // TODO: Implement database deletion
    Ok(())
}

pub fn list_databases() -> Result<Vec<String>, String> {
    // TODO: Implement database listing
    Ok(vec![])
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

pub fn get_connection_info() -> ConnectionInfo {
    ConnectionInfo {
        host: "127.0.0.1".to_string(),
        port: 3307,
        user: "root".to_string(),
        password: String::new(),
    }
}
