use std::path::{Path, PathBuf};
use std::fs;

/// Runtime binary paths
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub caddy: PathBuf,
    pub php_cgi: PathBuf,
    pub php_ini: PathBuf,
    pub mariadb: PathBuf,
    pub phpmyadmin: PathBuf,
    /// Data directory for MariaDB
    pub mysql_data_dir: PathBuf,
    /// Logs directory
    pub logs_dir: PathBuf,
    /// Config directory
    pub config_dir: PathBuf,
    /// Projects directory
    pub projects_dir: PathBuf,
}

/// Application data directory structure
#[derive(Debug, Clone)]
pub struct AppDataPaths {
    /// Base data directory (e.g., ~/.campp)
    pub base_dir: PathBuf,
    /// Runtime binaries directory
    pub runtime_dir: PathBuf,
    /// Configuration files directory
    pub config_dir: PathBuf,
    /// MariaDB data directory
    pub mysql_data_dir: PathBuf,
    /// Logs directory
    pub logs_dir: PathBuf,
    /// Projects directory
    pub projects_dir: PathBuf,
}

impl AppDataPaths {
    /// Create all necessary directories
    pub fn ensure_directories(&self) -> Result<(), String> {
        for dir in [&self.config_dir, &self.mysql_data_dir, &self.logs_dir, &self.projects_dir] {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| format!("Failed to create directory {}: {}", dir.display(), e))?;
            }
        }
        Ok(())
    }
}

/// Get the application data directory paths
pub fn get_app_data_paths() -> Result<AppDataPaths, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Cannot find data directory".to_string())?
        .join("campp");

    Ok(AppDataPaths {
        base_dir: data_dir.clone(),
        runtime_dir: data_dir.join("runtime"),
        config_dir: data_dir.join("config"),
        mysql_data_dir: data_dir.join("mysql").join("data"),
        logs_dir: data_dir.join("logs"),
        projects_dir: data_dir.join("projects"),
    })
}

/// Locate runtime binaries after download
pub fn locate_runtime_binaries() -> Result<RuntimePaths, String> {
    let app_paths = get_app_data_paths()?;
    let runtime_dir = &app_paths.runtime_dir;

    // Ensure runtime directory exists
    if !runtime_dir.exists() {
        return Err(format!(
            "Runtime directory not found. Please download runtime binaries first. Expected: {}",
            runtime_dir.display()
        ));
    }

    // Detect phpMyAdmin directory (may be versioned like phpMyAdmin-5.2.2-all-languages)
    let phpmyadmin_path = detect_phpmyadmin_directory(runtime_dir)?;

    Ok(RuntimePaths {
        caddy: detect_caddy_binary(runtime_dir)?,
        php_cgi: detect_php_binary(runtime_dir)?,
        php_ini: detect_php_ini(runtime_dir)?,
        mariadb: detect_mariadb_binary(runtime_dir)?,
        phpmyadmin: phpmyadmin_path,
        mysql_data_dir: app_paths.mysql_data_dir.clone(),
        logs_dir: app_paths.logs_dir.clone(),
        config_dir: app_paths.config_dir.clone(),
        projects_dir: app_paths.projects_dir.clone(),
    })
}

