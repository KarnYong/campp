use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use reqwest::Client;

use crate::runtime::locator::get_app_data_paths;
use crate::runtime::packages::{PackageSelection, get_php_package, get_mysql_package, get_mariadb_package, get_phpmyadmin_package, get_config};
use sha2::{Digest, Sha256};

/// Runtime configuration loaded from runtime-config.json (shared with packages.rs)
pub use crate::runtime::packages::{
    RuntimeConfig, BinariesConfig, BinaryConfig, PhpMyAdminConfig, VersionInfo, VersionInfoSingleUrl, Urls, Checksums
};

/// Binary component types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryComponent {
    Caddy,
    Php,
    MySQL,
    MariaDB,
    PhpMyAdmin,
}

impl BinaryComponent {
    pub fn name(&self) -> &str {
        match self {
            BinaryComponent::Caddy => "Caddy",
            BinaryComponent::Php => "PHP",
            BinaryComponent::MySQL => "MySQL",
            BinaryComponent::MariaDB => "MariaDB",
            BinaryComponent::PhpMyAdmin => "phpMyAdmin",
        }
    }

    pub fn version(&self) -> String {
        let config = match get_config() {
            Some(c) => c,
            None => return String::new(),
        };
        match self {
            BinaryComponent::Caddy => {
                config.binaries.caddy.versions.iter()
                    .find(|v| v.selected)
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| config.binaries.caddy.versions.first().map(|v| v.version.clone()).unwrap_or_default())
            }
            BinaryComponent::Php => {
                config.binaries.php.versions.iter()
                    .find(|v| v.selected)
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| config.binaries.php.versions.first().map(|v| v.version.clone()).unwrap_or_default())
            }
            BinaryComponent::MySQL => {
                config.binaries.mysql.versions.iter()
                    .find(|v| v.selected)
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| config.binaries.mysql.versions.first().map(|v| v.version.clone()).unwrap_or_default())
            }
            BinaryComponent::MariaDB => {
                config.binaries.mariadb.as_ref()
                    .and_then(|mc| mc.versions.iter().find(|v| v.selected).map(|v| v.version.clone()))
                    .or_else(|| config.binaries.mariadb.as_ref().and_then(|mc| mc.versions.first().map(|v| v.version.clone())))
                    .unwrap_or_default()
            }
            BinaryComponent::PhpMyAdmin => {
                config.binaries.phpmyadmin.versions.iter()
                    .find(|v| v.selected)
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| config.binaries.phpmyadmin.versions.first().map(|v| v.version.clone()).unwrap_or_default())
            }
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.name(), self.version())
    }

    pub fn binary_name(&self) -> &str {
        match self {
            BinaryComponent::Caddy => "caddy",
            BinaryComponent::Php => "php",
            BinaryComponent::MySQL => "mysql",
            BinaryComponent::MariaDB => "mariadb",
            BinaryComponent::PhpMyAdmin => "phpmyadmin",
        }
    }
}

impl RuntimeDownloader {
    /// Get version for a component based on current package selection
    pub fn get_component_version(&self, component: &BinaryComponent) -> String {
        if let Some(selection) = &self.package_selection {
            match component {
                BinaryComponent::Php => {
                    if let Some(pkg) = get_php_package(&selection.php) {
                        return pkg.version;
                    }
                }
                BinaryComponent::MySQL => {
                    if let Some(pkg) = get_mysql_package(&selection.mysql) {
                        return pkg.version;
                    }
                }
                BinaryComponent::MariaDB => {
                    if let Some(pkg) = get_mariadb_package(&selection.mariadb) {
                        return pkg.version;
                    }
                }
                BinaryComponent::PhpMyAdmin => {
                    if let Some(pkg) = get_phpmyadmin_package(&selection.phpmyadmin) {
                        return pkg.version;
                    }
                }
                BinaryComponent::Caddy => {
                    // Caddy uses default version
                }
            }
        }

        // Fall back to default config
        component.version()
    }
}

