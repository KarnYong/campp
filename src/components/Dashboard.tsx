import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useState, useEffect, useCallback } from "react";
import { ServiceMap, ServiceType, ServiceState } from "../types/services";
import { ServiceCard } from "./ServiceCard";
import { StatusBar } from "./StatusBar";
import { SettingsPanel } from "./SettingsPanel";

export function Dashboard() {
  const [services, setServices] = useState<Partial<ServiceMap>>({});
  const [showSettings, setShowSettings] = useState(false);
  const [projectRoot, setProjectRoot] = useState<string>("");
  const [installDir, setInstallDir] = useState<string>("");
  const [installedVersions, setInstalledVersions] = useState<Record<string, string>>({});

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

  useEffect(() => {
    const loadProjectRoot = async () => {
      try {
        const settings = await invoke<{ project_root: string }>("get_settings");
        setProjectRoot(settings.project_root);
      } catch (error) {
        console.error("Failed to load project root:", error);
      }
    };
    loadProjectRoot();

    // Load install directory (on Windows, this is where the exe is located)
    const loadInstallDir = async () => {
      try {
        const dir = await invoke<string>("get_install_dir");
        console.log("get_install_dir returned:", dir);
        setInstallDir(dir);
      } catch (error) {
        console.error("Failed to load install directory:", error);
      }
    };
    loadInstallDir();

    // Load installed versions
    const loadInstalledVersions = async () => {
      try {
        const versions = await invoke<Record<string, string>>("get_installed_versions");
        setInstalledVersions(versions);
      } catch (error) {
        console.error("Failed to load installed versions:", error);
      }
    };
    loadInstalledVersions();
  }, []);

  const startService = async (serviceType: ServiceType) => {
    try {
      await invoke("start_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to start ${serviceType}:`, error);
      alert(`Failed to start ${serviceType}:\n${error}`);
      await refreshStatuses();
    }
  };

  const stopService = async (serviceType: ServiceType) => {
    try {
      await invoke("stop_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to stop ${serviceType}:`, error);
      alert(`Failed to stop ${serviceType}:\n${error}`);
      await refreshStatuses();
    }
  };

  const restartService = async (serviceType: ServiceType) => {
    try {
      await invoke("restart_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to restart ${serviceType}:`, error);
      alert(`Failed to restart ${serviceType}:\n${error}`);
      await refreshStatuses();
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

  const openProjectRoot = async () => {
    try {
      const pathToOpen = installDir || projectRoot;
      if (pathToOpen) {
        await revealItemInDir(pathToOpen);
      }
    } catch (error) {
      console.error("Failed to open directory:", error);
    }
  };

  return (
    <>
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          flexDirection: "column",
          backgroundColor: "var(--bg-app)",
          color: "var(--text-primary)",
        }}
        data-testid="dashboard"
      >
        {/* Header */}
        <header
          style={{
            backgroundColor: "var(--bg-card)",
            borderBottom: "1px solid var(--border-color)",
            padding: "0.75rem 1.5rem",
          }}
        >
          <div style={{ maxWidth: "80rem", margin: "0 auto" }}>
            {/* Title Section */}
            <div>
              <h1
                style={{
                  fontSize: "1.25rem",
                  fontWeight: 600,
                  margin: 0,
                  color: "var(--text-primary)",
                }}
              >
                CAMPP = Caddy + MySQL + PHP
              </h1>
              <p
                style={{
                  fontSize: "0.875rem",
                  color: "var(--text-secondary)",
                  marginTop: "0.25rem",
                  marginBottom: 0,
                }}
              >
                Development environment for apps using MySQL (PHP included)
              </p>
            </div>

            {/* Quick Actions */}
            <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem" }}>
              <button
                className="btn-quick-action"
                onClick={openWebServer}
                disabled={!isCaddyRunning}
                title={isCaddyRunning ? `Open ${webServerUrl}` : "Start Caddy to enable"}
              >
                <span style={{ fontSize: "1rem" }}>🌐</span>
                Open Site
              </button>
              <button
                className="btn-quick-action"
                onClick={openProjectRoot}
                disabled={!installDir && !projectRoot}
                title={installDir || projectRoot ? `Open ${installDir || projectRoot}` : "Directory not set"}
              >
                <span style={{ fontSize: "1rem" }}>📁</span>
                Projects
              </button>
              <button
                className="btn-quick-action"
                onClick={openPhpMyAdmin}
                disabled={!isCaddyRunning}
                title={isCaddyRunning ? `Open ${phpMyAdminUrl}` : "Start Caddy to enable"}
              >
                <span style={{ fontSize: "1rem" }}>🗄️</span>
                phpMyAdmin
              </button>
              <button
                className="btn-quick-action"
                onClick={() => setShowSettings(true)}
                title="Open Settings"
              >
                <span style={{ fontSize: "1rem" }}>⚙️</span>
                Settings
              </button>
              <button
                className="btn-quick-action"
                onClick={async () => {
                  try {
                    await invoke("open_manual");
                  } catch (err) {
                    console.error("Failed to open manual:", err);
                  }
                }}
                title="Read User Manual"
              >
                <span style={{ fontSize: "1rem" }}>?</span>
                Help
              </button>
            </div>
          </div>
        </header>

        {/* Installed Versions Bar */}
        {Object.keys(installedVersions).length > 0 && (
          <div
            style={{
              backgroundColor: "var(--bg-card-secondary)",
              borderBottom: "1px solid var(--border-color)",
              padding: "0.5rem 1rem",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.75rem",
                flexWrap: "wrap",
                maxWidth: "96rem",
                margin: "0 auto",
              }}
            >
              <span
                style={{
                  fontSize: "0.75rem",
                  fontWeight: 500,
                  color: "var(--text-secondary)",
                  textTransform: "uppercase",
                  letterSpacing: "0.05em",
                }}
              >
                Installed:
              </span>
              {installedVersions.caddy && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    padding: "0.25rem 0.625rem",
                    backgroundColor: "var(--bg-card)",
                    borderRadius: "0.25rem",
                    fontSize: "0.75rem",
                    fontWeight: 500,
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-color)",
                  }}
                >
                  Caddy {installedVersions.caddy}
                </span>
              )}
              {installedVersions.php && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    padding: "0.25rem 0.625rem",
                    backgroundColor: "var(--bg-card)",
                    borderRadius: "0.25rem",
                    fontSize: "0.75rem",
                    fontWeight: 500,
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-color)",
                  }}
                >
                  PHP {installedVersions.php}
                </span>
              )}
              {installedVersions.mysql && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    padding: "0.25rem 0.625rem",
                    backgroundColor: "var(--bg-card)",
                    borderRadius: "0.25rem",
                    fontSize: "0.75rem",
                    fontWeight: 500,
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-color)",
                  }}
                >
                  MySQL {installedVersions.mysql}
                </span>
              )}
              {installedVersions.phpmyadmin && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    padding: "0.25rem 0.625rem",
                    backgroundColor: "var(--bg-card)",
                    borderRadius: "0.25rem",
                    fontSize: "0.75rem",
                    fontWeight: 500,
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-color)",
                  }}
                >
                  phpMyAdmin {installedVersions.phpmyadmin}
                </span>
              )}
            </div>
          </div>
        )}

        {/* Main Content - Service Grid */}
        <main
          style={{
            flex: 1,
            padding: "1rem 1.5rem",
          }}
        >
          <div
            className="service-grid-responsive"
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(3, 1fr)",
              gap: "1rem",
              maxWidth: "80rem",
              margin: "0 auto",
            }}
          >
            {[ServiceType.Caddy, ServiceType.PhpFpm, ServiceType.MySQL].map((serviceType) => {
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

        {/* Status Bar */}
        <StatusBar services={services} data-testid="status-bar" />

        {/* Settings Panel */}
        {showSettings && (
          <SettingsPanel
            onClose={() => setShowSettings(false)}
            onSettingsChanged={refreshStatuses}
          />
        )}
      </div>
    </>
  );
}
