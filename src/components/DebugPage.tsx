import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { AppSettings, ServiceMap, ServiceType, ServiceState, getDatabaseDisplayName } from "../types/services";
import { Icon } from "./icons";

interface DebugInfo {
  version: string;
  exePath: string;
  resourceDir: string;
  runtimeDir: string;
  configLoaded: boolean;
  os: string;
  arch: string;
}

type PortCheck = Record<"web" | "php" | "mysql" | "postgres", { port: number; available: boolean }>;

const PORT_ROWS: { key: keyof PortCheck; label: () => string; service: ServiceType }[] = [
  { key: "web", label: () => "Web server", service: ServiceType.Caddy },
  { key: "php", label: () => "PHP-FPM", service: ServiceType.PhpFpm },
  { key: "mysql", label: () => getDatabaseDisplayName(), service: ServiceType.MySQL },
  { key: "postgres", label: () => "PostgreSQL", service: ServiceType.PostgreSQL },
];

export function DebugPage({ services }: { services: Partial<ServiceMap> }) {
  const [info, setInfo] = useState<DebugInfo | null>(null);
  const [folders, setFolders] = useState<{ name: string; path: string }[]>([]);
  const [ports, setPorts] = useState<PortCheck | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirmingReset, setConfirmingReset] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkPorts = useCallback(async () => {
    try {
      const s = await invoke<AppSettings>("get_settings");
      setPorts(await invoke<PortCheck>("check_ports", {
        webPort: s.web_port,
        phpPort: s.php_port,
        mysqlPort: s.mysql_port,
        postgresPort: s.postgres_port,
      }));
    } catch (e) {
      console.error("Failed to check ports:", e);
    }
  }, []);

  useEffect(() => {
    invoke<DebugInfo>("get_debug_info").then(setInfo).catch((e) => console.error("Failed to load debug info:", e));
    Promise.all([
      invoke<string>("get_runtime_dir").then((path) => ({ name: "Runtime", path })),
      invoke<string>("get_download_dir").then((path) => ({ name: "Downloads", path })),
      invoke<string>("get_logs_dir").then((path) => ({ name: "Logs", path })),
    ]).then(setFolders).catch((e) => console.error("Failed to load folders:", e));
    checkPorts();
  }, [checkPorts]);

  const openFolder = async (path: string) => {
    try {
      await invoke("open_folder", { path });
    } catch (e) {
      setError(`Failed to open folder: ${e}`);
    }
  };

  const copyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify({ ...info, folders, ports }, null, 2));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      setError(`Failed to copy: ${e}`);
    }
  };

  const resetInstallation = async () => {
    setResetting(true);
    try {
      await invoke("reset_installation");
      window.location.reload();
    } catch (e) {
      setError(`Failed to reset: ${e}`);
      setResetting(false);
      setConfirmingReset(false);
    }
  };

  const systemRows: { k: string; v: string; ok?: boolean }[] = info
    ? [
        { k: "Version", v: `v${info.version}` },
        { k: "Platform", v: `${info.os} · ${info.arch}` },
        { k: "Executable", v: info.exePath },
        { k: "Resource folder", v: info.resourceDir },
        { k: "Runtime folder", v: info.runtimeDir },
        { k: "Runtime config", v: info.configLoaded ? "Loaded" : "Not loaded", ok: info.configLoaded },
      ]
    : [];

  return (
    <div className="page">
      <header className="main__header">
        <div>
          <h2>Debug</h2>
          <p>Troubleshooting tools for this CAMPP install.</p>
        </div>
        <button type="button" className="btn-secondary btn-sm" onClick={copyDiagnostics} disabled={!info}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <rect x="9" y="9" width="11" height="11" rx="2" />
            <path d="M5 15V5a1 1 0 0 1 1-1h10" />
          </svg>
          {copied ? "Copied" : "Copy diagnostics"}
        </button>
      </header>

      {error && (
        <div className="error-box">
          <p className="error-box-text">{error}</p>
        </div>
      )}

      <section className="card-section" aria-labelledby="d-system">
        <div className="card-section__head">
          <h3 id="d-system">System</h3>
        </div>
        <dl className="kv-grid">
          {systemRows.map((r) => (
            <div key={r.k}>
              <dt>{r.k}</dt>
              <dd className={`mono${r.ok === true ? " text-ok" : r.ok === false ? " text-bad" : ""}`} title={r.v}>{r.v}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section className="card-section" aria-labelledby="d-folders">
        <div className="card-section__head">
          <h3 id="d-folders">Folders</h3>
        </div>
        <div className="list-box">
          {folders.map((f) => (
            <div key={f.name} className="list-row">
              <span className="quick-card__icon"><Icon name="folder" size={16} /></span>
              <div className="list-row__main">
                <span>{f.name}</span>
                <span className="mono" title={f.path}>{f.path}</span>
              </div>
              <button type="button" className="btn-secondary btn-xs" onClick={() => openFolder(f.path)}>
                Open
              </button>
            </div>
          ))}
        </div>
      </section>

      <section className="card-section" aria-labelledby="d-ports">
        <div className="card-section--row">
          <div className="card-section__head">
            <h3 id="d-ports">Port check</h3>
            <p>Whether each configured port is free for CAMPP to use.</p>
          </div>
          <button type="button" className="btn-secondary btn-xs" onClick={checkPorts}>
            Re-check
          </button>
        </div>
        <div className="port-grid">
          {PORT_ROWS.map((row) => {
            const p = ports?.[row.key];
            const ownedByCampp = services[row.service]?.state === ServiceState.Running;
            const status = !p ? "…" : p.available ? "Available" : ownedByCampp ? "Used by CAMPP" : "In use";
            const cls = !p ? "" : p.available || ownedByCampp ? " status-green" : " status-red";
            return (
              <div key={row.key} className="port-tile">
                <span className="port-tile__label">{row.label()}</span>
                <span className="port-tile__port mono">{p?.port ?? "—"}</span>
                <span className={`service-status${cls}`}>
                  <span className="status-dot" />
                  {status}
                </span>
              </div>
            );
          })}
        </div>
      </section>

      <section className="card-section card-section--danger" aria-labelledby="d-reset">
        <div className="card-section--row">
          <div className="card-section__head">
            <h3 id="d-reset">Reset installation</h3>
            <p>Stops every service and deletes all binaries, databases and settings. Your projects folder is kept.</p>
          </div>
          {!confirmingReset && (
            <button type="button" className="btn-secondary btn-sm btn-danger" onClick={() => setConfirmingReset(true)}>
              Reset installation…
            </button>
          )}
        </div>
        {confirmingReset && (
          <div className="danger-confirm">
            <span>This can't be undone. Reset now?</span>
            <div className="main__actions">
              <button type="button" className="btn-secondary btn-xs" onClick={() => setConfirmingReset(false)} disabled={resetting}>
                Cancel
              </button>
              <button type="button" className="btn-danger-solid btn-xs" onClick={resetInstallation} disabled={resetting}>
                {resetting ? "Resetting…" : "Yes, reset everything"}
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
