use serde::{Deserialize, Serialize};

pub const DEFAULT_PORTS: Ports = Ports {
    web: 8080,
    php: 9000,
    mysql: 3307,
};

#[derive(Debug, Clone, Copy)]
pub struct Ports {
    pub web: u16,
    pub php: u16,
    pub mysql: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub web_port: u16,
    pub php_port: u16,
    pub mysql_port: u16,
    pub project_root: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            web_port: DEFAULT_PORTS.web,
            php_port: DEFAULT_PORTS.php,
            mysql_port: DEFAULT_PORTS.mysql,
            project_root: dirs::home_dir()
                .unwrap_or_default()
                .join(".campp")
                .join("projects")
                .to_string_lossy()
                .to_string(),
        }
    }
}

pub fn load_settings() -> AppSettings {
    // TODO: Implement loading from config file in Phase 4
    AppSettings::default()
}

pub fn save_settings(_settings: &AppSettings) -> Result<(), String> {
    // TODO: Implement saving to config file in Phase 4
    Ok(())
}
