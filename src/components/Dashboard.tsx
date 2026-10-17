import { invoke } from "@tauri-apps/api/core";
import { showAlert } from "./Dialog";
import { LOGOS } from "../assets/logos";
import { PasswordInput } from "./PasswordInput";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useState, useEffect, useCallback } from "react";
import { ServiceMap, ServiceType, ServiceState } from "../types/services";
import { ServiceCard } from "./ServiceCard";
import { StatusBar } from "./StatusBar";
import { SettingsPanel } from "./SettingsPanel";
import { DebugPage } from "./DebugPage";
import { Icon } from "./icons";
import { detectPlatform } from "../utils/platform";
import logoUrl from "../assets/melivecode-logo.png";
import phpMyAdminLogo from "../assets/logos/phpmyadmin.svg";
import adminerLogo from "../assets/logos/adminer.svg";

export function Dashboard() {
  const [services, setServices] = useState<Partial<ServiceMap>>({});
  // Which action is in flight per service. start_service/stop_service block
  // until the backend finishes (holding the process lock, so the poll can't
  // observe the transient Starting/Stopping state) — track it here so the card
  // can show a spinner immediately on click.
  const [busy, setBusy] = useState<Partial<Record<ServiceType, "start" | "stop" | "restart">>>({});
  const [view, setView] = useState<"services" | "settings" | "debug">("services");
  const [projectRoot, setProjectRoot] = useState<string>("");
  const [installDir, setInstallDir] = useState<string>("");
  const [installedVersions, setInstalledVersions] = useState<Record<string, string>>({});
  const [listenAllInterfaces, setListenAllInterfaces] = useState(false);
  const [ngrokUrl, setNgrokUrl] = useState<string | null>(null);
  const [showNgrokTokenModal, setShowNgrokTokenModal] = useState(false);
  const [ngrokTokenInput, setNgrokTokenInput] = useState("");
  const [ngrokTokenSaving, setNgrokTokenSaving] = useState(false);

  const currentPlatform = detectPlatform();

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
        void showAlert(`Failed to start ${serviceType}`, String(error));
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
      void showAlert("Failed to save authtoken", String(error));
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
      void showAlert(`Failed to stop ${serviceType}`, String(error));
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
      void showAlert(`Failed to restart ${serviceType}`, String(error));
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

  const serviceOrder = [ServiceType.Caddy, ServiceType.PhpFpm, ServiceType.MySQL, ServiceType.PostgreSQL, ...(currentPlatform === "windows" ? [ServiceType.Ngrok] : [])];

  const isInstalled = (serviceType: ServiceType) =>
    serviceType === ServiceType.MySQL
      ? !!(installedVersions.mysql || installedVersions.mariadb)
      : !!installedVersions[serviceType === ServiceType.PhpFpm ? "php" : serviceType];

  const anyRunning = Object.values(services).some((s) => s?.state === ServiceState.Running);
  const anyBusy = Object.keys(busy).length > 0;

  // ngrok is left out of "Start all": it needs an authtoken and exposes the site publicly.
  const startAll = async () => {
    for (const t of serviceOrder) {
      if (t !== ServiceType.Ngrok && isInstalled(t) && services[t]?.state !== ServiceState.Running) {
        await startService(t);
      }
    }
  };

  const stopAll = async () => {
    for (const t of [...serviceOrder].reverse()) {
      if (services[t]?.state === ServiceState.Running) await stopService(t);
    }
  };

  return (
    <>
      <div className="app-shell" data-testid="dashboard">
        <aside className="sidebar">
          <div className="sidebar__brand">
            <img src={logoUrl} alt="meLiveCode logo" />
            <div>
              <h1>CAMPP</h1>
              <span>Local dev stack</span>
            </div>
          </div>

          <nav className="sidebar__nav" aria-label="Main">
            <button
              type="button"
              className={`nav-item${view === "services" ? " nav-item--active" : ""}`}
              aria-current={view === "services" ? "page" : undefined}
              onClick={() => setView("services")}
            >
              <Icon name="layers" size={16} />
              Services
            </button>
            <button
              type="button"
              className={`nav-item${view === "settings" ? " nav-item--active" : ""}`}
              aria-current={view === "settings" ? "page" : undefined}
              onClick={() => setView("settings")}
              disabled={anyServiceActive && view !== "settings"}
              title={anyServiceActive ? "Stop all services to change settings" : "Open Settings"}
            >
              <Icon name="sliders" size={16} />
              Settings
            </button>
            <button
              type="button"
              className={`nav-item${view === "debug" ? " nav-item--active" : ""}`}
              aria-current={view === "debug" ? "page" : undefined}
              onClick={() => setView("debug")}
            >
              <Icon name="wrench" size={16} />
              Debug
            </button>
          </nav>

          <div className="sidebar__footer">
            <StatusBar services={services} data-testid="status-bar" />
            <button
              type="button"
              className="nav-item"
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
              User manual
            </button>
          </div>
        </aside>

        <main className="main">
          {view === "settings" ? (
            <SettingsPanel
              onClose={() => setView("services")}
              onSettingsChanged={async () => {
                await refreshStatuses();
                await loadInstalledVersions();
              }}
            />
          ) : view === "debug" ? (
            <DebugPage services={services} />
          ) : (<>
          <header className="main__header">
            <div>
              <h2>Services</h2>
              <p>
                {listenAllInterfaces ? (
                  <>Listening on all interfaces <span className="lan-badge">LAN</span></>
                ) : (
                  <>Local stack on <span className="mono">127.0.0.1</span></>
                )}
              </p>
            </div>
            <div className="main__actions">
              <button
                type="button"
                className="btn-secondary btn-sm"
                onClick={openWebServer}
                disabled={!isCaddyRunning}
                title={isCaddyRunning ? `Open ${webServerUrl}` : "Start Caddy to enable"}
              >
                <Icon name="globe" size={15} />
                Open site
              </button>
              <button
                type="button"
                className="btn-primary btn-sm"
                onClick={anyRunning ? stopAll : startAll}
                disabled={anyBusy}
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                  <path d={anyRunning ? "M6 6h12v12H6z" : "M7 4.5v15l13-7.5z"} />
                </svg>
                {anyRunning ? "Stop all" : "Start all"}
              </button>
            </div>
          </header>

          <section className="service-list" aria-label="Services">
            {serviceOrder.map((serviceType) => {
              const service = services[serviceType];
              if (!service) return null;
              const version = serviceType === ServiceType.MySQL
                ? installedVersions.mysql || installedVersions.mariadb
                : installedVersions[serviceType === ServiceType.PhpFpm ? "php" : serviceType];
              return (
                <ServiceCard
                  key={serviceType}
                  serviceType={serviceType}
                  state={service.state as ServiceState}
                  port={service.port}
                  error={service.error_message}
                  notInstalled={!isInstalled(serviceType)}
                  version={version}
                  busyAction={busy[serviceType]}
                  tunnelUrl={serviceType === ServiceType.Ngrok ? ngrokUrl : undefined}
                  onStart={() => startService(serviceType)}
                  onStop={() => stopService(serviceType)}
                  onRestart={() => restartService(serviceType)}
                />
              );
            })}
          </section>

          <section className="quick-grid" aria-label="Quick open">
            <button
              type="button"
              className="quick-card"
              onClick={openWebServer}
              disabled={!isCaddyRunning}
              title={isCaddyRunning ? `Open ${webServerUrl}` : "Start Caddy to enable"}
            >
              <span className="quick-card__icon"><Icon name="globe" size={16} /></span>
              <span className="quick-card__text">
                <span>Website</span>
                <span className="mono">{webServerUrl.replace("http://", "")}</span>
              </span>
            </button>
            <button
              type="button"
              className="quick-card"
              onClick={openPhpMyAdmin}
              disabled={!isCaddyRunning || !installedVersions.phpmyadmin}
              title={!installedVersions.phpmyadmin ? "phpMyAdmin not installed" : isCaddyRunning ? `Open ${phpMyAdminUrl}` : "Start Caddy to enable"}
            >
              <span className="logo-tile logo-tile--sm"><img src={phpMyAdminLogo} alt="" /></span>
              <span className="quick-card__text">
                <span>phpMyAdmin</span>
                <span className="mono">/phpmyadmin</span>
              </span>
            </button>
            <button
              type="button"
              className="quick-card"
              onClick={async () => { try { await openUrl(adminerUrl); } catch (error) { console.error("Failed to open Adminer URL:", error); } }}
              disabled={!isCaddyRunning || !installedVersions.adminer}
              title={!installedVersions.adminer ? "Adminer not installed" : isCaddyRunning ? `Open ${adminerUrl}` : "Start Caddy to enable"}
            >
              <span className="logo-tile logo-tile--sm"><img src={adminerLogo} alt="" /></span>
              <span className="quick-card__text">
                <span>Adminer</span>
                <span className="mono">/adminer</span>
              </span>
            </button>
            <button
              type="button"
              className="quick-card"
              onClick={openProjectRoot}
              disabled={!installDir && !projectRoot}
              title={installDir || projectRoot ? `Open ${installDir || projectRoot}` : "Directory not set"}
            >
              <span className="quick-card__icon"><Icon name="folder" size={16} /></span>
              <span className="quick-card__text">
                <span>Projects folder</span>
                <span className="mono">{installDir || projectRoot || "Not set"}</span>
              </span>
            </button>
          </section>
          </>)}
        </main>

        {/* Ngrok Authtoken Modal */}
        {showNgrokTokenModal && (
          <div className="modal-overlay" onClick={() => setShowNgrokTokenModal(false)}>
            <div
              className="modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="ngrok-token-title"
              onClick={(e) => e.stopPropagation()}
            >
              <header className="modal__head">
                <span className="logo-tile"><img src={LOGOS.ngrok} alt="" /></span>
                <div>
                  <h3 id="ngrok-token-title">ngrok authtoken required</h3>
                  <p>Needed once to share your local server.</p>
                </div>
              </header>
              <div className="modal__body">
                <p className="wiz-muted">
                  Sign up free at{" "}
                  <a className="link" href="https://dashboard.ngrok.com/signup" target="_blank" rel="noopener noreferrer">
                    ngrok.com
                  </a>{" "}
                  and copy your authtoken from the{" "}
                  <a
                    className="link"
                    href="https://dashboard.ngrok.com/get-started/your-authtoken"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    dashboard
                  </a>
                  .
                </p>
                <PasswordInput
                  className="mono"
                  aria-label="ngrok authtoken"
                  value={ngrokTokenInput}
                  onChange={(e) => setNgrokTokenInput(e.target.value)}
                  placeholder="Paste your authtoken here"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveNgrokTokenAndStart();
                  }}
                />
              </div>
              <footer className="modal__foot">
                <button type="button" className="btn-secondary btn-sm" onClick={() => setShowNgrokTokenModal(false)}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="btn-primary btn-sm"
                  onClick={saveNgrokTokenAndStart}
                  disabled={!ngrokTokenInput.trim() || ngrokTokenSaving}
                >
                  {ngrokTokenSaving ? "Saving..." : "Save & Start ngrok"}
                </button>
              </footer>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
