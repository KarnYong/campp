import { invoke } from "@tauri-apps/api/core";
import { showAlert, showConfirm } from "./Dialog";
import { PasswordInput } from "./PasswordInput";
import { useState, useEffect, useCallback } from "react";
import { AppSettings, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";
import { applyTheme, themeLabel } from "../utils/theme";
import { NgrokAntivirusNotice } from "./NgrokAntivirusNotice";
import { ReinstallDialog } from "./ReinstallDialog";
import { LOGOS } from "../assets/logos";

interface SettingsPanelProps {
  onClose: () => void;
  onSettingsChanged?: () => void;
  [key: string]: any; // Allow additional props like data-testid
}

interface ComponentInfo {
  key: string;
  displayName: string;
  installed: boolean;
  version: string | null;
}

const COMPONENT_ORDER: { key: string; getDisplayName: () => string }[] = (() => {
  const platform = detectPlatform();
  const dbKey = platform === "linux" ? "mariadb" : "mysql";
  return [
    { key: "caddy", getDisplayName: () => "Caddy" },
    { key: "php", getDisplayName: () => "PHP" },
    { key: dbKey, getDisplayName: () => getDatabaseDisplayName(platform) },
    { key: "phpmyadmin", getDisplayName: () => "phpMyAdmin" },
    { key: "postgresql", getDisplayName: () => "PostgreSQL" },
    { key: "adminer", getDisplayName: () => "Adminer" },
    { key: "ngrok", getDisplayName: () => "ngrok" },
  ];
})();

export function SettingsPanel({ onClose, onSettingsChanged, ...props }: SettingsPanelProps) {
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
  const [components, setComponents] = useState<ComponentInfo[]>([]);
  const [componentAction, setComponentAction] = useState<string | null>(null);
  // Last uninstall error (install errors are shown inside ReinstallDialog).
  const [componentError, setComponentError] = useState<string | null>(null);
  const [installTarget, setInstallTarget] = useState<ComponentInfo | null>(null);

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
      const versions = await invoke<Record<string, string>>("check_existing_components");
      const infos: ComponentInfo[] = COMPONENT_ORDER.map(({ key, getDisplayName }) => ({
        key,
        displayName: getDisplayName(),
        installed: !!versions[key],
        version: versions[key] || null,
      }));
      setComponents(infos);
    } catch (e) {
      console.error("Failed to load components:", e);
    }
  }, []);

  useEffect(() => {
    loadSettings();
    loadComponents();
  }, [loadSettings, loadComponents]);

  const refreshComponents = async () => {
    await loadComponents();
    onSettingsChanged?.();
  };

  const handlePortChange = (field: keyof AppSettings, value: string) => {
    const numValue = parseInt(value, 10);
    if (isNaN(numValue) || numValue < 1 || numValue > 65535) return;
    setSettings({ ...settings, [field]: numValue });
  };

  const isInstalled = (key: string) => !!components.find((c) => c.key === key)?.installed;

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

  const handleUninstall = async (componentKey: string) => {
    clearComponentError();
    if (componentKey === "php" || componentKey === "mysql" || componentKey === "mariadb") {
      const name = componentKey === "php" ? "PHP" : getDatabaseDisplayName();
      const msg = componentKey === "php"
        ? "PHP is required by Caddy for serving PHP files and phpMyAdmin. Uninstall anyway?"
        : `${getDatabaseDisplayName()} is required by phpMyAdmin. Uninstall anyway?`;
      if (!(await showConfirm(`Uninstall ${name}?`, msg, "Uninstall", true))) return;
    }

    // Prevent uninstalling last component
    const installedCount = components.filter(c => c.installed).length;
    if (installedCount <= 1) {
      await showAlert("Can't uninstall", "At least one component must remain installed.");
      return;
    }

    const actionKey = `uninstall-${componentKey}`;
    setComponentAction(actionKey);
    try {
      await invoke("uninstall_component", { component: componentKey });
      await refreshComponents();
    } catch (e) {
      setComponentError(`Failed to uninstall: ${e}`);
    } finally {
      setComponentAction(null);
    }
  };

  // Install/reinstall happen in ReinstallDialog (version picker + progress).
  const openInstallDialog = (componentKey: string) => {
    clearComponentError();
    setInstallTarget(components.find((c) => c.key === componentKey) ?? null);
  };

  // Clear a stale error when a new action starts.
  const clearComponentError = () => setComponentError(null);

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
          <button type="button" className="btn-primary btn-sm" onClick={handleSave} disabled={saving}>
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

      {/* Appearance */}
      <section className="card-section card-section--row" aria-labelledby="s-appearance">
        <div className="card-section__head">
          <h3 id="s-appearance">Appearance</h3>
          <p>{themeLabel(settings.theme)}</p>
        </div>
        <div className="seg" role="group" aria-label="Theme">
          {(["system", "light", "dark"] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              aria-pressed={(settings.theme || "system") === mode}
              onClick={() => {
                setSettings({ ...settings, theme: mode });
                applyTheme(mode);
              }}
            >
              {mode.charAt(0).toUpperCase() + mode.slice(1)}
            </button>
          ))}
        </div>
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

      {/* Components */}
      <section className="card-section" aria-labelledby="s-components">
        <div className="card-section__head">
          <h3 id="s-components">Components</h3>
          <p>Install, reinstall or remove the software CAMPP runs.</p>
        </div>
        {componentError && (
          <div className="error-box">
            <p className="error-box-text" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", marginBottom: "0.5rem" }}>
              {componentError}
            </p>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <button type="button" onClick={clearComponentError} className="btn-secondary btn-xs btn-danger">
                Dismiss
              </button>
            </div>
          </div>
        )}
        <div className="list-box">
          {components.map((comp) => {
            const isActing = componentAction === `uninstall-${comp.key}`;
            const canUninstall = comp.installed && components.filter((c) => c.installed).length > 1;
            return (
              <div key={comp.key} className="list-row">
                <span className="logo-tile logo-tile--sm"><img src={LOGOS[comp.key]} alt="" /></span>
                <div className="list-row__main">
                  <span>{comp.displayName}</span>
                  {comp.installed && comp.version ? (
                    <span className="mono">v{comp.version}</span>
                  ) : (
                    <span className="list-row__warn">Not installed</span>
                  )}
                </div>
                {isActing && (
                  <span className="list-row__busy">
                    <span className="btn-spinner" aria-hidden="true" />
                    Working…
                  </span>
                )}
                {comp.installed ? (
                  <>
                    <button
                      type="button"
                      onClick={() => openInstallDialog(comp.key)}
                      disabled={!!componentAction}
                      className="btn-secondary btn-xs"
                    >
                      Reinstall
                    </button>
                    <button
                      type="button"
                      onClick={() => handleUninstall(comp.key)}
                      disabled={!!componentAction || !canUninstall}
                      className="btn-secondary btn-xs btn-danger"
                      title={!canUninstall ? "At least one component must remain installed" : undefined}
                    >
                      Uninstall
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={() => openInstallDialog(comp.key)}
                    disabled={!!componentAction}
                    className="btn-primary btn-xs"
                  >
                    Install
                  </button>
                )}
              </div>
            );
          })}
        </div>
        {detectPlatform() === "windows" && <NgrokAntivirusNotice />}
      </section>

      {installTarget && (
        <ReinstallDialog
          componentKey={installTarget.key}
          displayName={installTarget.displayName}
          currentVersion={installTarget.installed ? installTarget.version : null}
          onClose={(changed) => {
            setInstallTarget(null);
            if (changed) refreshComponents();
          }}
        />
      )}
    </div>
  );
}
