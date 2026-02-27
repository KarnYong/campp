use std::collections::HashMap;

/// Runtime binary download system
/// TODO: Implement in Phase 2

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeManifest {
    pub caddy_url: String,
    pub php_url: String,
    pub mariadb_url: String,
    pub phpmyadmin_url: String,
    pub checksums: HashMap<String, String>,
}

pub struct RuntimeDownloader {
    pub base_url: String,
    pub platform: String,
}

pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadProgress {
    pub step: DownloadStep,
    pub percent: u8,
    pub current_component: String,
    pub total_components: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStep {
    Downloading,
    Extracting,
    Installing,
    Complete,
}

impl RuntimeDownloader {
    pub async fn download_all(&self, _progress_cb: ProgressCallback) -> Result<(), String> {
        // TODO: Implement binary download in Phase 2
        Ok(())
    }

    pub async fn verify_checksums(&self) -> Result<bool, String> {
        // TODO: Implement checksum verification in Phase 2
        Ok(true)
    }
}
