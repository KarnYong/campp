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
    #[serde(default)]
    pub postgresql: Vec<MySQLPackage>,
    pub phpmyadmin: Vec<PhpMyAdminPackage>,
    #[serde(default)]
    pub adminer: Vec<PhpMyAdminPackage>,
    #[serde(default)]
    pub composer: Vec<PhpMyAdminPackage>,
    #[serde(default)]
    pub ngrok: Vec<MySQLPackage>,
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
    #[serde(default = "default_postgresql")]
    pub postgresql: String,
    #[serde(default = "default_adminer")]
    pub adminer: String,
    #[serde(default = "default_ngrok")]
    pub ngrok: String,
    #[serde(default = "default_composer")]
    pub composer: String,
    /// Windows only: "mysql" (default) or "mariadb". Ignored on macOS/Linux,
    /// where the engine is fixed. Old settings files load as "mysql".
    #[serde(default = "default_db_engine")]
    pub db_engine: String,
}

/// The MySQL-compatible database engine that fills the database slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbEngine {
    MySql,
    MariaDb,
}

impl DbEngine {
    /// Runtime component key ("mysql" / "mariadb").
    pub fn key(&self) -> &'static str {
        match self {
            DbEngine::MySql => "mysql",
            DbEngine::MariaDb => "mariadb",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            DbEngine::MySql => "MySQL",
            DbEngine::MariaDb => "MariaDB",
        }
    }
}

/// Linux is MariaDB-only, macOS MySQL-only; Windows follows the user's choice.
pub fn engine_for(os: &str, db_engine: &str) -> DbEngine {
    match os {
        "linux" => DbEngine::MariaDb,
        "windows" if db_engine == "mariadb" => DbEngine::MariaDb,
        _ => DbEngine::MySql,
    }
}

/// The active database engine on this machine.
// ponytail: reads settings.json per call; fine for start/locate/download paths, cache if it ever lands in a hot loop.
pub fn active_db_engine() -> DbEngine {
    engine_for(
        std::env::consts::OS,
        &crate::config::settings::AppSettings::load().package_selection.db_engine,
    )
}

fn default_composer() -> String {
    "composer-2".to_string()
}

fn default_db_engine() -> String {
    "mysql".to_string()
}

fn default_mariadb() -> String {
    "mariadb-12.3".to_string()
}

fn default_postgresql() -> String {
    "postgresql-18.3".to_string()
}

fn default_adminer() -> String {
    "adminer-5.1".to_string()
}

fn default_ngrok() -> String {
    "ngrok-3".to_string()
}