/// Platform information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    WindowsX64,
    WindowsArm64,
    MacOSX64,
    MacOSArm64,
    LinuxX64,
    LinuxArm64,
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Platform::WindowsX64;

        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        return Platform::WindowsArm64;

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Platform::MacOSX64;

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Platform::MacOSArm64;

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Platform::LinuxX64;

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Platform::LinuxArm64;

        // Default to Linux x64 for unknown platforms
        Platform::LinuxX64
    }

    /// Get the platform identifier for URLs
    pub fn identifier(&self) -> &str {
        match self {
            Platform::WindowsX64 => "windows-x64",
            Platform::WindowsArm64 => "windows-arm64",
            Platform::MacOSX64 => "darwin-x64",
            Platform::MacOSArm64 => "darwin-arm64",
            Platform::LinuxX64 => "linux-x64",
            Platform::LinuxArm64 => "linux-arm64",
        }
    }

    /// Get the file extension for archives
    pub fn archive_extension(&self) -> &str {
        match self {
            Platform::WindowsX64 | Platform::WindowsArm64 => "zip",
            Platform::MacOSX64 | Platform::MacOSArm64 | Platform::LinuxX64 | Platform::LinuxArm64 => {
                "tar.gz"
            }
        }
    }

    /// Get the URL key for config lookup (matches JSON keys)
    pub fn url_key(&self) -> String {
        match self {
            Platform::WindowsX64 => "windowsX64",
            Platform::WindowsArm64 => "windowsArm64",
            Platform::LinuxX64 => "linuxX64",
            Platform::LinuxArm64 => "linuxArm64",
            Platform::MacOSX64 => "macOSX64",
            Platform::MacOSArm64 => "macOSArm64",
        }.to_string()
    }
}

