use crate::runtime::locator::RuntimePaths;
use crate::runtime::packages::{active_db_engine, DbEngine};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::process::manager::configure_no_window;

/// Has this engine already initialized the data dir? MySQL 8 leaves `.sdi` files in
/// `mysql/`; MariaDB leaves Aria/system tables there (no `.sdi`). Windows'
/// mariadb-install-db.exe refuses a non-empty datadir, so MariaDB must not re-init.
fn is_initialized(engine: DbEngine, mysql_dir: &Path) -> bool {
    let entries: Vec<_> = mysql_dir.read_dir()
        .and_then(|e| e.collect::<Result<_, _>>())
        .unwrap_or_default();
    match engine {
        DbEngine::MySql => entries.iter().any(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sdi"))
                .unwrap_or(false)
        }),
        DbEngine::MariaDb => !entries.is_empty(),
    }
}

pub fn initialize_mysql(paths: &RuntimePaths) -> Result<(), String> {
    let engine = active_db_engine();
    let name = engine.display();
    let mysql_dir = paths.mysql_data_dir.join("mysql");
    if is_initialized(engine, &mysql_dir) {
        tracing::info!("{} data directory already initialized", name);
        return Ok(());
    }

    fs::create_dir_all(&paths.mysql_data_dir)
        .map_err(|e| format!("Failed to create {} data directory: {}", name, e))?;

    let data_dir_str = paths.mysql_data_dir.to_string_lossy().replace('\\', "/");

    if engine == DbEngine::MariaDb {
        tracing::info!("MariaDB: Initializing data directory using mariadb-install-db");

        let mariadbd_dir = paths.mysql.parent()
            .ok_or("Failed to get MariaDB binary directory")?;

        // Linux bintar ships the script in scripts/; the Windows zip ships
        // mariadb-install-db.exe next to mariadbd.exe in bin/.
        #[cfg(target_os = "windows")]
        let (install_db_script, install_db_fallback) =
            (mariadbd_dir.join("mariadb-install-db.exe"), mariadbd_dir.join("mysql_install_db.exe"));
        #[cfg(not(target_os = "windows"))]
        let (install_db_script, install_db_fallback) = {
            let scripts_dir = mariadbd_dir.parent()
                .ok_or("Failed to get MariaDB base directory")?
                .join("scripts");
            (scripts_dir.join("mariadb-install-db"), scripts_dir.join("mysql_install_db"))
        };

        // Resolve the actual init script. (Previously the fallback was checked
        // for existence but never selected, so a MariaDB build shipping only
        // mysql_install_db would still try the missing mariadb-install-db.)
        let init_script = if install_db_script.exists() {
            install_db_script
        } else if install_db_fallback.exists() {
            install_db_fallback
        } else {
            return Err(format!(
                "MariaDB installation script not found. Tried:\n  - {}\n  - {}\n\
                Please ensure the MariaDB runtime was downloaded correctly.",
                install_db_script.display(),
                install_db_fallback.display()
            ));
        };

        // Tar entries extracted via io::copy lose their Unix mode, so the init
        // script and the server binary it invokes land at 0o644 and can't run.
        // Ensure both are executable right before spawning. No-op on Windows.
        crate::runtime::locator::ensure_executable(&init_script);
        crate::runtime::locator::ensure_executable(&paths.mysql);

        let init_log_path = paths.logs_dir.join("mysql_init.log");
        let init_log_file = fs::File::create(&init_log_path)
            .map_err(|e| format!("Failed to create init log file: {}", e))?;

        let mut cmd = configure_no_window(Command::new(&init_script));
        cmd.arg(format!("--datadir={}", data_dir_str));
        // Unix script options; the Windows .exe takes only --datadir here.
        #[cfg(not(target_os = "windows"))]
        cmd.arg(format!("--basedir={}", mariadbd_dir.parent().unwrap().display()))
            .arg("--user=");
        cmd.stdout(Stdio::from(init_log_file.try_clone().unwrap()))
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

    else {
        tracing::info!("MySQL 8.x: Initializing data directory at: {}", data_dir_str);

        let mysqld = &paths.mysql;

        // Ensure mysqld is executable before spawning (tar extraction drops the
        // exec bit). No-op on Windows.
        crate::runtime::locator::ensure_executable(mysqld);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialized_check_is_engine_specific() {
        let d = tempfile::TempDir::new().unwrap();
        let sys = d.path().join("mysql");
        assert!(!is_initialized(DbEngine::MariaDb, &sys)); // missing dir
        fs::create_dir_all(&sys).unwrap();
        assert!(!is_initialized(DbEngine::MariaDb, &sys)); // empty dir
        fs::write(sys.join("global_priv.MAD"), "").unwrap();
        assert!(is_initialized(DbEngine::MariaDb, &sys));
        assert!(!is_initialized(DbEngine::MySql, &sys)); // MySQL needs .sdi
        fs::write(sys.join("mysql.sdi"), "").unwrap();
        assert!(is_initialized(DbEngine::MySql, &sys));
    }
}
