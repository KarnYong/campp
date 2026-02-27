pub mod generator;
pub mod ports;
pub mod settings;

pub use settings::{AppSettings, DEFAULT_PORTS};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub web_port: u16,
    pub php_port: u16,
    pub mysql_port: u16,
    pub project_root: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            web_port: 8080,
            php_port: 9000,
            mysql_port: 3307,
            project_root: dirs::home_dir()
                .unwrap_or_default()
                .join(".campp")
                .join("projects")
                .to_string_lossy()
                .to_string(),
        }
    }
}