/// Runtime manifest with binary URLs and checksums
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeManifest {
    pub version: String,
    pub binaries: HashMap<String, BinaryInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BinaryInfo {
    pub url: String,
    pub checksum: String,
    pub size: u64,
}

/// Download progress information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub step: DownloadStep,
    pub percent: u8,
    pub current_component: String,
    pub component_display: String,
    pub version: String,
    pub total_components: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

/// Download step
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStep {
    Downloading,
    Extracting,
    Installing,
    Complete,
    Error(String),
}

pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Runtime binary downloader
pub struct RuntimeDownloader {
    base_url: String,
    platform: Platform,
    client: Client,
    package_selection: Option<PackageSelection>,
}

impl RuntimeDownloader {
    /// Create a new runtime downloader
    pub fn new() -> Self {
        Self {
            base_url: "https://github.com".to_string(),
            platform: Platform::current(),
            client: Client::new(),
            package_selection: None,
        }
    }

    /// Create a new runtime downloader with custom package selection
    pub fn with_packages(package_selection: PackageSelection) -> Self {
        Self {
            base_url: "https://github.com".to_string(),
            platform: Platform::current(),
            client: Client::new(),
            package_selection: Some(package_selection),
        }
    }

    /// Get the URL for a binary component from config
    fn get_binary_url(&self, component: BinaryComponent) -> String {
        // Use selected packages if available, otherwise fall back to default config
        if let Some(selection) = &self.package_selection {
            match component {
                BinaryComponent::Php => {
                    if let Some(pkg) = get_php_package(&selection.php) {
                        return match self.platform {
                            Platform::WindowsX64 => pkg.windows_x64,
                            Platform::WindowsArm64 => pkg.windows_arm64,
                            Platform::MacOSX64 => pkg.macos_x64,
                            Platform::MacOSArm64 => pkg.macos_arm64,
                            Platform::LinuxX64 => pkg.linux_x64,
                            Platform::LinuxArm64 => pkg.linux_arm64,
                        };
                    }
                }
                BinaryComponent::MySQL => {
                    if let Some(pkg) = get_mysql_package(&selection.mysql) {
                        return match self.platform {
                            Platform::WindowsX64 => pkg.windows_x64,
                            Platform::WindowsArm64 => pkg.windows_arm64,
                            Platform::MacOSX64 => pkg.macos_x64,
                            Platform::MacOSArm64 => pkg.macos_arm64,
                            Platform::LinuxX64 => pkg.linux_x64,
                            Platform::LinuxArm64 => pkg.linux_arm64,
                        };
                    }
                }
                BinaryComponent::MariaDB => {
                    if let Some(pkg) = get_mariadb_package(&selection.mariadb) {
                        return match self.platform {
                            Platform::WindowsX64 => pkg.windows_x64,
                            Platform::WindowsArm64 => pkg.windows_arm64,
                            Platform::MacOSX64 => pkg.macos_x64,
                            Platform::MacOSArm64 => pkg.macos_arm64,
                            Platform::LinuxX64 => pkg.linux_x64,
                            Platform::LinuxArm64 => pkg.linux_arm64,
                        };
                    }
                }
                BinaryComponent::PhpMyAdmin => {
                    if let Some(pkg) = get_phpmyadmin_package(&selection.phpmyadmin) {
                        return pkg.url;
                    }
                }
                BinaryComponent::Caddy => {
                    // Caddy doesn't have package selection, use default
                }
            }
        }

        // Fall back to default config
        let config = match get_config() {
            Some(c) => c,
            None => return String::new(),
        };

        match component {
            BinaryComponent::Caddy => {
                let version_info = config.binaries.caddy.versions.iter()
                    .find(|v| v.selected)
                    .or_else(|| config.binaries.caddy.versions.first())
                    .unwrap();
                match self.platform {
                    Platform::WindowsX64 => version_info.urls.windows_x64.clone().unwrap_or_default(),
                    Platform::WindowsArm64 => version_info.urls.windows_arm64.clone().unwrap_or_default(),
                    Platform::MacOSX64 => version_info.urls.macos_x64.clone().unwrap_or_default(),
                    Platform::MacOSArm64 => version_info.urls.macos_arm64.clone().unwrap_or_default(),
                    Platform::LinuxX64 => version_info.urls.linux_x64.clone().unwrap_or_default(),
                    Platform::LinuxArm64 => version_info.urls.linux_arm64.clone().unwrap_or_default(),
                }
            }
            BinaryComponent::Php => {
                let version_info = config.binaries.php.versions.iter()
                    .find(|v| v.selected)
                    .or_else(|| config.binaries.php.versions.first())
                    .unwrap();
                match self.platform {
                    Platform::WindowsX64 => version_info.urls.windows_x64.clone().unwrap_or_default(),
                    Platform::WindowsArm64 => version_info.urls.windows_arm64.clone().unwrap_or_default(),
                    Platform::MacOSX64 => version_info.urls.macos_x64.clone().unwrap_or_default(),
                    Platform::MacOSArm64 => version_info.urls.macos_arm64.clone().unwrap_or_default(),
                    Platform::LinuxX64 => version_info.urls.linux_x64.clone().unwrap_or_default(),
                    Platform::LinuxArm64 => version_info.urls.linux_arm64.clone().unwrap_or_default(),
                }
            }
            BinaryComponent::MySQL => {
                let version_info = config.binaries.mysql.versions.iter()
                    .find(|v| v.selected)
                    .or_else(|| config.binaries.mysql.versions.first())
                    .unwrap();
                match self.platform {
                    Platform::WindowsX64 => version_info.urls.windows_x64.clone().unwrap_or_default(),
                    Platform::WindowsArm64 => version_info.urls.windows_arm64.clone().unwrap_or_default(),
                    Platform::MacOSX64 => version_info.urls.macos_x64.clone().unwrap_or_default(),
                    Platform::MacOSArm64 => version_info.urls.macos_arm64.clone().unwrap_or_default(),
                    Platform::LinuxX64 => version_info.urls.linux_x64.clone().unwrap_or_default(),
                    Platform::LinuxArm64 => version_info.urls.linux_arm64.clone().unwrap_or_default(),
                }
            }
            BinaryComponent::MariaDB => {
                if let Some(mc) = &config.binaries.mariadb {
                    let version_info = mc.versions.iter()
                        .find(|v| v.selected)
                        .or_else(|| mc.versions.first())
                        .unwrap();
                    match self.platform {
                        Platform::WindowsX64 => version_info.urls.windows_x64.clone().unwrap_or_default(),
                        Platform::WindowsArm64 => version_info.urls.windows_arm64.clone().unwrap_or_default(),
                        Platform::MacOSX64 => version_info.urls.macos_x64.clone().unwrap_or_default(),
                        Platform::MacOSArm64 => version_info.urls.macos_arm64.clone().unwrap_or_default(),
                        Platform::LinuxX64 => version_info.urls.linux_x64.clone().unwrap_or_default(),
                        Platform::LinuxArm64 => version_info.urls.linux_arm64.clone().unwrap_or_default(),
                    }
                } else {
                    String::new()
                }
            }
            BinaryComponent::PhpMyAdmin => {
                let version_info = config.binaries.phpmyadmin.versions.iter()
                    .find(|v| v.selected)
                    .or_else(|| config.binaries.phpmyadmin.versions.first())
                    .unwrap();
                version_info.url.clone()
            }
        }
    }

    /// Extract file extension from URL
    fn get_extension_from_url(url: &str) -> String {
        // Get the filename from the URL
        if let Some(filename) = url.split('/').last() {
            // Check for .tar.gz first (compound extension)
            if filename.ends_with(".tar.gz") {
                return "tar.gz".to_string();
            }
            // Otherwise get the extension after the last dot
            if let Some(ext) = filename.split('.').last() {
                return ext.to_string();
            }
        }
        // Default to zip if we can't determine
        "zip".to_string()
    }

    /// Download a single binary component
    async fn download_component(
        &self,
        component: BinaryComponent,
        dest_dir: &Path,
        progress_cb: &ProgressCallback,
        current: u8,
        total: u8,
    ) -> Result<PathBuf, String> {
        let url = self.get_binary_url(component);
        let extension = Self::get_extension_from_url(&url);

        tracing::debug!("Platform: {:?}", self.platform);
        tracing::info!("Downloading {} from: {}", component.name(), url);
        tracing::debug!("Full URL ({} chars): {}", url.len(), url);

        // Set platform-appropriate User-Agent
        let user_agent = match self.platform {
            Platform::LinuxX64 | Platform::LinuxArm64 => "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            Platform::MacOSX64 | Platform::MacOSArm64 => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            Platform::WindowsX64 | Platform::WindowsArm64 => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        };

        // MySQL downloads require specific headers to bypass their gateway
        let mut request = self.client.get(&url)
            .header("User-Agent", user_agent);

        // Add Referer header for MySQL downloads (required by dev.mysql.com gateway)
        if component == BinaryComponent::MySQL && url.contains("dev.mysql.com") {
            request = request.header("Referer", "https://dev.mysql.com/downloads/mysql/")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8");
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {}: {}", component.name(), e))?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "HTTP error {}: Failed to download {}\nURL: {}",
                status.as_u16(),
                component.name(),
                url
            ));
        }

        // Get final URL after redirects
        let final_url = response.url().clone();
        if final_url.as_str() != url {
            tracing::info!("Redirected: {} -> {}", url, final_url);
        }

        // Check content type
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(ct) = content_type.to_str() {
                if ct.contains("text/html") {
                    return Err(format!(
                        "Server returned HTML instead of binary. URL may be incorrect: {}",
                        final_url
                    ));
                }
            }
        }

        let total_bytes = response.content_length().unwrap_or(0);
        let version = self.get_component_version(&component);
        let file_path = dest_dir.join(format!(
            "{}-{}.{}",
            component.binary_name(),
            version,
            extension
        ));

        // Create parent directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        // Download using bytes() for simplicity
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to download bytes: {}", e))?;

        // Verify the file is valid by checking magic bytes
        if bytes.len() < 4 {
            return Err(format!(
                "Downloaded file is too small ({} bytes) to be a valid archive",
                bytes.len()
            ));
        }

        // Check if it's a ZIP file (starts with PK)
        let is_zip = bytes[0] == 0x50 && bytes[1] == 0x4B;
        // Check if it's gzip (starts with 0x1f 0x8b)
        let is_gzip = bytes[0] == 0x1f && bytes[1] == 0x8b;

        if extension == "zip" && !is_zip {
            return Err(format!(
                "Expected ZIP file but downloaded file doesn't have ZIP magic bytes. URL may have redirected to HTML page."
            ));
        }

        if (extension == "gz" || extension == "tar.gz") && !is_gzip {
            return Err(format!(
                "Expected gzip file but downloaded file doesn't have gzip magic bytes."
            ));
        }

        let downloaded_bytes = bytes.len() as u64;
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        // Verify checksum if available
        if let Some(expected_checksum) = self.get_expected_checksum(&component, &url) {
            let actual_checksum = self.calculate_checksum_from_bytes(&bytes)
                .map_err(|e| format!("Failed to calculate checksum: {}", e))?;

            if actual_checksum.to_lowercase() != expected_checksum.to_lowercase() {
                return Err(format!(
                    "Checksum verification failed for {}.\nExpected: {}\nActual: {}\n\nThe downloaded file may be corrupted or tampered with.",
                    component.name(),
                    expected_checksum,
                    actual_checksum
                ));
            }
            tracing::info!("Checksum verified for {}: {}", component.name(), actual_checksum);
        } else {
            tracing::warn!("No checksum configured for {} — integrity not verified", component.name());
        }

        let percent = if total_bytes > 0 {
            ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u8
        } else {
            100
        };

        progress_cb(DownloadProgress {
            step: DownloadStep::Downloading,
            percent,
            current_component: component.name().to_string(),
            component_display: component.display_name(),
            version: self.get_component_version(&component),
            total_components: total,
            downloaded_bytes,
            total_bytes,
        });

        Ok(file_path)
    }

    /// Calculate SHA256 checksum of a file
    fn calculate_checksum(&self, path: &Path) -> Result<String, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Calculate SHA256 checksum from bytes
    fn calculate_checksum_from_bytes(&self, bytes: &[u8]) -> Result<String, String> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Get the expected checksum for a component based on current platform
    fn get_expected_checksum(&self, component: &BinaryComponent, _url: &str) -> Option<String> {
        let config = get_config()?;
        let platform_key = self.platform.url_key();

        match component {
            BinaryComponent::Php | BinaryComponent::MySQL | BinaryComponent::MariaDB | BinaryComponent::Caddy => {
                let version_info = match component {
                    BinaryComponent::Caddy => config.binaries.caddy.versions.iter(),
                    BinaryComponent::Php => config.binaries.php.versions.iter(),
                    BinaryComponent::MySQL => config.binaries.mysql.versions.iter(),
                    BinaryComponent::MariaDB => {
                        match &config.binaries.mariadb {
                            Some(mc) => mc.versions.iter(),
                            None => return None,
                        }
                    }
                    _ => return None,
                };

                // Determine which version ID to look for based on package selection
                let target_id = if let Some(ref selection) = self.package_selection {
                    match component {
                        BinaryComponent::Php => Some(selection.php.as_str()),
                        BinaryComponent::MySQL => Some(selection.mysql.as_str()),
                        BinaryComponent::MariaDB => Some(selection.mariadb.as_str()),
                        _ => None,
                    }
                } else {
                    None
                };

                for version in version_info {
                    if target_id.is_some() && version.id == target_id.unwrap() {
                        // Use the package selection
                        return match platform_key.as_str() {
                            "windowsX64" => version.checksums.windows_x64.clone(),
                            "windowsArm64" => version.checksums.windows_arm64.clone(),
                            "linuxX64" => version.checksums.linux_x64.clone(),
                            "linuxArm64" => version.checksums.linux_arm64.clone(),
                            "macOSX64" => version.checksums.macos_x64.clone(),
                            "macOSArm64" => version.checksums.macos_arm64.clone(),
                            _ => None,
                        };
                    } else if target_id.is_none() && version.selected {
                        // Fall back to selected flag
                        return match platform_key.as_str() {
                            "windowsX64" => version.checksums.windows_x64.clone(),
                            "windowsArm64" => version.checksums.windows_arm64.clone(),
                            "linuxX64" => version.checksums.linux_x64.clone(),
                            "linuxArm64" => version.checksums.linux_arm64.clone(),
                            "macOSX64" => version.checksums.macos_x64.clone(),
                            "macOSArm64" => version.checksums.macos_arm64.clone(),
                            _ => None,
                        };
                    }
                }
                None
            }
            BinaryComponent::PhpMyAdmin => {
                let version = config.binaries.phpmyadmin.versions.iter()
                    .find(|v| v.selected)?;
                version.checksum.clone()
            }
        }
    }

    /// Extract a ZIP archive
    fn extract_zip(&self, archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
        let file = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to get file: {}", e))?;
            let outpath = dest_dir.join(file.enclosed_name().ok_or("Invalid path")?);

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create file: {}", e))?;
                io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to write file: {}", e))?;

                // Set executable permission on Unix for binary files
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if is_executable(&file.name()) {
                        let mut perms = fs::metadata(&outpath)
                            .map_err(|e| format!("Failed to get metadata: {}", e))?
                            .permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&outpath, perms)
                            .map_err(|e| format!("Failed to set permissions: {}", e))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract a tar.gz archive
    fn extract_tar_gz(&self, archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
        use flate2::read::GzDecoder;

        let file = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        archive
            .unpack(dest_dir)
            .map_err(|e| format!("Failed to extract {}: {}", archive_path.display(), e))?;

        Self::set_binary_permissions(dest_dir);
        Ok(())
    }

    fn extract_tar_xz(&self, archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
        use xz2::read::XzDecoder;

        let file = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
        let decoder = XzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        archive
            .unpack(dest_dir)
            .map_err(|e| format!("Failed to extract {}: {}", archive_path.display(), e))?;

        Self::set_binary_permissions(dest_dir);
        Ok(())
    }

    fn set_binary_permissions(dest_dir: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let binary_paths = [
                dest_dir.join("caddy"),
                dest_dir.join("php-fpm"),
                dest_dir.join("php-cgi"),
                dest_dir.join("buildroot/bin/php-fpm"),
                dest_dir.join("buildroot/bin/php"),
                dest_dir.join("mysql/bin/mysqld"),
                dest_dir.join("bin/mysqld"),
            ];

            for path in &binary_paths {
                if path.exists() {
                    if let Ok(metadata) = fs::metadata(path) {
                        let mut perms = metadata.permissions();
                        let mode = perms.mode();
                        if mode & 0o111 == 0 {
                            perms.set_mode(mode | 0o755);
                            let _ = fs::set_permissions(path, perms);
                        }
                    }
                }
            }

            if let Ok(entries) = fs::read_dir(dest_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if dir_name == "bin" || dir_name == "sbin" {
                                if let Ok(bin_entries) = fs::read_dir(&path) {
                                    for bin_entry in bin_entries.flatten() {
                                        let bin_path = bin_entry.path();
                                        if bin_path.is_file() {
                                            if let Ok(metadata) = fs::metadata(&bin_path) {
                                                let mut perms = metadata.permissions();
                                                perms.set_mode(0o755);
                                                let _ = fs::set_permissions(&bin_path, perms);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Download and install all runtime binaries
    pub async fn download_all(
        &self,
        progress_cb: ProgressCallback,
    ) -> Result<Vec<PathBuf>, String> {
        self.download_all_impl(progress_cb, &[]).await
    }

    /// Download and install runtime binaries with option to skip existing components
    pub async fn download_all_with_skip(
        &self,
        progress_cb: ProgressCallback,
        skip_list: &[&str],
    ) -> Result<Vec<PathBuf>, String> {
        self.download_all_impl(progress_cb, skip_list).await
    }

    async fn download_all_impl(
        &self,
        progress_cb: ProgressCallback,
        skip_list: &[&str],
    ) -> Result<Vec<PathBuf>, String> {
        // On Linux, use MariaDB instead of MySQL
        let db_component = match self.platform {
            Platform::LinuxX64 | Platform::LinuxArm64 => BinaryComponent::MariaDB,
            _ => BinaryComponent::MySQL,
        };

        let components = [
            BinaryComponent::Caddy,
            BinaryComponent::Php,
            db_component,
            BinaryComponent::PhpMyAdmin,
        ];
        let total = components.len() as u8;

        // Create temp directory for downloads
        let temp_dir = std::env::temp_dir().join("campp-download");
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let mut downloaded_files = Vec::new();

        for (i, component) in components.iter().enumerate() {
            let component_name = component.binary_name();

            // Skip if component is in skip list
            if skip_list.contains(&component_name) {
                tracing::info!("Skipping {} (already installed)", component.name());
                continue;
            }

            let current = (i + 1) as u8;

            // Download
            let downloaded_path = self
                .download_component(*component, &temp_dir, &progress_cb, current, total)
                .await?;

            // Checksum is verified during download_component
            progress_cb(DownloadProgress {
                step: DownloadStep::Extracting,
                percent: 0,
                current_component: component.name().to_string(),
                component_display: component.display_name(),
                version: self.get_component_version(&component),
                total_components: total,
                downloaded_bytes: 0,
                total_bytes: 0,
            });

            let runtime_dir = self.get_runtime_dir()?;
            fs::create_dir_all(&runtime_dir)
                .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

            // Determine extraction method based on file extension
            let extension = downloaded_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            let is_tar_gz = downloaded_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".tar.gz"))
                .unwrap_or(false);

            let is_tar_xz = downloaded_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".tar.xz"))
                .unwrap_or(false);

            if is_tar_gz || extension == "gz" {
                self.extract_tar_gz(&downloaded_path, &runtime_dir)?;
            } else if is_tar_xz || extension == "xz" {
                self.extract_tar_xz(&downloaded_path, &runtime_dir)?;
            } else if extension == "zip" {
                self.extract_zip(&downloaded_path, &runtime_dir)?;
            } else if extension.is_empty() {
                // Bare binary - copy directly to runtime directory
                let binary_name = downloaded_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("Invalid binary name")?;

                let dest_path = runtime_dir.join(binary_name);
                fs::copy(&downloaded_path, &dest_path)
                    .map_err(|e| format!("Failed to copy binary: {}", e))?;

                // Set executable permission on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&dest_path)
                        .map_err(|e| format!("Failed to get metadata: {}", e))?
                        .permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&dest_path, perms)
                        .map_err(|e| format!("Failed to set permissions: {}", e))?;
                }
            } else {
                return Err(format!("Unsupported archive format: {}", extension));
            }

            // Create marker file to indicate component was installed with version
            let version = self.get_component_version(&component);
            let marker_file = runtime_dir.join(format!("{}_installed.txt", component.binary_name()));
            fs::write(&marker_file, format!("version={}\ninstalled_at={:?}", version, std::time::SystemTime::now()))
                .map_err(|e| format!("Failed to create marker file: {}", e))?;

            downloaded_files.push(downloaded_path);
        }

        // Create all application directories (config, logs, mysql/data, projects)
        if let Ok(app_paths) = get_app_data_paths() {
            if let Err(e) = app_paths.ensure_directories() {
                tracing::warn!("Failed to create app directories: {}", e);
            }
        }

        progress_cb(DownloadProgress {
            step: DownloadStep::Complete,
            percent: 100,
            current_component: "All".to_string(),
            component_display: "All Components".to_string(),
            version: String::new(),
            total_components: total,
            downloaded_bytes: 0,
            total_bytes: 0,
        });

        // Keep temp files for user to access if needed
        // Uncomment to cleanup: let _ = fs::remove_dir_all(temp_dir);

        Ok(downloaded_files)
    }

    /// Get the runtime directory
    pub fn get_runtime_dir(&self) -> Result<PathBuf, String> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, use the installation folder (where the exe is located)
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Failed to get exe path: {}", e))?;
            let install_dir = exe_path.parent()
                .ok_or("Failed to get installation directory")?;
            Ok(install_dir.join("runtime"))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let data_dir = dirs::data_local_dir().ok_or("Failed to get data directory")?;
            Ok(data_dir.join("campp").join("runtime"))
        }
    }

    /// Check if runtime binaries are already installed
    pub fn is_installed(&self) -> bool {
        let runtime_dir = match self.get_runtime_dir() {
            Ok(dir) => dir,
            Err(_) => return false,
        };

        // Check for marker files created during simulation or actual binaries
        let caddy_marker = runtime_dir.join("caddy_installed.txt");
        let php_marker = runtime_dir.join("php_installed.txt");
        let mysql_marker = runtime_dir.join("mysql_installed.txt");
        let mariadb_marker = runtime_dir.join("mariadb_installed.txt");
        let phpmyadmin_marker = runtime_dir.join("phpmyadmin_installed.txt");

        // Also check for actual binaries
        #[cfg(target_os = "windows")]
        let bin_check = {
            let caddy_exe = runtime_dir.join("caddy").join("caddy.exe");
            let php_exe = runtime_dir.join("php").join("php.exe");
            let mysql_exe = runtime_dir.join("mysql").join("bin").join("mysqld.exe");
            caddy_exe.exists() || php_exe.exists() || mysql_exe.exists()
        };
        #[cfg(not(target_os = "windows"))]
        let bin_check = {
            let caddy_bin = runtime_dir.join("caddy");
            let php_fpm = runtime_dir.join("php-fpm");
            let php_cgi = runtime_dir.join("php-cgi");
            let mysql_bin = runtime_dir.join("mysql").join("bin").join("mysqld");
            let mariadbd_bin = runtime_dir.join("mariadb").join("bin").join("mariadbd");
            caddy_bin.exists() || php_fpm.exists() || php_cgi.exists()
                || mysql_bin.exists() || mariadbd_bin.exists()
        };

        caddy_marker.exists() || php_marker.exists() || mysql_marker.exists()
            || mariadb_marker.exists() || phpmyadmin_marker.exists() || bin_check
    }

    /// Check which components are already installed with their versions
    pub fn get_installed_components(&self) -> std::collections::HashMap<String, String> {
        let mut installed = std::collections::HashMap::new();
        let runtime_dir = match self.get_runtime_dir() {
            Ok(dir) => dir,
            Err(_) => return installed,
        };

        for component in ["caddy", "php", "mysql", "mariadb", "phpmyadmin"] {
            let marker_file = runtime_dir.join(format!("{}_installed.txt", component));
            if let Ok(content) = fs::read_to_string(&marker_file) {
                // Parse version from format: "version=1.2.3\ninstalled_at=..."
                for line in content.lines() {
                    if let Some(version) = line.strip_prefix("version=") {
                        installed.insert(component.to_string(), version.to_string());
                        break;
                    }
                }
            }
        }

        installed
    }

    /// Uninstall a specific component by removing its marker file and binary files
    pub fn uninstall_component(&self, component: &str) -> Result<(), String> {
        let valid_components = ["caddy", "php", "mysql", "mariadb", "phpmyadmin"];
        if !valid_components.contains(&component) {
            return Err(format!("Invalid component: {}", component));
        }

        let runtime_dir = self.get_runtime_dir()?;

        // Remove marker file
        let marker = runtime_dir.join(format!("{}_installed.txt", component));
        if marker.exists() {
            fs::remove_file(&marker)
                .map_err(|e| format!("Failed to remove marker file: {}", e))?;
        }

        // Remove binary files/directories
        match component {
            "caddy" => {
                Self::remove_entries(&runtime_dir, &["caddy"])?;
            }
            "php" => {
                // Versioned dirs (php-8.4.16-Win32-vs17-x64), buildroot, php/, standalone binaries
                let mut removed = Self::remove_entries(&runtime_dir, &["php"])?;
                removed |= Self::remove_versioned_dirs(&runtime_dir, "php-")?;
                // Static-php buildroot
                let buildroot = runtime_dir.join("buildroot");
                if buildroot.exists() {
                    fs::remove_dir_all(&buildroot)
                        .map_err(|e| format!("Failed to remove buildroot: {}", e))?;
                    removed = true;
                }
                // Standalone binaries
                for name in ["php-fpm", "php-cgi", "php-fpm.exe", "php-cgi.exe", "php.exe"] {
                    let p = runtime_dir.join(name);
                    if p.exists() {
                        let _ = fs::remove_file(&p);
                    }
                }
                if !removed {
                    // Also try php/ directory
                    let php_dir = runtime_dir.join("php");
                    if php_dir.exists() {
                        fs::remove_dir_all(&php_dir)
                            .map_err(|e| format!("Failed to remove php dir: {}", e))?;
                    }
                }
            }
            "mysql" => {
                let mut removed = Self::remove_entries(&runtime_dir, &["mysql", "mariadb"])?;
                removed |= Self::remove_versioned_dirs(&runtime_dir, "mysql-")?;
                removed |= Self::remove_versioned_dirs(&runtime_dir, "mariadb-")?;
                // Standalone binaries
                for name in ["mysqld", "mysqld.exe", "mariadbd"] {
                    let p = runtime_dir.join(name);
                    if p.exists() {
                        let _ = fs::remove_file(&p);
                    }
                }
                if !removed {
                    // Try mysql/ and mariadb/ directories
                    for dir_name in ["mysql", "mariadb"] {
                        let d = runtime_dir.join(dir_name);
                        if d.exists() {
                            fs::remove_dir_all(&d)
                                .map_err(|e| format!("Failed to remove {} dir: {}", dir_name, e))?;
                        }
                    }
                }
            }
            "phpmyadmin" => {
                Self::remove_versioned_dirs(&runtime_dir, "phpMyAdmin")?;
                // Also lowercase variants
                let mut entries_to_remove = Vec::new();
                if let Ok(entries) = fs::read_dir(&runtime_dir) {
                    for entry in entries.flatten() {
                        if let Ok(name) = entry.file_name().into_string() {
                            let lower = name.to_lowercase();
                            if lower.starts_with("phpmyadmin") && entry.path().is_dir() {
                                entries_to_remove.push(entry.path());
                            }
                        }
                    }
                }
                for path in entries_to_remove {
                    fs::remove_dir_all(&path)
                        .map_err(|e| format!("Failed to remove phpmyadmin dir: {}", e))?;
                }
                // Also phpmyadmin/ (lowercase, non-versioned)
                let pma_dir = runtime_dir.join("phpmyadmin");
                if pma_dir.exists() {
                    fs::remove_dir_all(&pma_dir)
                        .map_err(|e| format!("Failed to remove phpmyadmin dir: {}", e))?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Remove entries (files or directories) in runtime_dir that match exact names
    fn remove_entries(runtime_dir: &Path, names: &[&str]) -> Result<bool, String> {
        let mut removed = false;
        for name in names {
            let path = runtime_dir.join(name);
            if path.exists() {
                if path.is_dir() {
                    fs::remove_dir_all(&path)
                        .map_err(|e| format!("Failed to remove {}: {}", name, e))?;
                } else {
                    fs::remove_file(&path)
                        .map_err(|e| format!("Failed to remove {}: {}", name, e))?;
                }
                removed = true;
            }
            // Also check with .exe extension
            let exe_path = runtime_dir.join(format!("{}.exe", name));
            if exe_path.exists() {
                let _ = fs::remove_file(&exe_path);
                removed = true;
            }
        }
        Ok(removed)
    }

    /// Remove directories in runtime_dir whose names start with a given prefix
    fn remove_versioned_dirs(runtime_dir: &Path, prefix: &str) -> Result<bool, String> {
        let mut removed = false;
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with(prefix) && entry.path().is_dir() {
                        fs::remove_dir_all(&entry.path())
                            .map_err(|e| format!("Failed to remove {}: {}", name, e))?;
                        removed = true;
                    }
                }
            }
        }
        Ok(removed)
    }
}

impl Default for RuntimeDownloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a file is executable based on its name
#[cfg(unix)]
fn is_executable(name: &str) -> bool {
    name.ends_with("caddy")
        || name.ends_with("php")
        || name.ends_with("php-cgi")
        || name.ends_with("php-fpm")
        || name.ends_with("mysqld")
        || name.ends_with("mysql")
        || name.ends_with("mysqld")
        || name.contains("bin/")
}
