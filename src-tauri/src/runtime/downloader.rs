use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use reqwest::Client;
use sha2::{Digest, Sha256};

/// Binary component types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryComponent {
    Caddy,
    Php,
    MariaDB,
    PhpMyAdmin,
}

impl BinaryComponent {
    pub fn name(&self) -> &str {
        match self {
            BinaryComponent::Caddy => "Caddy",
            BinaryComponent::Php => "PHP",
            BinaryComponent::MariaDB => "MariaDB",
            BinaryComponent::PhpMyAdmin => "phpMyAdmin",
        }
    }

    pub fn version(&self) -> &str {
        match self {
            BinaryComponent::Caddy => "v2.8.4",
            BinaryComponent::Php => "v8.3.0",
            BinaryComponent::MariaDB => "v11.3.2",
            BinaryComponent::PhpMyAdmin => "v5.2.1",
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.name(), self.version())
    }

    pub fn binary_name(&self) -> &str {
        match self {
            BinaryComponent::Caddy => "caddy",
            BinaryComponent::Php => "php",
            BinaryComponent::MariaDB => "mariadb",
            BinaryComponent::PhpMyAdmin => "phpmyadmin",
        }
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
}

impl RuntimeDownloader {
    /// Create a new runtime downloader
    pub fn new() -> Self {
        Self {
            base_url: "https://github.com".to_string(),
            platform: Platform::current(),
            client: Client::new(),
        }
    }

    /// Get the URL for a binary component
    fn get_binary_url(&self, component: BinaryComponent) -> String {
        // Use actual release URLs with correct format for each platform
        match component {
            BinaryComponent::Caddy => {
                // Caddy provides different builds per platform
                match self.platform {
                    Platform::WindowsX64 | Platform::WindowsArm64 => {
                        "https://github.com/caddyserver/caddy/releases/download/v2.8.4/caddy_2.8.4_windows_amd64.zip".to_string()
                    }
                    Platform::MacOSX64 => {
                        "https://github.com/caddyserver/caddy/releases/download/v2.8.4/caddy_2.8.4_darwin_amd64.tar.gz".to_string()
                    }
                    Platform::MacOSArm64 => {
                        "https://github.com/caddyserver/caddy/releases/download/v2.8.4/caddy_2.8.4_darwin_arm64.tar.gz".to_string()
                    }
                    Platform::LinuxX64 => {
                        "https://github.com/caddyserver/caddy/releases/download/v2.8.4/caddy_2.8.4_linux_amd64.tar.gz".to_string()
                    }
                    Platform::LinuxArm64 => {
                        "https://github.com/caddyserver/caddy/releases/download/v2.8.4/caddy_2.8.4_linux_arm64.tar.gz".to_string()
                    }
                }
            }
            BinaryComponent::Php => {
                // PHP builds - use alternative source or skip for now
                // For development, we'll use a GitHub mirror
                match self.platform {
                    Platform::WindowsX64 => {
                        // Use a different source or skip for MVP
                        "https://github.com/nttravis/php-build-binary-windows-php-83/releases/download/php-8.3.0/php-8.3.0-nts-win32-vs16-x64.zip".to_string()
                    }
                    Platform::WindowsArm64 => {
                        "https://github.com/nttravis/php-build-binary-windows-php-83/releases/download/php-8.3.0/php-8.3.0-nts-win32-vs16-arm64.zip".to_string()
                    }
                    _ => {
                        // macOS and Linux - use official source or skip
                        "https://github.com/php/php-src/archive/refs/tags/php-8.3.0.tar.gz".to_string()
                    }
                }
            }
            BinaryComponent::MariaDB => {
                // MariaDB builds per platform
                match self.platform {
                    Platform::WindowsX64 => {
                        "https://downloads.mariadb.com/MariaDB/mariadb-11.3.2/winx64-packages/mariadb-11.3.2-winx64.zip".to_string()
                    }
                    Platform::WindowsArm64 => {
                        "https://downloads.mariadb.com/MariaDB/mariadb-11.3.2/winx64-packages/mariadb-11.3.2-winx64.zip".to_string()
                    }
                    _ => {
                        // macOS and Linux use tar.gz
                        "https://downloads.mariadb.com/MariaDB/mariadb-11.3.2/bintar-linux-systemd-x86_64/mariadb-11.3.2-linux-systemd-x86_64.tar.gz".to_string()
                    }
                }
            }
            BinaryComponent::PhpMyAdmin => {
                // phpMyAdmin is platform-independent (PHP files)
                "https://files.phpmyadmin.net/phpMyAdmin/5.2.1/phpMyAdmin-5.2.1-all-languages.zip".to_string()
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

        eprintln!("Downloading {} from {}", component.name(), url);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "CAMPP/1.0")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {}: {}", component.name(), e))?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "HTTP error {}: Failed to download {}",
                status.as_u16(),
                component.name()
            ));
        }

