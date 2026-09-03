import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useState, useEffect, useCallback } from "react";
import { ServiceMap, ServiceType, ServiceState, getDatabaseDisplayName } from "../types/services";
import { ServiceCard } from "./ServiceCard";
import { StatusBar } from "./StatusBar";
import { SettingsPanel } from "./SettingsPanel";
import { DebugMenu } from "./DebugMenu";
import { Icon } from "./icons";
import { detectPlatform } from "../utils/platform";
import logoUrl from "../assets/melivecode-logo.png";

export function Dashboard() {
  const [services, setServices] = useState<Partial<ServiceMap>>({});
  // Which action is in flight per service. start_service/stop_service block
  // until the backend finishes (holding the process lock, so the poll can't
  // observe the transient Starting/Stopping state) — track it here so the card
  // can show a spinner immediately on click.
  const [busy, setBusy] = useState<Partial<Record<ServiceType, "start" | "stop" | "restart">>>({});
  const [showSettings, setShowSettings] = useState(false);
  const [showDebugMenu, setShowDebugMenu] = useState(false);
  const [projectRoot, setProjectRoot] = useState<string>("");
  const [installDir, setInstallDir] = useState<string>("");
  const [installedVersions, setInstalledVersions] = useState<Record<string, string>>({});
  const [listenAllInterfaces, setListenAllInterfaces] = useState(false);
  const [ngrokUrl, setNgrokUrl] = useState<string | null>(null);
  const [showNgrokTokenModal, setShowNgrokTokenModal] = useState(false);
  const [ngrokTokenInput, setNgrokTokenInput] = useState("");
  const [ngrokTokenSaving, setNgrokTokenSaving] = useState(false);

  const currentPlatform = detectPlatform();
  const dbName = getDatabaseDisplayName(currentPlatform);

  // Get Caddy port from services
  const caddyPort = services[ServiceType.Caddy]?.port || 8080;
  const webServerUrl = `http://${listenAllInterfaces ? "localhost" : "127.0.0.1"}:${caddyPort}`;
  const phpMyAdminUrl = `${webServerUrl}/phpmyadmin`;
  const adminerUrl = `${webServerUrl}/adminer/`;

  // Check if Caddy is running
  const isCaddyRunning = services[ServiceType.Caddy]?.state === ServiceState.Running;

  // Settings (ports, passwords, project root) can't be changed safely while any
  // service is up or transitioning — gate the Settings button until all stopped.
  const anyServiceActive = Object.values(services).some(
    (s) =>
      s?.state === ServiceState.Running ||
      s?.state === ServiceState.Starting ||
      s?.state === ServiceState.Stopping
  );

  const refreshStatuses = useCallback(async () => {
    let currentStatuses: Partial<ServiceMap> = {};
    try {
      const statuses = await invoke<ServiceMap>("get_all_statuses");
      setServices(statuses);
      currentStatuses = statuses;
    } catch (error) {
      console.error("Failed to get service statuses:", error);
    }
    try {
      const s = await invoke<{ listen_all_interfaces?: boolean }>("get_settings");
      setListenAllInterfaces(s.listen_all_interfaces || false);
    } catch (_) {}
    // Poll ngrok public URL if running
    try {
      const ngrokRunning = currentStatuses[ServiceType.Ngrok]?.state === "running";
      if (ngrokRunning) {
        const url = await invoke<string | null>("get_ngrok_public_url");
        setNgrokUrl(url);
      } else {
        setNgrokUrl(null);
      }
    } catch (_) { setNgrokUrl(null); }
  }, []);

  const loadInstalledVersions = useCallback(async () => {
    try {
      const versions = await invoke<Record<string, string>>("get_installed_versions");
      setInstalledVersions(versions);
    } catch (error) {
      console.error("Failed to load installed versions:", error);
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

    loadInstalledVersions();
  }, [loadInstalledVersions]);

  const clearBusy = (serviceType: ServiceType) =>
    setBusy((b) => {
      const next = { ...b };
      delete next[serviceType];
      return next;
    });

  const startService = async (serviceType: ServiceType) => {
    setBusy((b) => ({ ...b, [serviceType]: "start" }));
    try {
      await invoke("start_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      const errMsg = String(error);
      if (serviceType === ServiceType.Ngrok && errMsg.includes("authtoken")) {
        setNgrokTokenInput("");
        setShowNgrokTokenModal(true);
      } else {
        alert(`Failed to start ${serviceType}:\n${error}`);
      }
      await refreshStatuses();
    } finally {
      clearBusy(serviceType);
    }
  };

  const saveNgrokTokenAndStart = async () => {
    if (!ngrokTokenInput.trim()) return;
    setNgrokTokenSaving(true);
    setBusy((b) => ({ ...b, [ServiceType.Ngrok]: "start" }));
    try {
      const settings = await invoke<any>("get_settings");
      settings.ngrok_authtoken = ngrokTokenInput.trim();
      await invoke("save_settings", { settings });
      setShowNgrokTokenModal(false);
      setNgrokTokenInput("");
      // Now start ngrok
      await invoke("start_service", { service: ServiceType.Ngrok });
      await refreshStatuses();
    } catch (error) {
      alert(`Failed to save authtoken:\n${error}`);
    } finally {
      setNgrokTokenSaving(false);
      clearBusy(ServiceType.Ngrok);
    }
  };

  const stopService = async (serviceType: ServiceType) => {
    setBusy((b) => ({ ...b, [serviceType]: "stop" }));
    try {
      await invoke("stop_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to stop ${serviceType}:`, error);
      alert(`Failed to stop ${serviceType}:\n${error}`);
      await refreshStatuses();
    } finally {
      clearBusy(serviceType);
    }
  };

  const restartService = async (serviceType: ServiceType) => {
    setBusy((b) => ({ ...b, [serviceType]: "restart" }));
    try {
      await invoke("restart_service", { service: serviceType });
      await refreshStatuses();
    } catch (error) {
      console.error(`Failed to restart ${serviceType}:`, error);
      alert(`Failed to restart ${serviceType}:\n${error}`);
      await refreshStatuses();
    } finally {
      clearBusy(serviceType);
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
            <div className="app-brand">
              <img src={logoUrl} alt="meLiveCode logo" className="brand-mark" />
              <div>
                <h1
                  style={{
                    fontSize: "1.25rem",
                    fontWeight: 700,
                    margin: 0,
                    letterSpacing: "-0.01em",
                    color: "var(--text-primary)",
                  }}
                >
                  CAMPP
                </h1>
                <p
                  style={{
                    fontSize: "0.8125rem",
                    color: "var(--text-secondary)",
                    marginTop: "0.125rem",
                    marginBottom: 0,
                  }}
                >
                  Caddy · PHP · {dbName} · phpMyAdmin · PostgreSQL
                </p>
              </div>
            </div>

            {/* Quick Actions */}
            <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem" }}>
              <button
                className="btn-quick-action"
                onClick={openWebServer}
                disabled={!isCaddyRunning}
                title={isCaddyRunning ? `Open ${webServerUrl}` : "Start Caddy to enable"}
              >
                <Icon name="globe" size={16} />
                Open Site
                {listenAllInterfaces && isCaddyRunning && (
                  <span style={{
                    fontSize: "0.6rem",
                    backgroundColor: "rgba(255, 180, 0, 0.15)",
                    color: "var(--color-warning)",
                    padding: "0.1rem 0.35rem",
                    borderRadius: "0.25rem",
                    marginLeft: "0.35rem",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.03em",
                  }}>
                    LAN
                  </span>
                )}
              </button>
              <button
                className="btn-quick-action"
                onClick={openProjectRoot}
                disabled={!installDir && !projectRoot}
                title={installDir || projectRoot ? `Open ${installDir || projectRoot}` : "Directory not set"}
              >
                <Icon name="folder" size={16} />
                Projects
              </button>
              <button
                className="btn-quick-action"
                onClick={openPhpMyAdmin}
                disabled={!isCaddyRunning || !installedVersions.phpmyadmin}
                title={!installedVersions.phpmyadmin ? "phpMyAdmin not installed" : isCaddyRunning ? `Open ${phpMyAdminUrl}` : "Start Caddy to enable"}
              >
                <Icon name="database" size={16} />
                phpMyAdmin
              </button>
              <button
                className="btn-quick-action"
                onClick={async () => { try { await openUrl(adminerUrl); } catch (error) { console.error("Failed to open Adminer URL:", error); } }}
                disabled={!isCaddyRunning || !installedVersions.adminer}
                title={!installedVersions.adminer ? "Adminer not installed" : isCaddyRunning ? `Open ${adminerUrl}` : "Start Caddy to enable"}
              >
                <Icon name="table" size={16} />
                Adminer
              </button>
              <button
                className="btn-quick-action"
                onClick={() => setShowSettings(true)}
                disabled={anyServiceActive}
                title={anyServiceActive ? "Stop all services to change settings" : "Open Settings"}
              >
                <Icon name="sliders" size={16} />
                Settings
              </button>
              <button
                className="btn-quick-action"
                onClick={() => setShowDebugMenu(true)}
                title="Open Debug Menu"
              >
                <Icon name="wrench" size={16} />
                Debug
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
                <Icon name="help" size={16} />
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
              {[
                installedVersions.caddy && { label: "Caddy", version: installedVersions.caddy },
                installedVersions.php && { label: "PHP", version: installedVersions.php },
                (installedVersions.mysql || installedVersions.mariadb) && { label: dbName, version: installedVersions.mysql || installedVersions.mariadb },
                installedVersions.phpmyadmin && { label: "phpMyAdmin", version: installedVersions.phpmyadmin },
                installedVersions.postgresql && { label: "PostgreSQL", version: installedVersions.postgresql },
                installedVersions.adminer && { label: "Adminer", version: installedVersions.adminer },
                installedVersions.ngrok && { label: "ngrok", version: installedVersions.ngrok },
              ]
                .filter((b): b is { label: string; version: string } => !!b)
                .map((b) => (
                  <span className="badge" key={b.label}>
                    {b.label}
                    <span className="badge__version"> {b.version}</span>
                  </span>
                ))}
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
              gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
              gap: "1rem",
              maxWidth: "80rem",
              margin: "0 auto",
            }}
          >
            {[ServiceType.Caddy, ServiceType.PhpFpm, ServiceType.MySQL, ServiceType.PostgreSQL, ...(currentPlatform === "windows" ? [ServiceType.Ngrok] : [])].map((serviceType) => {
              const service = services[serviceType];
              if (!service) return null;
              let componentKey: string = serviceType === ServiceType.PhpFpm ? "php" : serviceType;
              // For MySQL service, check platform-specific key (mysql or mariadb)
              const isNotInstalled = serviceType === ServiceType.MySQL
                ? !installedVersions.mysql && !installedVersions.mariadb
                : !installedVersions[componentKey];
              return (
                <ServiceCard
                  key={serviceType}
                  serviceType={serviceType}
                  state={service.state as ServiceState}
                  port={service.port}
                  error={service.error_message}
                  notInstalled={isNotInstalled}
                  busyAction={busy[serviceType]}
                  tunnelUrl={serviceType === ServiceType.Ngrok ? ngrokUrl : undefined}
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
            onSettingsChanged={async () => {
              await refreshStatuses();
              await loadInstalledVersions();
            }}
          />
        )}

        {/* Debug Menu */}
        {showDebugMenu && (
          <DebugMenu
            onClose={() => setShowDebugMenu(false)}
            onOpenRuntimeFolder={async () => {
              try {
                const runtimeDir = await invoke<string>("get_runtime_dir");
                await invoke("open_folder", { path: runtimeDir });
              } catch (error) {
                console.error("Failed to open folder:", error);
                alert("Failed to open folder: " + error);
              }
            }}
            onOpenDownloadFolder={async () => {
              try {
                const downloadDir = await invoke<string>("get_download_dir");
                await invoke("open_folder", { path: downloadDir });
              } catch (error) {
                console.error("Failed to open download folder:", error);
                alert("Failed to open download folder: " + error);
              }
            }}
            onResetInstallation={async () => {
              alert("This will reset your installation. All services will be stopped and all binaries, databases, and settings will be deleted. Your projects folder will be kept.");
              if (!confirm("Are you sure you want to reset?")) return;
              try {
                await invoke("reset_installation");
                window.location.reload();
              } catch (error) {
                console.error("Failed to reset:", error);
                alert("Failed to reset: " + error);
              }
            }}
          />
        )}

        {/* Ngrok Authtoken Modal */}
        {showNgrokTokenModal && (
          <div
            style={{
              position: "fixed",
              inset: 0,
              backgroundColor: "rgba(0, 0, 0, 0.5)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 1000,
            }}
            onClick={() => setShowNgrokTokenModal(false)}
          >
            <div
              style={{
                backgroundColor: "var(--bg-card)",
                borderRadius: "0.75rem",
                padding: "1.5rem",
                maxWidth: "28rem",
                width: "90%",
                boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.25)",
                border: "1px solid var(--border-color)",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h2 style={{ fontSize: "1.125rem", fontWeight: 600, margin: "0 0 0.5rem 0", color: "var(--text-primary)" }}>
                ngrok Authtoken Required
              </h2>
              <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", margin: "0 0 1rem 0", lineHeight: 1.5 }}>
                To share your local server via ngrok, you need a free authtoken.
                Sign up at{" "}
                <a
                  href="https://dashboard.ngrok.com/signup"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "var(--color-primary, #3b82f6)" }}
                >
                  ngrok.com
                </a>{" "}
                and copy your authtoken from the{" "}
                <a
                  href="https://dashboard.ngrok.com/get-started/your-authtoken"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "var(--color-primary, #3b82f6)" }}
                >
                  dashboard
                </a>.
              </p>
              <div style={{ marginBottom: "1rem" }}>
                <input
                  type="password"
                  value={ngrokTokenInput}
                  onChange={(e) => setNgrokTokenInput(e.target.value)}
                  placeholder="Paste your authtoken here"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveNgrokTokenAndStart();
                  }}
                  style={{
                    width: "100%",
                    padding: "0.5rem 0.75rem",
                    borderRadius: "0.375rem",
                    border: "1px solid var(--border-color)",
                    backgroundColor: "var(--bg-card-secondary)",
                    color: "var(--text-primary)",
                    fontSize: "0.875rem",
                    boxSizing: "border-box",
                  }}
                />
              </div>
              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button
                  onClick={() => setShowNgrokTokenModal(false)}
                  style={{
                    padding: "0.5rem 1rem",
                    borderRadius: "0.375rem",
                    border: "1px solid var(--border-color)",
                    backgroundColor: "transparent",
                    color: "var(--text-secondary)",
                    fontSize: "0.875rem",
                    cursor: "pointer",
                  }}
                >
                  Cancel
                </button>
                <button
                  onClick={saveNgrokTokenAndStart}
                  disabled={!ngrokTokenInput.trim() || ngrokTokenSaving}
                  style={{
                    padding: "0.5rem 1rem",
                    borderRadius: "0.375rem",
                    border: "none",
                    backgroundColor: ngrokTokenInput.trim() ? "var(--color-primary, #3b82f6)" : "var(--bg-card-secondary)",
                    color: ngrokTokenInput.trim() ? "#fff" : "var(--text-secondary)",
                    fontSize: "0.875rem",
                    cursor: ngrokTokenInput.trim() ? "pointer" : "not-allowed",
                    fontWeight: 500,
                  }}
                >
                  {ngrokTokenSaving ? "Saving..." : "Save & Start ngrok"}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