impl Default for PackageSelection {
    fn default() -> Self {
        Self {
            php: "php-8.5".to_string(),
            mysql: "mysql-8.4".to_string(),
            mariadb: "mariadb-12.3".to_string(),
            phpmyadmin: "phpmyadmin-5.2".to_string(),
            postgresql: "postgresql-18.3".to_string(),
            adminer: "adminer-5.1".to_string(),
            ngrok: "ngrok-3".to_string(),
            composer: default_composer(),
            db_engine: default_db_engine(),
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
    #[serde(default)]
    #[serde(rename = "postgresql")]
    pub postgresql: Option<BinaryConfig>,
    #[serde(default)]
    #[serde(rename = "adminer")]
    pub adminer: Option<PhpMyAdminConfig>,
    #[serde(default)]
    #[serde(rename = "ngrok")]
    pub ngrok: Option<BinaryConfig>,
    /// Command-line PHP for Linux/macOS (Windows' PHP zip already has php.exe).
    /// Entry ids match the `php` entries, so the selected PHP picks its CLI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub php_cli: Option<BinaryConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composer: Option<PhpMyAdminConfig>,
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
    /// Upstream URLs tried when the (mirrored) `urls` download fails. Same file, same checksum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_urls: Option<Urls>,
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
    /// Upstream URL tried when the (mirrored) `url` download fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_url: Option<String>,
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

impl Urls {
    /// URL for a platform key ("windowsX64", "linuxArm64", ...), if any.
    pub fn for_key(&self, key: &str) -> Option<String> {
        match key {
            "windowsX64" => self.windows_x64.clone(),
            "windowsArm64" => self.windows_arm64.clone(),
            "linuxX64" => self.linux_x64.clone(),
            "linuxArm64" => self.linux_arm64.clone(),
            "macOSX64" => self.macos_x64.clone(),
            "macOSArm64" => self.macos_arm64.clone(),
            _ => None,
        }
    }
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


/// Get all available packages from config file or defaults
pub fn get_available_packages() -> PackagesConfig {
    let config = {
        let mut guard = RUNTIME_CONFIG.write().unwrap();
        if guard.is_none() {
            *guard = load_runtime_config_from_file();
        }
        guard.clone()
    };

    let mut packages = if let Some(cfg) = &config {
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
            postgresql: cfg.binaries.postgresql.as_ref().map(|pc| pc.versions.iter().map(|v| MySQLPackage {
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
            adminer: cfg.binaries.adminer.as_ref().map(|ac| ac.versions.iter().map(|v| PhpMyAdminPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                url: v.url.clone(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect()).unwrap_or_default(),
            composer: cfg.binaries.composer.as_ref().map(|cc| cc.versions.iter().map(|v| PhpMyAdminPackage {
                id: v.id.clone(),
                version: v.version.clone(),
                display_name: v.display_name.clone(),
                url: v.url.clone(),
                eol: v.eol,
                lts: v.lts,
                recommended: v.selected,
            }).collect()).unwrap_or_default(),
            ngrok: cfg.binaries.ngrok.as_ref().map(|nc| nc.versions.iter().map(|v| MySQLPackage {
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
        }
    } else {
        // Fallback to hardcoded defaults
        tracing::info!("Using default package configuration");
        get_default_packages()
    };

    // ngrok is currently a Windows-only feature (upcoming on Linux/macOS). Expose
    // an empty list on non-Windows so every consumer — the wizard package
    // selector, the downloader, and get_ngrok_package() — treats it as
    // unavailable, i.e. it can't be selected, downloaded, or started.
    #[cfg(not(target_os = "windows"))]
    {
        packages.ngrok.clear();
    }

    packages
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
            postgresql: cfg.binaries.postgresql.as_ref()
                .and_then(|pc| pc.versions.iter().find(|v| v.selected).map(|v| v.id.clone()))
                .unwrap_or_else(|| "postgresql-18.3".to_string()),
            adminer: cfg.binaries.adminer.as_ref()
                .and_then(|ac| ac.versions.iter().find(|v| v.selected).map(|v| v.id.clone()))
                .unwrap_or_else(|| "adminer-5.1".to_string()),
            ngrok: cfg.binaries.ngrok.as_ref()
                .and_then(|nc| nc.versions.iter().find(|v| v.selected).map(|v| v.id.clone()))
                .unwrap_or_else(|| "ngrok-3".to_string()),
            composer: cfg.binaries.composer.as_ref()
                .and_then(|cc| cc.versions.iter().find(|v| v.selected).map(|v| v.id.clone()))
                .unwrap_or_else(default_composer),
            db_engine: default_db_engine(),
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

/// Get PostgreSQL package by ID
pub fn get_postgresql_package(id: &str) -> Option<MySQLPackage> {
    get_available_packages()
        .postgresql
        .into_iter()
        .find(|p| p.id == id)
}

/// Get Adminer package by ID
pub fn get_adminer_package(id: &str) -> Option<PhpMyAdminPackage> {
    get_available_packages()
        .adminer
        .into_iter()
        .find(|p| p.id == id)
}

/// Get Composer package by ID
pub fn get_composer_package(id: &str) -> Option<PhpMyAdminPackage> {
    get_available_packages()
        .composer
        .into_iter()
        .find(|p| p.id == id)
}

/// Get ngrok package by ID
pub fn get_ngrok_package(id: &str) -> Option<MySQLPackage> {
    get_available_packages()
        .ngrok
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
                version: "8.5.12".to_string(),
                display_name: "PHP 8.5.12 (Latest)".to_string(),
                windows_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.12/php-8.5.12-Win32-vs17-x64.zip".to_string(),
                // No native ARM64 PHP build published; use the x64 build under
                // Windows 11 x64 emulation (same approach as MySQL below). The
                // previous x86 (32-bit) build was the wrong architecture.
                windows_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.12/php-8.5.12-Win32-vs17-x64.zip".to_string(),
                linux_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.11/php-8.5.8-fpm-linux-x86_64.tar.gz".to_string(),
                linux_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.11/php-8.5.8-fpm-linux-aarch64.tar.gz".to_string(),
                macos_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.11/php-8.5.8-fpm-macos-x86_64.tar.gz".to_string(),
                macos_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/php-8.5.11/php-8.5.8-fpm-macos-aarch64.tar.gz".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        mysql: vec![
            MySQLPackage {
                id: "mysql-8.4".to_string(),
                version: "8.4.11".to_string(),
                display_name: "MySQL 8.4.11 LTS (Recommended)".to_string(),
                windows_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.11/mysql-8.4.11-winx64.zip".to_string(),
                windows_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.11/mysql-8.4.11-winx64.zip".to_string(),
                linux_x64: String::new(),
                linux_arm64: String::new(),
                macos_x64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.11/mysql-8.4.11-macos15-x86_64.tar.gz".to_string(),
                macos_arm64: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/mysql-8.4.11/mysql-8.4.11-macos15-arm64.tar.gz".to_string(),
                eol: false,
                lts: true,
                recommended: true,
            },
        ],
        mariadb: vec![
            MySQLPackage {
                id: "mariadb-12.3".to_string(),
                version: "12.3.3".to_string(),
                display_name: "MariaDB 12.3.3 LTS (Latest)".to_string(),
                windows_x64: "https://archive.mariadb.org/mariadb-12.3.3/winx64-packages/mariadb-12.3.3-winx64.zip".to_string(),
                windows_arm64: "https://archive.mariadb.org/mariadb-12.3.3/winx64-packages/mariadb-12.3.3-winx64.zip".to_string(),
                linux_x64: "https://archive.mariadb.org/mariadb-12.3.3/bintar-linux-systemd-x86_64/mariadb-12.3.3-linux-systemd-x86_64.tar.gz".to_string(),
                linux_arm64: String::new(),
                macos_x64: String::new(),
                macos_arm64: String::new(),
                eol: false,
                lts: true,
                recommended: true,
            },
        ],
        phpmyadmin: vec![
            PhpMyAdminPackage {
                id: "phpmyadmin-5.2".to_string(),
                version: "5.2.3".to_string(),
                display_name: "phpMyAdmin 5.2.3 (Latest)".to_string(),
                url: "https://github.com/KarnYong/campp-runtime-binaries/releases/download/phpmyadmin-5.2.3/phpMyAdmin-5.2.3-all-languages.zip".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        postgresql: vec![
            MySQLPackage {
                id: "postgresql-18.3".to_string(),
                version: "18.6.1".to_string(),
                display_name: "PostgreSQL 18.6.1 (Latest)".to_string(),
                windows_x64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.1/postgresql-18.6.1-x86_64-pc-windows-msvc.zip".to_string(),
                // theseus-rs publishes no Windows ARM64 build; use the x64 build
                // under Windows 11 x64 emulation (same approach as MySQL).
                windows_arm64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.1/postgresql-18.6.1-x86_64-pc-windows-msvc.zip".to_string(),
                // Linux uses 18.6.0: the 18.6.1 rebuild needs glibc 2.36 and fails on Ubuntu 22.04.
                linux_x64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.0/postgresql-18.6.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                linux_arm64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.0/postgresql-18.6.0-aarch64-unknown-linux-gnu.tar.gz".to_string(),
                macos_x64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.1/postgresql-18.6.1-x86_64-apple-darwin.tar.gz".to_string(),
                macos_arm64: "https://github.com/theseus-rs/postgresql-binaries/releases/download/18.6.1/postgresql-18.6.1-aarch64-apple-darwin.tar.gz".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        adminer: vec![
            PhpMyAdminPackage {
                id: "adminer-5.1".to_string(),
                version: "5.1.0".to_string(),
                display_name: "Adminer 5.1.0 (Latest)".to_string(),
                url: "https://github.com/vrana/adminer/releases/download/v5.1.0/adminer-5.1.0-mysql.php".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
        composer: vec![],
        ngrok: vec![
            MySQLPackage {
                id: "ngrok-3".to_string(),
                version: "3.19.2".to_string(),
                display_name: "ngrok 3.19.2".to_string(),
                windows_x64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-windows-amd64.zip".to_string(),
                windows_arm64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-windows-arm64.zip".to_string(),
                linux_x64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-linux-amd64.tgz".to_string(),
                linux_arm64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-linux-arm64.tgz".to_string(),
                macos_x64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-darwin-amd64.tgz".to_string(),
                macos_arm64: "https://bin.ngrok.com/c/bNyj1mQVY4c/ngrok-v3-stable-darwin-arm64.tgz".to_string(),
                eol: false,
                lts: false,
                recommended: true,
            },
        ],
    }
}

#[cfg(test)]
mod db_engine_tests {
    use super::*;

    #[test]
    fn engine_is_fixed_on_linux_and_macos_and_chosen_on_windows() {
        assert_eq!(engine_for("linux", "mysql"), DbEngine::MariaDb);
        assert_eq!(engine_for("macos", "mariadb"), DbEngine::MySql);
        assert_eq!(engine_for("windows", "mariadb"), DbEngine::MariaDb);
        assert_eq!(engine_for("windows", "mysql"), DbEngine::MySql);
        assert_eq!(engine_for("windows", "bogus"), DbEngine::MySql);
    }

    #[test]
    fn fallback_urls_are_optional_and_resolve_per_platform() {
        let with = r#"{"id":"postgresql-17","version":"17.12.0","selected":false,"display_name":"PostgreSQL 17",
            "urls":{"linuxX64":"https://mirror/pg.tar.gz"},
            "fallback_urls":{"linuxX64":"https://upstream/pg.tar.gz"}}"#;
        let v: VersionInfo = serde_json::from_str(with).unwrap();
        assert_eq!(v.fallback_urls.unwrap().for_key("linuxX64").as_deref(), Some("https://upstream/pg.tar.gz"));

        let without = r#"{"id":"php-8.4","version":"8.4.27","selected":false,"display_name":"PHP","urls":{}}"#;
        let v: VersionInfo = serde_json::from_str(without).unwrap();
        assert!(v.fallback_urls.is_none());
        assert!(!serde_json::to_string(&v).unwrap().contains("fallback"), "absent field must not be written");

        let single = r#"{"id":"adminer-6.1","version":"6.3.0","selected":false,"display_name":"Adminer",
            "url":"https://mirror/a.php","fallback_url":"https://upstream/a.php"}"#;
        let s: VersionInfoSingleUrl = serde_json::from_str(single).unwrap();
        assert_eq!(s.fallback_url.as_deref(), Some("https://upstream/a.php"));
    }

    #[test]
    fn old_selection_without_composer_loads_the_default() {
        let old = r#"{"php":"php-8.5","mysql":"mysql-8.4","phpmyadmin":"phpmyadmin-5.2"}"#;
        let sel: PackageSelection = serde_json::from_str(old).unwrap();
        assert_eq!(sel.composer, "composer-2");
    }

    #[test]
    fn old_selection_without_db_engine_loads_as_mysql() {
        let old = r#"{"php":"php-8.5","mysql":"mysql-8.4","phpmyadmin":"phpmyadmin-5.2"}"#;
        let sel: PackageSelection = serde_json::from_str(old).unwrap();
        assert_eq!(sel.db_engine, "mysql");
    }
}
