import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState, useEffect, useCallback } from "react";
import { ServiceMap, ServiceType, ServiceState } from "../types/services";
import { ServiceCard } from "./ServiceCard";
import { StatusBar } from "./StatusBar";

export function Dashboard() {
  const [services, setServices] = useState<Partial<ServiceMap>>({});

  // Get Caddy port from services
  const caddyPort = services[ServiceType.Caddy]?.port || 8080;
  const webServerUrl = `http://localhost:${caddyPort}`;
  const phpMyAdminUrl = `${webServerUrl}/phpmyadmin`;

  // Check if Caddy is running
  const isCaddyRunning = services[ServiceType.Caddy]?.state === ServiceState.Running;

  const refreshStatuses = useCallback(async () => {
    try {
      const statuses = await invoke<ServiceMap>("get_all_statuses");
      setServices(statuses);
    } catch (error) {
      console.error("Failed to get service statuses:", error);
    }
  }, []);

  useEffect(() => {
    refreshStatuses();
    const interval = setInterval(refreshStatuses, 2000);
    return () => clearInterval(interval);
  }, [refreshStatuses]);

  const startService = async (serviceType: ServiceType) => {
    try {
      await invoke("start_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to start ${serviceType}:`, error);
    }
  };

  const stopService = async (serviceType: ServiceType) => {
    try {
      await invoke("stop_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to stop ${serviceType}:`, error);
    }
  };

  const restartService = async (serviceType: ServiceType) => {
    try {
      await invoke("restart_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to restart ${serviceType}:`, error);
    }
  };

  const openWebServer = async () => {
    try {
      await openUrl(webServerUrl);
    } catch (error) {
      console.error("Failed to open web server URL:", error);
    }
  };

  const openPhpMyAdmin = async () => {
    try {
      await openUrl(phpMyAdminUrl);
    } catch (error) {
      console.error("Failed to open phpMyAdmin URL:", error);
    }
  };

  return (
    <div className="dashboard" data-testid="dashboard">
      <header className="dashboard-header">
        <div className="header-content">
          <div className="header-title">
            <h1>CAMPP - Local Web Development Stack</h1>
            <p className="dashboard-subtitle">
              Control panel for Caddy, PHP-FPM, and MariaDB
            </p>
          </div>
          <div className="quick-actions">
            <button
              className="btn-quick-action"
              onClick={openWebServer}
              disabled={!isCaddyRunning}
              title={isCaddyRunning ? `Open ${webServerUrl}` : "Start Caddy to enable"}
            >
              <span className="btn-icon">🌐</span>
              Open Site
            </button>
            <button
              className="btn-quick-action"
              onClick={openPhpMyAdmin}
              disabled={!isCaddyRunning}
              title={isCaddyRunning ? `Open ${phpMyAdminUrl}` : "Start Caddy to enable"}
            >
              <span className="btn-icon">🗄️</span>
              phpMyAdmin
            </button>
          </div>
        </div>
      </header>

      <main className="dashboard-main">
        <div className="service-grid">
          {[ServiceType.Caddy, ServiceType.PhpFpm, ServiceType.MariaDB].map((serviceType) => {
            const service = services[serviceType];
            if (!service) return null;
            return (
              <ServiceCard
                key={serviceType}
                serviceType={serviceType}
                state={service.state as ServiceState}
                port={service.port}
                error={service.error_message}
                onStart={() => startService(serviceType)}
                onStop={() => stopService(serviceType)}
                onRestart={() => restartService(serviceType)}
              />
            );
          })}
        </div>
      </main>

      <StatusBar services={services} data-testid="status-bar" />
    </div>
  );
}