/// Detect Caddy binary based on platform
fn detect_caddy_binary(runtime_dir: &Path) -> Result<PathBuf, String> {
    // Caddy extraction creates different structures based on platform

    #[cfg(target_os = "windows")]
    {
        // Windows: caddy.exe might be at runtime/caddy.exe or runtime/caddy/caddy.exe
        let paths_to_check = vec![
            runtime_dir.join("caddy.exe"),
            runtime_dir.join("caddy").join("caddy.exe"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: caddy binary might be at runtime/caddy or runtime/caddy/caddy
        let paths_to_check = vec![
            runtime_dir.join("caddy"),
            runtime_dir.join("caddy").join("caddy"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "Caddy binary not found in {}. Please ensure runtime binaries are downloaded.",
        runtime_dir.display()
    ))
}

/// Detect PHP CGI binary based on platform
fn detect_php_binary(runtime_dir: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        // Windows PHP distribution structure:
        // runtime/php/php-cgi.exe or runtime/php-cgi.exe (CGI binary)
        // runtime/php/php.exe or runtime/php.exe (CLI binary)
        let paths_to_check = vec![
            runtime_dir.join("php-cgi.exe"),         // Direct in runtime dir
            runtime_dir.join("php").join("php-cgi.exe"),
            runtime_dir.join("php.exe"),              // Fallback to CLI
            runtime_dir.join("php").join("php.exe"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: PHP might be in runtime/php/bin/php or similar
        let paths_to_check = vec![
            runtime_dir.join("php").join("bin").join("php-cgi"),
            runtime_dir.join("php-cgi"),              // Direct in runtime dir
            runtime_dir.join("usr").join("local").join("bin").join("php"),
            runtime_dir.join("php").join("bin").join("php"),
            runtime_dir.join("php"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: PHP structure varies by distribution
        let paths_to_check = vec![
            runtime_dir.join("php").join("bin").join("php-cgi"),
            runtime_dir.join("php-cgi"),              // Direct in runtime dir
            runtime_dir.join("php").join("bin").join("php"),
            runtime_dir.join("usr").join("bin").join("php"),
            runtime_dir.join("php"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "PHP binary not found in {}. Please ensure runtime binaries are downloaded.",
        runtime_dir.display()
    ))
}

/// Detect PHP configuration file
fn detect_php_ini(runtime_dir: &Path) -> Result<PathBuf, String> {
    // PHP ini will be generated in config directory
    let app_paths = get_app_data_paths()?;
    let php_ini_path = app_paths.config_dir.join("php.ini");

    Ok(php_ini_path)
}

/// Detect MariaDB server binary based on platform
fn detect_mariadb_binary(runtime_dir: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        // Windows MariaDB structure:
        // - runtime/mariadb/bin/mysqld.exe
        // - runtime/mariadb-VERSION/bin/mysqld.exe (versioned directory)
        // Look for any directory starting with "mariadb"
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("mariadb") && entry.path().is_dir() {
                        let mysqld_path = entry.path().join("bin").join("mysqld.exe");
                        if mysqld_path.exists() {
                            return Ok(mysqld_path);
                        }
                    }
                }
            }
        }

        // Fallback paths
        let paths_to_check = vec![
            runtime_dir.join("mariadb").join("bin").join("mysqld.exe"),
            runtime_dir.join("bin").join("mysqld.exe"),
            runtime_dir.join("mysqld.exe"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS MariaDB structure
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("mariadb") && entry.path().is_dir() {
                        let mysqld_path = entry.path().join("bin").join("mysqld");
                        if mysqld_path.exists() {
                            return Ok(mysqld_path);
                        }
                    }
                }
            }
        }

        let paths_to_check = vec![
            runtime_dir.join("mariadb").join("bin").join("mysqld"),
            runtime_dir.join("mysql").join("bin").join("mysqld"),
            runtime_dir.join("bin").join("mysqld"),
            runtime_dir.join("mysqld"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux MariaDB structure
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("mariadb") && entry.path().is_dir() {
                        let mysqld_path = entry.path().join("bin").join("mysqld");
                        if mysqld_path.exists() {
                            return Ok(mysqld_path);
                        }
                    }
                }
            }
        }

        let paths_to_check = vec![
            runtime_dir.join("mariadb").join("bin").join("mysqld"),
            runtime_dir.join("mysql").join("bin").join("mysqld"),
            runtime_dir.join("bin").join("mysqld"),
            runtime_dir.join("mysqld"),
        ];

        for path in paths_to_check {
            if path.exists() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "MariaDB binary not found in {}. Please ensure runtime binaries are downloaded.",
        runtime_dir.display()
    ))
}

/// Detect phpMyAdmin directory (may be versioned like phpMyAdmin-5.2.2-all-languages)
fn detect_phpmyadmin_directory(runtime_dir: &Path) -> Result<PathBuf, String> {
    // First try the standard path
    let standard_path = runtime_dir.join("phpmyadmin");
    if standard_path.exists() {
        return Ok(standard_path);
    }

    // Look for any directory starting with "phpMyAdmin" or "phpmyadmin"
    if let Ok(entries) = fs::read_dir(runtime_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                let name_lower = name.to_lowercase();
                if name_lower.starts_with("phpmyadmin") && entry.path().is_dir() {
                    return Ok(entry.path());
                }
            }
        }
    }

    Err(format!(
        "phpMyAdmin directory not found in {}. Please ensure runtime binaries are downloaded.",
        runtime_dir.display()
    ))
}

/// Check if a binary is valid (exists and is executable)
pub fn is_valid_binary(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(path) {
            Ok(metadata) => {
                let permissions = metadata.permissions();
                let mode = permissions.mode();
                // Check if owner has execute permission
                mode & 0o100 != 0
            }
            Err(_) => false,
        }
    }

    #[cfg(windows)]
    {
        // On Windows, just check if the file exists
        true
    }
}