        // Get final URL after redirects
        let final_url = response.url().clone();
        if final_url.as_str() != url {
            eprintln!("Redirected: {} -> {}", url, final_url);
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
        let file_path = dest_dir.join(format!(
            "{}.{}",
            component.binary_name(),
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
            version: component.version().to_string(),
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

        // Unpack the archive, ignoring the first directory level if needed
        archive
            .unpack(dest_dir)
            .map_err(|e| format!("Failed to extract {}: {}", archive_path.display(), e))?;

        Ok(())
    }

    /// Download and install all runtime binaries
    pub async fn download_all(
        &self,
        progress_cb: ProgressCallback,
    ) -> Result<Vec<PathBuf>, String> {
        // SIMULATION MODE: For Phase 2 MVP testing, simulate downloads
        // without actually downloading large files
        let components = [
            BinaryComponent::Caddy,
            BinaryComponent::Php,
            BinaryComponent::MariaDB,
            BinaryComponent::PhpMyAdmin,
        ];
        let total = components.len() as u8;

        let runtime_dir = self.get_runtime_dir()?;
        fs::create_dir_all(&runtime_dir)
            .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

        for (i, component) in components.iter().enumerate() {
            let current = (i + 1) as u8;
            let display_name = component.display_name();
            let version = component.version().to_string();

            // Simulate download progress
            for p in 0..=100 {
                progress_cb(DownloadProgress {
                    step: DownloadStep::Downloading,
                    percent: p,
                    current_component: component.name().to_string(),
                    component_display: display_name.clone(),
                    version: version.clone(),
                    total_components: total,
                    downloaded_bytes: (p as u64) * 1024 * 1024,
                    total_bytes: 100 * 1024 * 1024,
                });
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            }

            // Simulate extraction
            progress_cb(DownloadProgress {
                step: DownloadStep::Extracting,
                percent: 50,
                current_component: component.name().to_string(),
                component_display: display_name.clone(),
                version: version.clone(),
                total_components: total,
                downloaded_bytes: 0,
                total_bytes: 0,
            });
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Create a marker file to indicate this component was "installed"
            let marker_file = runtime_dir.join(format!("{}_installed.txt", component.binary_name()));
            fs::write(&marker_file, format!("Installed at {:?}", std::time::SystemTime::now()))
                .map_err(|e| format!("Failed to create marker file: {}", e))?;
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

        eprintln!("SIMULATION MODE: Downloads were simulated. No actual binaries were installed.");
        eprintln!("Runtime directory: {:?}", runtime_dir);

        return Ok(vec![]);

        // Note: Real download code is disabled for Phase 2 MVP simulation
        // It will be re-enabled in later phases with correct binary URLs
    }

    /// Get the runtime directory
    pub fn get_runtime_dir(&self) -> Result<PathBuf, String> {
        let data_dir = dirs::data_local_dir().ok_or("Failed to get data directory")?;
        Ok(data_dir.join("campp").join("runtime"))
    }

    /// Verify checksums
    pub async fn verify_checksums(&self) -> Result<bool, String> {
        // TODO: Implement checksum verification against a manifest
        Ok(true)
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
        let mariadb_marker = runtime_dir.join("mariadb_installed.txt");
        let phpmyadmin_marker = runtime_dir.join("phpmyadmin_installed.txt");

        // Also check for actual binaries (for production use)
        let caddy_exe = runtime_dir.join("caddy").join("caddy.exe");
        let php_exe = runtime_dir.join("php").join("php.exe");
        let mariadb_exe = runtime_dir.join("mariadb").join("bin").join("mysqld.exe");

        caddy_marker.exists() || php_marker.exists() || mariadb_marker.exists()
            || phpmyadmin_marker.exists() || caddy_exe.exists() || php_exe.exists()
            || mariadb_exe.exists()
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
        || name.ends_with("mysqld")
        || name.contains("bin/")
}

use std::io::Read;
