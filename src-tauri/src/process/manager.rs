use super::{ServiceInfo, ServiceMap, ServiceState, ServiceType};
use std::collections::HashMap;
use std::process::Child;

pub struct ServiceProcess {
    pub name: ServiceType,
    pub child: Option<Child>,
    pub state: ServiceState,
    pub port: u16,
}

pub struct ProcessManager {
    services: HashMap<ServiceType, ServiceProcess>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut services = HashMap::new();

        for service_type in [ServiceType::Caddy, ServiceType::PhpFpm, ServiceType::MariaDB] {
            services.insert(
                service_type,
                ServiceProcess {
                    name: service_type,
                    child: None,
                    state: ServiceState::Stopped,
                    port: service_type.default_port(),
                },
            );
        }

        Self { services }
    }

    pub fn start(&mut self, service: ServiceType) -> Result<(), String> {
        let service_process = self
            .services
            .get_mut(&service)
            .ok_or_else(|| format!("Service {:?} not found", service))?;

        if service_process.state.is_running() {
            return Ok(());
        }

        service_process.state = ServiceState::Starting;

        // TODO: Implement actual process spawning in Phase 3
        // For now, just update state to Running
        service_process.state = ServiceState::Running;

        Ok(())
    }

    pub fn stop(&mut self, service: ServiceType) -> Result<(), String> {
        let service_process = self
            .services
            .get_mut(&service)
            .ok_or_else(|| format!("Service {:?} not found", service))?;

        if !service_process.state.is_running() {
            return Ok(());
        }

        service_process.state = ServiceState::Stopping;

        // TODO: Implement actual process termination in Phase 3
        // For now, just update state to Stopped
        service_process.state = ServiceState::Stopped;

        Ok(())
    }

    pub fn restart(&mut self, service: ServiceType) -> Result<(), String> {
        self.stop(service)?;
        self.start(service)?;
        Ok(())
    }

    pub fn status(&self, service: ServiceType) -> ServiceState {
        self.services
            .get(&service)
            .map(|s| s.state.clone())
            .unwrap_or(ServiceState::Stopped)
    }

    pub fn get_all_statuses(&self) -> ServiceMap {
        self.services
            .iter()
            .map(|(ty, proc)| {
                (
                    *ty,
                    ServiceInfo {
                        service_type: *ty,
                        state: proc.state.clone(),
                        port: proc.port,
                        error_message: None,
                    },
                )
            })
            .collect()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
