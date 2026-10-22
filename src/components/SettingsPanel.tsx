import { invoke } from "@tauri-apps/api/core";
import { showConfirm } from "./Dialog";
import { PasswordInput } from "./PasswordInput";
import { useState, useEffect, useCallback } from "react";
import { AppSettings, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";
import { LOGOS } from "../assets/logos";

interface SettingsPanelProps {
  onClose: () => void;
  onSettingsChanged?: () => void;
  /** Any service running: settings are read-only until everything is stopped. */
  servicesActive?: boolean;
  [key: string]: any; // Allow additional props like data-testid
}

export function SettingsPanel({ onClose, onSettingsChanged, servicesActive = false, ...props }: SettingsPanelProps) {
  const dbComponentKey = detectPlatform() === "linux" ? "mariadb" : "mysql";
  const [settings, setSettings] = useState<AppSettings>({
    web_port: 8080,
    php_port: 9000,
    mysql_port: 3307,
    postgres_port: 5433,
    project_root: "",
    mysql_root_password: "",
    postgres_root_password: "",
    listen_all_interfaces: false,
    ngrok_authtoken: "",
  });
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [confirmingWipe, setConfirmingWipe] = useState(false);
  const [wiping, setWiping] = useState(false);
  // Installed component -> version; ports/passwords only show for installed ones.
  const [installed, setInstalled] = useState<Record<string, string>>({});

  const loadSettings = useCallback(async () => {
    try {
      const loaded = await invoke<AppSettings>("get_settings");
      setSettings(loaded);
    } catch (e) {
      setError(`Failed to load settings: ${e}`);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadComponents = useCallback(async () => {
    try {
      setInstalled(await invoke<Record<string, string>>("check_existing_components"));
    } catch (e) {
      console.error("Failed to load components:", e);
    }
  }, []);

  useEffect(() => {
    loadSettings();
    loadComponents();
  }, [loadSettings, loadComponents]);

  const handlePortChange = (field: keyof AppSettings, value: string) => {
    const numValue = parseInt(value, 10);
    if (isNaN(numValue) || numValue < 1 || numValue > 65535) return;
    setSettings({ ...settings, [field]: numValue });
  };

  const isInstalled = (key: string) => !!installed[key];

  const handleDeleteAll = async () => {
    setWiping(true);
    setError(null);
    setSuccess(null);
    try {
      const safety = await invoke<string>("delete_all_data");
      setSuccess(`All projects and databases deleted. A backup was saved first (${safety}); restore it from the Backup page to undo.`);
      setConfirmingWipe(false);
      onSettingsChanged?.();
    } catch (e) {
      setError(`Delete failed: ${e}`);
    } finally {
      setWiping(false);
    }
  };

  const handleSave = async () => {
    // A blank root password would silently open the database to anyone with access.
    if (isInstalled(dbComponentKey) && !settings.mysql_root_password.trim()) {
      setError(`${getDatabaseDisplayName()} root password can't be empty.`);
      return;
    }
    if (isInstalled("postgresql") && !settings.postgres_root_password.trim()) {
      setError("PostgreSQL root password can't be empty.");
      return;
    }
    setSaving(true);
    setError(null);
    setSuccess(null);

    try {
      await invoke("save_settings", { settings });
      setSuccess("Settings saved successfully!");
      onSettingsChanged?.();
    } catch (e) {
      setError(`Failed to save settings: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="page-loading">Loading settings…</p>;
  }

  const ports: { id: string; field: keyof AppSettings; label: string; hint: string; key: string }[] = [
    { id: "web-port", field: "web_port", label: "Web server", hint: "Caddy", key: "caddy" },
    { id: "php-port", field: "php_port", label: "PHP-FPM", hint: "Internal FastCGI", key: "php" },
    { id: "mysql-port", field: "mysql_port", label: getDatabaseDisplayName(), hint: "Database", key: dbComponentKey },
    { id: "postgres-port", field: "postgres_port", label: "PostgreSQL", hint: "Database", key: "postgresql" },
  ];

  return (
    <div className="page" {...props}>
      <header className="main__header">
        <div>
          <h2>Settings</h2>
          <p>Changes apply the next time services start.</p>
        </div>
        <div className="main__actions">
          <button type="button" className="btn-secondary btn-sm" onClick={onClose} disabled={saving}>
            Cancel
          </button>
          <button
            type="button"
            className="btn-primary btn-sm"
            onClick={handleSave}
            disabled={saving || servicesActive}
            title={servicesActive ? "Stop all services first" : undefined}
          >
            {saving ? "Saving…" : "Save changes"}
          </button>
        </div>
      </header>

      {error && (
        <div className="error-box">
          <p className="error-box-text">{error}</p>
        </div>
      )}
      {success && (
        <div className="success-box">
          <p className="success-box-text">{success}</p>
        </div>
      )}

      {servicesActive && (
        <div className="warn-box">
          <span>Stop all services to change settings. Ports, network and passwords can't be changed while services are running.</span>
        </div>
      )}

      {/* Read-only while services run: disabling the fieldset disables every control inside. */}
      <fieldset className="fieldset-plain" disabled={servicesActive}>
      {/* Network */}
      <section className="card-section" aria-labelledby="s-network">
        <div className="card-section--row">
          <div className="card-section__head">
            <h3 id="s-network">Network</h3>
            <p>
              {settings.listen_all_interfaces
                ? "Reachable from other devices on your network"
                : "Reachable only from this computer"}
            </p>
          </div>
          <div className="seg" role="group" aria-label="Network mode">
            <button
              type="button"
              aria-pressed={!settings.listen_all_interfaces}
              onClick={() => setSettings({ ...settings, listen_all_interfaces: false })}
            >
              Local only
            </button>
            <button
              type="button"
              aria-pressed={!!settings.listen_all_interfaces}
              onClick={async () => {
                if (!settings.listen_all_interfaces && !(await showConfirm(
                  "Enable LAN mode?",
                  "LAN mode exposes your development services to all devices on your network. " +
                  "Only use this on trusted, private networks.",
                  "Enable LAN mode",
                  true
                ))) {
                  return;
                }
                setSettings({ ...settings, listen_all_interfaces: true });
              }}
            >
              LAN
            </button>
          </div>
        </div>
        {settings.listen_all_interfaces && (
          <div className="warn-box">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M12 3l9.5 17H2.5z" />
              <path d="M12 10v4M12 17h.01" />
            </svg>
            <span>LAN mode exposes your dev services to every device on this network. Use it only on trusted networks and check your firewall.</span>
          </div>
        )}
      </section>

      {/* Ports */}
      <section className="card-section" aria-labelledby="s-ports">
        <div className="card-section__head">
          <h3 id="s-ports">Ports</h3>
          <p>Non-default ports, so CAMPP runs without admin rights.</p>
        </div>
        <div className="tile-grid">
          {ports.map((p) => {
            const installed = isInstalled(p.key);
            return (
              <div key={p.id} className={`tile${installed ? "" : " tile--disabled"}`}>
                <span className="logo-tile logo-tile--sm"><img src={LOGOS[p.key]} alt="" /></span>
                <label htmlFor={p.id} className="tile__label">
                  <span>{p.label}</span>
                  <span>{installed ? p.hint : "Not installed"}</span>
                </label>
                <input
                  id={p.id}
                  type="number"
                  className="input input--port mono"
                  value={settings[p.field] as number}
                  onChange={(e) => handlePortChange(p.field, e.target.value)}
                  min={1}
                  max={65535}
                  disabled={!installed}
                />
              </div>
            );
          })}
        </div>
      </section>

      {/* ngrok — only visible when ngrok is installed */}
      {isInstalled("ngrok") && (
        <section className="card-section card-section--row" aria-labelledby="s-ngrok">
          <div className="card-section__head">
            <h3 id="s-ngrok">ngrok tunnel</h3>
            <p>
              Share your local site publicly. Get a free token at{" "}
              <a className="link" href="https://dashboard.ngrok.com/signup" target="_blank" rel="noopener noreferrer">ngrok.com</a>.
            </p>
          </div>
          <div className="field">
            <label htmlFor="ngrok-token">Authtoken</label>
            <PasswordInput
              id="ngrok-token"
              className="mono"
              value={settings.ngrok_authtoken || ""}
              onChange={(e) => setSettings({ ...settings, ngrok_authtoken: e.target.value })}
              placeholder="Paste your ngrok authtoken"
            />
          </div>
        </section>
      )}

      {/* Database passwords: services are stopped while Settings is open, so the
          new password is applied on the next database start. */}
      {(isInstalled(dbComponentKey) || isInstalled("postgresql")) && (
        <section className="card-section" aria-labelledby="s-db-passwords">
          <div className="card-section__head">
            <h3 id="s-db-passwords">Database passwords</h3>
            <p>Root password for each database. Applied the next time the database starts.</p>
          </div>
          {isInstalled(dbComponentKey) && (
            <div className="field">
              <label htmlFor="mysql-password">{getDatabaseDisplayName()} root password</label>
              <PasswordInput
                id="mysql-password"
                value={settings.mysql_root_password || ""}
                onChange={(e) => setSettings({ ...settings, mysql_root_password: e.target.value })}
                autoComplete="new-password"
              />
            </div>
          )}
          {isInstalled("postgresql") && (
            <div className="field">
              <label htmlFor="postgres-password">PostgreSQL root password</label>
              <PasswordInput
                id="postgres-password"
                value={settings.postgres_root_password || ""}
                onChange={(e) => setSettings({ ...settings, postgres_root_password: e.target.value })}
                autoComplete="new-password"
              />
            </div>
          )}
        </section>
      )}

      {/* Danger zone */}
      <section className="card-section card-section--danger" aria-labelledby="s-danger">
        <div className="card-section--row">
          <div className="card-section__head">
            <h3 id="s-danger">Delete all projects and databases</h3>
            <p>Empties the projects folder and deletes every MySQL/MariaDB and PostgreSQL database. A backup is taken first.</p>
          </div>
          {!confirmingWipe && (
            <button type="button" className="btn-secondary btn-sm btn-danger" onClick={() => setConfirmingWipe(true)}>
              Delete everything…
            </button>
          )}
        </div>
        {confirmingWipe && (
          <div className="danger-confirm">
            <span>All your sites and data will be gone (you can restore the automatic backup). Delete now?</span>
            <div className="main__actions">
              <button type="button" className="btn-secondary btn-xs" onClick={() => setConfirmingWipe(false)} disabled={wiping}>
                Cancel
              </button>
              <button type="button" className="btn-danger-solid btn-xs" onClick={handleDeleteAll} disabled={wiping}>
                {wiping ? "Deleting…" : "Yes, delete everything"}
              </button>
            </div>
          </div>
        )}
      </section>
      </fieldset>

    </div>
  );
}