/// Verify all runtime binaries are present and valid
pub fn verify_runtime_binaries() -> Result<RuntimePaths, String> {
    let paths = locate_runtime_binaries()?;

    if !is_valid_binary(&paths.caddy) {
        return Err(format!("Caddy binary not found or not executable: {}", paths.caddy.display()));
    }

    if !is_valid_binary(&paths.php_cgi) {
        return Err(format!("PHP binary not found or not executable: {}", paths.php_cgi.display()));
    }

    if !is_valid_binary(&paths.mariadb) {
        return Err(format!("MariaDB binary not found or not executable: {}", paths.mariadb.display()));
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_mock_binary(path: &Path) -> Result<(), String> {
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create binary: {}", e))?;
        writeln!(file, "#! /bin/sh\n# mock binary").unwrap();
        Ok(())
    }

    #[cfg(unix)]
    fn set_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn test_app_data_paths_creation() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("campp");
        fs::create_dir_all(&base_dir).unwrap();

        let paths = AppDataPaths {
            base_dir: base_dir.clone(),
            runtime_dir: base_dir.join("runtime"),
            config_dir: base_dir.join("config"),
            mysql_data_dir: base_dir.join("mysql").join("data"),
            logs_dir: base_dir.join("logs"),
            projects_dir: base_dir.join("projects"),
        };

        let result = paths.ensure_directories();
        assert!(result.is_ok());

        assert!(paths.config_dir.exists());
        assert!(paths.mysql_data_dir.exists());
        assert!(paths.logs_dir.exists());
        assert!(paths.projects_dir.exists());
    }

    #[test]
    fn test_is_valid_binary_with_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.exe");

        assert!(!is_valid_binary(&nonexistent));
    }

    #[test]
    fn test_is_valid_binary_with_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let binary_path = temp_dir.path().join("test_binary");

        create_mock_binary(&binary_path).unwrap();

        #[cfg(unix)]
        set_executable(&binary_path);

        let is_valid = is_valid_binary(&binary_path);
        #[cfg(windows)]
        assert!(is_valid);

        #[cfg(unix)]
        assert!(is_valid);
    }

    #[test]
    fn test_runtime_paths_structure() {
        let temp_dir = TempDir::new().unwrap();

        let paths = RuntimePaths {
            caddy: temp_dir.path().join("caddy.exe"),
            php_cgi: temp_dir.path().join("php").join("php.exe"),
            php_ini: temp_dir.path().join("config").join("php.ini"),
            mariadb: temp_dir.path().join("mariadb").join("bin").join("mysqld.exe"),
            phpmyadmin: temp_dir.path().join("phpmyadmin"),
            mysql_data_dir: temp_dir.path().join("mysql").join("data"),
            logs_dir: temp_dir.path().join("logs"),
            config_dir: temp_dir.path().join("config"),
            projects_dir: temp_dir.path().join("projects"),
        };

        assert!(paths.caddy.ends_with("caddy.exe"));
        assert!(paths.php_cgi.ends_with("php.exe"));
        assert!(paths.php_ini.ends_with("php.ini"));
        assert!(paths.mariadb.ends_with("mysqld.exe"));
        assert!(paths.phpmyadmin.ends_with("phpmyadmin"));
    }

    #[test]
    fn test_runtime_paths_clone() {
        let temp_dir = TempDir::new().unwrap();

        let paths1 = RuntimePaths {
            caddy: temp_dir.path().join("caddy.exe"),
            php_cgi: temp_dir.path().join("php").join("php.exe"),
            php_ini: temp_dir.path().join("config").join("php.ini"),
            mariadb: temp_dir.path().join("mariadb").join("bin").join("mysqld.exe"),
            phpmyadmin: temp_dir.path().join("phpmyadmin"),
            mysql_data_dir: temp_dir.path().join("mysql").join("data"),
            logs_dir: temp_dir.path().join("logs"),
            config_dir: temp_dir.path().join("config"),
            projects_dir: temp_dir.path().join("projects"),
        };

        let paths2 = paths1.clone();

        assert_eq!(paths1.caddy, paths2.caddy);
        assert_eq!(paths1.php_cgi, paths2.php_cgi);
        assert_eq!(paths1.mariadb, paths2.mariadb);
    }

    #[test]
    fn test_app_data_paths_clone() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("campp");

        let paths1 = AppDataPaths {
            base_dir: base_dir.clone(),
            runtime_dir: base_dir.join("runtime"),
            config_dir: base_dir.join("config"),
            mysql_data_dir: base_dir.join("mysql").join("data"),
            logs_dir: base_dir.join("logs"),
            projects_dir: base_dir.join("projects"),
        };

        let paths2 = paths1.clone();

        assert_eq!(paths1.base_dir, paths2.base_dir);
        assert_eq!(paths1.runtime_dir, paths2.runtime_dir);
        assert_eq!(paths1.config_dir, paths2.config_dir);
    }
}
