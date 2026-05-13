use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::RwLock;

/// Available package versions for each component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesConfig {
    pub php: Vec<PhpPackage>,
    pub mysql: Vec<MySQLPackage>,
    #[serde(default)]
    pub mariadb: Vec<MySQLPackage>,
    pub phpmyadmin: Vec<PhpMyAdminPackage>,
}

/// PHP package with version and download URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpPackage {
    pub id: String,
    pub version: String,
    pub display_name: String,
    #[serde(rename = "windowsX64")]
    pub windows_x64: String,
    #[serde(rename = "windowsArm64")]
    pub windows_arm64: String,
    #[serde(rename = "linuxX64")]
    pub linux_x64: String,
    #[serde(rename = "linuxArm64")]
    pub linux_arm64: String,
    #[serde(rename = "macOSX64")]
    pub macos_x64: String,
    #[serde(rename = "macOSArm64")]
    pub macos_arm64: String,
    #[serde(default)]
    pub eol: bool,
    #[serde(default)]
    pub lts: bool,
    #[serde(default)]
    pub recommended: bool,
}

/// MySQL package with version and download URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLPackage {
    pub id: String,
    pub version: String,
    pub display_name: String,
    #[serde(rename = "windowsX64")]
    pub windows_x64: String,
    #[serde(rename = "windowsArm64")]
    pub windows_arm64: String,
    #[serde(rename = "linuxX64")]
    pub linux_x64: String,
    #[serde(rename = "linuxArm64")]
    pub linux_arm64: String,
    #[serde(rename = "macOSX64")]
    pub macos_x64: String,
    #[serde(rename = "macOSArm64")]
    pub macos_arm64: String,
    #[serde(default)]
    pub eol: bool,
    #[serde(default)]
    pub lts: bool,
    #[serde(default)]
    pub recommended: bool,
}

/// phpMyAdmin package with version and download URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpMyAdminPackage {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub url: String,
    #[serde(default)]
    pub eol: bool,
    #[serde(default)]
    pub lts: bool,
    #[serde(default)]
    pub recommended: bool,
}

/// User's selected package versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSelection {
    pub php: String,
    pub mysql: String,
    #[serde(default = "default_mariadb")]
    pub mariadb: String,
    pub phpmyadmin: String,
}

fn default_mariadb() -> String {
    "mariadb-12.3".to_string()
}

impl Default for PackageSelection {
    fn default() -> Self {
        Self {
            php: "php-8.5".to_string(),
            mysql: "mysql-8.4".to_string(),
            mariadb: "mariadb-12.3".to_string(),
            phpmyadmin: "phpmyadmin-5.2".to_string(),
        }
    }
}

