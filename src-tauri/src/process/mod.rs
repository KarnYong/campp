pub mod manager;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Caddy,
    PhpFpm,
    MariaDB,
}

impl ServiceType {
    pub fn default_port(&self) -> u16 {
        match self {
            ServiceType::Caddy => 8080,
            ServiceType::PhpFpm => 9000,
            ServiceType::MariaDB => 3307,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ServiceType::Caddy => "Caddy",
            ServiceType::PhpFpm => "PHP-FPM 8.3",
            ServiceType::MariaDB => "MariaDB",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ServiceType::Caddy => "Web Server",
            ServiceType::PhpFpm => "PHP Runtime",
            ServiceType::MariaDB => "Database Server",
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            ServiceType::Caddy => "caddy",
            ServiceType::PhpFpm => "php-cgi",
            ServiceType::MariaDB => "mysqld",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    #[serde(rename = "error")]
    Error(String),
}

impl ServiceState {
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceState::Running)
    }

    pub fn is_transitioning(&self) -> bool {
        matches!(self, ServiceState::Starting | ServiceState::Stopping)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub service_type: ServiceType,
    pub state: ServiceState,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl ServiceInfo {
    pub fn new(service_type: ServiceType) -> Self {
        Self {
            port: service_type.default_port(),
            service_type,
            state: ServiceState::Stopped,
            error_message: None,
        }
    }
}

pub type ServiceMap = std::collections::HashMap<ServiceType, ServiceInfo>;