/// Runtime configuration loaded from runtime-config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub version: String,
    pub binaries: BinariesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinariesConfig {
    #[serde(rename = "caddy")]
    pub caddy: BinaryConfig,
    #[serde(rename = "php")]
    pub php: BinaryConfig,
    #[serde(rename = "mysql")]
    pub mysql: BinaryConfig,
    #[serde(default)]
    #[serde(rename = "mariadb")]
    pub mariadb: Option<BinaryConfig>,
    #[serde(rename = "phpmyadmin")]
    pub phpmyadmin: PhpMyAdminConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryConfig {
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpMyAdminConfig {
    pub versions: Vec<VersionInfoSingleUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub version: String,
    pub selected: bool,
    pub display_name: String,
    #[serde(default)]
    pub eol: bool,
    #[serde(default)]
    pub lts: bool,
    #[serde(default)]
    pub checksums: Checksums,
    pub urls: Urls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksums {
    #[serde(rename = "windowsX64", default)]
    pub windows_x64: Option<String>,
    #[serde(rename = "windowsArm64", default)]
    pub windows_arm64: Option<String>,
    #[serde(rename = "linuxX64", default)]
    pub linux_x64: Option<String>,
    #[serde(rename = "linuxArm64", default)]
    pub linux_arm64: Option<String>,
    #[serde(rename = "macOSX64", default)]
    pub macos_x64: Option<String>,
    #[serde(rename = "macOSArm64", default)]
    pub macos_arm64: Option<String>,
}

impl Default for Checksums {
    fn default() -> Self {
        Self {
            windows_x64: None,
            windows_arm64: None,
            linux_x64: None,
            linux_arm64: None,
            macos_x64: None,
            macos_arm64: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfoSingleUrl {
    pub id: String,
    pub version: String,
    pub selected: bool,
    pub display_name: String,
    #[serde(default)]
    pub eol: bool,
    #[serde(default)]
    pub lts: bool,
    #[serde(default)]
    pub checksum: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Urls {
    #[serde(rename = "windowsX64")]
    pub windows_x64: Option<String>,
    #[serde(rename = "windowsArm64")]
    pub windows_arm64: Option<String>,
    #[serde(rename = "linuxX64")]
    pub linux_x64: Option<String>,
    #[serde(rename = "linuxArm64")]
    pub linux_arm64: Option<String>,
    #[serde(rename = "macOSX64")]
    pub macos_x64: Option<String>,
    #[serde(rename = "macOSArm64")]
    pub macos_arm64: Option<String>,
}

/// Global runtime config cache
static RUNTIME_CONFIG: RwLock<Option<RuntimeConfig>> = RwLock::new(None);

/// Load runtime configuration from file
pub fn load_runtime_config_from_file() -> Option<RuntimeConfig> {
    // Try to load from various locations
    let mut paths_to_try = vec![
        "runtime-config.json".to_string(),
        "src-tauri/runtime-config.json".to_string(),
    ];

    // Also try alongside the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths_to_try.push(exe_dir.join("runtime-config.json").to_string_lossy().to_string());

            // Linux AppImage: resources are in the .data/lib/ or ../lib/ relative to binary
            #[cfg(target_os = "linux")]
            {
                // AppImage extracts to a temporary mount point like /tmp/.mount_XXXX/usr/bin/
                // Resources are at /tmp/.mount_XXXX/usr/lib/
                if let Some(parent) = exe_dir.parent() {
                    paths_to_try.push(parent.join("lib").join("runtime-config.json").to_string_lossy().to_string());
                }
                // Also try ../share/ which some packagers use
                if let Some(parent) = exe_dir.parent() {
                    paths_to_try.push(parent.join("share").join("runtime-config.json").to_string_lossy().to_string());
                }
            }
        }
    }

    // Try from Cargo manifest dir (for dev mode)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        paths_to_try.push(std::path::PathBuf::from(manifest_dir)
            .join("runtime-config.json")
            .to_string_lossy()
            .to_string());
    }

    // Try XDG data directories on Linux
    #[cfg(target_os = "linux")]
    {
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            paths_to_try.push(std::path::PathBuf::from(data_home)
                .join("campp")
                .join("runtime-config.json")
                .to_string_lossy()
                .to_string());
        }
        if let Some(data_home) = std::env::var_os("HOME") {
            paths_to_try.push(std::path::PathBuf::from(data_home)
                .join(".local")
                .join("share")
                .join("campp")
                .join("runtime-config.json")
                .to_string_lossy()
                .to_string());
        }
    }

    tracing::debug!("Searching for runtime-config.json in {} paths", paths_to_try.len());
    for path in &paths_to_try {
        tracing::debug!("  Checking: {}", path);
    }

    for path in paths_to_try {
        if let Ok(content) = fs::read_to_string(&path) {
            match serde_json::from_str::<RuntimeConfig>(&content) {
                Ok(config) => {
                    tracing::info!("Loaded runtime configuration from {}", path);
                    return Some(config);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse runtime-config.json from {}: {}", path, e);
                }
            }
        }
    }

    tracing::error!("runtime-config.json not found in any search path. Tried all known locations.");
    None
}

/// Get the platform-appropriate database display name
fn get_database_display_name(display_name: &str) -> String {
    // On Linux, show "MariaDB", on Windows/macOS show "MySQL"
    #[cfg(target_os = "linux")]
    {
        display_name.replace("MySQL", "MariaDB")
    }
    #[cfg(not(target_os = "linux"))]
    {
        display_name.replace("MariaDB", "MySQL")
    }
}

/// Get all available packages from config file or defaults
pub fn get_available_packages() -> PackagesConfig {
    let config = {
        let mut guard = RUNTIME_CONFIG.write().unwrap();
        if guard.is_none() {
            *guard = load_runtime_config_from_file();
        }
        guard.clone()
    };

    if let Some(cfg) = &config {
        // Convert from config format to package format
        PackagesConfig {
            php: cfg.binaries.php.versions.iter().map(|v| PhpPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                windows_x64: v.urls.windows_x64.clone().unwrap_or_default(),
                windows_arm64: v.urls.windows_arm64.clone().unwrap_or_default(),
                linux_x64: v.urls.linux_x64.clone().unwrap_or_default(),
                linux_arm64: v.urls.linux_arm64.clone().unwrap_or_default(),
                macos_x64: v.urls.macos_x64.clone().unwrap_or_default(),
                macos_arm64: v.urls.macos_arm64.clone().unwrap_or_default(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect(),
            mysql: cfg.binaries.mysql.versions.iter().map(|v| MySQLPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                windows_x64: v.urls.windows_x64.clone().unwrap_or_default(),
                windows_arm64: v.urls.windows_arm64.clone().unwrap_or_default(),
                linux_x64: v.urls.linux_x64.clone().unwrap_or_default(),
                linux_arm64: v.urls.linux_arm64.clone().unwrap_or_default(),
                macos_x64: v.urls.macos_x64.clone().unwrap_or_default(),
                macos_arm64: v.urls.macos_arm64.clone().unwrap_or_default(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect(),
            mariadb: cfg.binaries.mariadb.as_ref().map(|mc| mc.versions.iter().map(|v| MySQLPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                windows_x64: v.urls.windows_x64.clone().unwrap_or_default(),
                windows_arm64: v.urls.windows_arm64.clone().unwrap_or_default(),
                linux_x64: v.urls.linux_x64.clone().unwrap_or_default(),
                linux_arm64: v.urls.linux_arm64.clone().unwrap_or_default(),
                macos_x64: v.urls.macos_x64.clone().unwrap_or_default(),
                macos_arm64: v.urls.macos_arm64.clone().unwrap_or_default(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect()).unwrap_or_default(),
            phpmyadmin: cfg.binaries.phpmyadmin.versions.iter().map(|v| PhpMyAdminPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                url: v.url.clone(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect(),
        }
    } else {
        // Fallback to hardcoded defaults
        tracing::info!("Using default package configuration");
        get_default_packages()
    }
}

/// Get the selected package IDs from config
pub fn get_selected_package_ids() -> PackageSelection {
    let config = {
        let mut guard = RUNTIME_CONFIG.write().unwrap();
        if guard.is_none() {
            *guard = load_runtime_config_from_file();
        }
        guard.clone()
    };

    if let Some(cfg) = &config {
        PackageSelection {
            php: cfg.binaries.php.versions.iter()
                .find(|v| v.selected)
                .map(|v| v.id.clone())
                .unwrap_or_else(|| "php-8.5".to_string()),
            mysql: cfg.binaries.mysql.versions.iter()
                .find(|v| v.selected)
                .map(|v| v.id.clone())
                .unwrap_or_else(|| "mysql-8.4".to_string()),
            mariadb: cfg.binaries.mariadb.as_ref()
                .and_then(|mc| mc.versions.iter().find(|v| v.selected).map(|v| v.id.clone()))
                .unwrap_or_else(|| "mariadb-12.3".to_string()),
            phpmyadmin: cfg.binaries.phpmyadmin.versions.iter()
                .find(|v| v.selected)
                .map(|v| v.id.clone())
                .unwrap_or_else(|| "phpmyadmin-5.2".to_string()),
        }
    } else {
        PackageSelection::default()
    }
}

/// Get PHP package by ID
pub fn get_php_package(id: &str) -> Option<PhpPackage> {
    get_available_packages()
        .php
        .into_iter()
        .find(|p| p.id == id)
}

/// Get MySQL package by ID
pub fn get_mysql_package(id: &str) -> Option<MySQLPackage> {
    get_available_packages()
        .mysql
        .into_iter()
        .find(|p| p.id == id)
}

/// Get MariaDB package by ID
pub fn get_mariadb_package(id: &str) -> Option<MySQLPackage> {
    get_available_packages()
        .mariadb
        .into_iter()
        .find(|p| p.id == id)
}

/// Get phpMyAdmin package by ID
pub fn get_phpmyadmin_package(id: &str) -> Option<PhpMyAdminPackage> {
    get_available_packages()
        .phpmyadmin
        .into_iter()
        .find(|p| p.id == id)
}

/// Reload the runtime configuration (call after modifying the config file)
pub fn reload_runtime_config() {
    let mut guard = RUNTIME_CONFIG.write().unwrap();
    *guard = load_runtime_config_from_file();
}

/// Load runtime config from an explicit resource directory path (e.g. Tauri's resource_dir)
pub fn load_config_from_resource_dir(resource_dir: &std::path::Path) {
    let config_path = resource_dir.join("runtime-config.json");
    tracing::info!("Trying to load runtime config from resource dir: {}", config_path.display());

    if !config_path.exists() {
        tracing::error!("runtime-config.json NOT FOUND at resource dir: {}", config_path.display());
        // List files in resource dir to help debug
        if let Ok(entries) = fs::read_dir(resource_dir) {
            tracing::info!("Files in resource dir ({}):", resource_dir.display());
            for entry in entries.flatten() {
                tracing::info!("  {}", entry.path().display());
            }
        } else {
            tracing::error!("Cannot read resource dir: {}", resource_dir.display());
        }
        // Don't return — try load_runtime_config_from_file as fallback
        let mut guard = RUNTIME_CONFIG.write().unwrap();
        if guard.is_none() {
            *guard = load_runtime_config_from_file();
        }
        return;
    }

    if let Ok(content) = fs::read_to_string(&config_path) {
        match serde_json::from_str::<RuntimeConfig>(&content) {
            Ok(config) => {
                tracing::info!("Loaded runtime configuration from {}", config_path.display());
                let mut guard = RUNTIME_CONFIG.write().unwrap();
                *guard = Some(config);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to parse runtime-config.json from {}: {}", config_path.display(), e);
            }
        }
    }
    tracing::error!("Could not read runtime-config.json from resource dir: {}", config_path.display());
    // Fallback
    let mut guard = RUNTIME_CONFIG.write().unwrap();
    if guard.is_none() {
        *guard = load_runtime_config_from_file();
    }
}

/// Get the runtime configuration
pub fn get_config() -> Option<RuntimeConfig> {
    let mut guard = RUNTIME_CONFIG.write().unwrap();
    if guard.is_none() {
        *guard = load_runtime_config_from_file();
    }
    guard.clone()
}

/// Get default hardcoded packages (fallback when config file is not available)
fn get_default_packages() -> PackagesConfig {
    PackagesConfig {
        php: vec![
            PhpPackage {
                id: "php-8.5".to_string(),
                version: "8.5.1".to_string(),
                display_name: "PHP 8.5.1 (Latest)".to_string(),
                windows_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.5.1-Win32-vs17-x64.zip".to_string(),
                windows_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.5.1-Win32-vs17-x86.zip".to_string(),
                linux_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.4.18-fpm-linux-x86_64.tar.gz".to_string(),
                linux_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.4.18-fpm-linux-aarch64.tar.gz".to_string(),
                macos_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.4.18-fpm-macos-x86_64.tar.gz".to_string(),
                macos_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.1/php-8.4.18-fpm-macos-aarch64.tar.gz".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        mysql: vec![
            MySQLPackage {
                id: "mysql-8.4".to_string(),
                version: "8.4.0".to_string(),
                display_name: "MySQL 8.4.0 LTS (Recommended)".to_string(),
                windows_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.0/mysql-8.4.0-winx64.zip".to_string(),
                windows_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.0/mysql-8.4.0-winx64.zip".to_string(),
                linux_x64: String::new(),
                linux_arm64: String::new(),
                macos_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.0/mysql-8.4.0-macos14-x86_64.tar.gz".to_string(),
                macos_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.0/mysql-8.4.0-macos14-arm64.tar.gz".to_string(),
                eol: false,
                lts: true,
                recommended: true,
            },
        ],
        mariadb: vec![
            MySQLPackage {
                id: "mariadb-12.3".to_string(),
                version: "12.3.1".to_string(),
                display_name: "MariaDB 12.3.1 (Latest)".to_string(),
                windows_x64: String::new(),
                windows_arm64: String::new(),
                linux_x64: "https://archive.mariadb.org/mariadb-12.3.1/bintar-linux-systemd-x86_64/mariadb-12.3.1-linux-systemd-x86_64.tar.gz".to_string(),
                linux_arm64: "https://archive.mariadb.org/mariadb-12.3.1/bintar-linux-systemd-aarch64/mariadb-12.3.1-linux-systemd-aarch64.tar.gz".to_string(),
                macos_x64: String::new(),
                macos_arm64: String::new(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        phpmyadmin: vec![
            PhpMyAdminPackage {
                id: "phpmyadmin-5.2".to_string(),
                version: "5.2.2".to_string(),
                display_name: "phpMyAdmin 5.2.2 (Latest)".to_string(),
                url: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/phpmyadmin-5.2.2/phpMyAdmin-5.2.2-all-languages.zip".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
    }
}
