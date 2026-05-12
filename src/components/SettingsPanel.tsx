import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { AppSettings, PackageSelection, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";

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
  ];
})();

export function SettingsPanel({ onClose, onSettingsChanged, ...props }: SettingsPanelProps) {
  const dbComponentKey = detectPlatform() === "linux" ? "mariadb" : "mysql";
  const [settings, setSettings] = useState<AppSettings>({
    web_port: 8080,
    php_port: 9000,
    mysql_port: 3307,
    project_root: "",
  });
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [components, setComponents] = useState<ComponentInfo[]>([]);
  const [componentAction, setComponentAction] = useState<string | null>(null);

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

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);

    try {
      await invoke("save_settings", { settings });
      setSuccess("Settings saved successfully!");
      onSettingsChanged?.();
      setTimeout(() => onClose(), 2000);
    } catch (e) {
      setError(`Failed to save settings: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleUninstall = async (componentKey: string) => {
    if (componentKey === "php" || componentKey === "mysql" || componentKey === "mariadb") {
      const msg = componentKey === "php"
        ? "PHP is required by Caddy for serving PHP files and phpMyAdmin. Uninstall anyway?"
        : `${getDatabaseDisplayName()} is required by phpMyAdmin. Uninstall anyway?`;
      if (!confirm(msg)) return;
    }

    // Prevent uninstalling last component
    const installedCount = components.filter(c => c.installed).length;
    if (installedCount <= 1) {
      alert("At least one component must remain installed.");
      return;
    }

    const actionKey = `uninstall-${componentKey}`;
    setComponentAction(actionKey);
    try {
      await invoke("uninstall_component", { component: componentKey });
      await refreshComponents();
    } catch (e) {
      alert(`Failed to uninstall: ${e}`);
    } finally {
      setComponentAction(null);
    }
  };

  const handleReinstall = async (componentKey: string) => {
    const actionKey = `reinstall-${componentKey}`;
    setComponentAction(actionKey);
    try {
      const settings = await invoke<AppSettings>("get_settings");
      const packageSelection: PackageSelection = settings.package_selection || {
        php: "php-8.5",
        mysql: "mysql-8.4",
        mariadb: "mariadb-12.3",
        phpmyadmin: "phpmyadmin-5.2",
      };

      // Download only this component by skipping all others
      const allComponents = ["caddy", "php", "mysql", "mariadb", "phpmyadmin"];
      const skipList = allComponents.filter(c => c !== componentKey);

      await invoke("download_runtime_with_skip", {
        packageSelection,
        skipList,
      });
      await refreshComponents();
    } catch (e) {
      alert(`Failed to reinstall: ${e}`);
    } finally {
      setComponentAction(null);
    }
  };

  const handleInstall = async (componentKey: string) => {
    await handleReinstall(componentKey);
  };

  if (loading) {
    return (
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
      >
        <div
          style={{
            backgroundColor: "var(--bg-card)",
            borderRadius: "0.75rem",
            boxShadow: "0 8px 32px rgba(0, 0, 0, 0.2)",
            maxWidth: "28rem",
            width: "100%",
            maxHeight: "90vh",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <div style={{ padding: "3rem", textAlign: "center", color: "var(--text-secondary)" }}>
            Loading settings...
          </div>
        </div>
      </div>
    );
  }

  return (
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
      onClick={onClose}
      {...props}
    >
      <div
        style={{
          backgroundColor: "var(--bg-card)",
          borderRadius: "0.75rem",
          boxShadow: "0 8px 32px rgba(0, 0, 0, 0.2)",
          width: "100%",
          maxWidth: "28rem",
          maxHeight: "90vh",
          display: "flex",
          flexDirection: "column",
          animation: "slide-in 0.2s ease-out",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "1.25rem 1.5rem",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          <h2 style={{ fontSize: "1.25rem", fontWeight: 600, margin: 0 }}>Settings</h2>
          <button
            style={{
              backgroundColor: "transparent",
              border: "none",
              fontSize: "1.5rem",
              color: "var(--text-secondary)",
              cursor: "pointer",
              padding: 0,
              lineHeight: 1,
            }}
            onClick={onClose}
            onMouseEnter={(e) => { e.currentTarget.style.color = "var(--color-error)"; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = "var(--text-secondary)"; }}
          >
            ×
          </button>
        </div>

        {/* Content */}
        <div style={{ padding: "1.5rem", overflowY: "auto", flex: 1 }}>
          {error && (
            <div className="error-box" style={{ marginBottom: "1rem" }}>
              <p className="error-box-text" style={{ margin: "0 0 0.5rem 0" }}>{error}</p>
            </div>
          )}
          {success && (
            <div className="success-box" style={{ marginBottom: "1rem" }}>
              <p className="success-box-text" style={{ margin: 0 }}>{success}</p>
            </div>
          )}

          {/* Port Configuration Section */}
          <div style={{ marginBottom: "1.5rem" }}>
            <h3 style={{ fontSize: "1rem", fontWeight: 600, marginBottom: "0.5rem" }}>Port Configuration</h3>
            <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", marginBottom: "1rem" }}>
              Configure the ports used by each service. Changes will be applied when services restart.
            </p>

            {/* Web Server Port */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: "0.75rem",
                padding: "0.5rem",
                borderRadius: "0.5rem",
                opacity: components.find(c => c.key === "caddy")?.installed ? 1 : 0.5,
                transition: "background-color 0.15s",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "var(--bg-card-secondary)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
            >
              <label htmlFor="web-port" style={{ fontSize: "0.875rem", fontWeight: 500 }}>
                Web Server Port
              </label>
              <input
                id="web-port"
                type="number"
                value={settings.web_port}
                onChange={(e) => handlePortChange("web_port", e.target.value)}
                min={1}
                max={65535}
                disabled={!components.find(c => c.key === "caddy")?.installed}
                className="input"
                style={{ width: "180px" }}
              />
            </div>

            {/* PHP-FPM Port */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: "0.75rem",
                padding: "0.5rem",
                borderRadius: "0.5rem",
                opacity: components.find(c => c.key === "php")?.installed ? 1 : 0.5,
                transition: "background-color 0.15s",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "var(--bg-card-secondary)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
            >
              <label htmlFor="php-port" style={{ fontSize: "0.875rem", fontWeight: 500 }}>
                PHP-FPM Port
              </label>
              <input
                id="php-port"
                type="number"
                value={settings.php_port}
                onChange={(e) => handlePortChange("php_port", e.target.value)}
                min={1}
                max={65535}
                disabled={!components.find(c => c.key === "php")?.installed}
                className="input"
                style={{ width: "180px" }}
              />
            </div>

            {/* MySQL/MariaDB Port */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "0.5rem",
                borderRadius: "0.5rem",
                opacity: components.find(c => c.key === dbComponentKey)?.installed ? 1 : 0.5,
                transition: "background-color 0.15s",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "var(--bg-card-secondary)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
            >
              <label htmlFor="mysql-port" style={{ fontSize: "0.875rem", fontWeight: 500 }}>
                {getDatabaseDisplayName()} Port
              </label>
              <input
                id="mysql-port"
                type="number"
                value={settings.mysql_port}
                onChange={(e) => handlePortChange("mysql_port", e.target.value)}
                min={1}
                max={65535}
                disabled={!components.find(c => c.key === dbComponentKey)?.installed}
                className="input"
                style={{ width: "180px" }}
              />
            </div>
          </div>

          {/* Components Section */}
          <div style={{ marginBottom: "1.5rem" }}>
            <h3 style={{ fontSize: "1rem", fontWeight: 600, marginBottom: "0.5rem" }}>Components</h3>
            <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", marginBottom: "0.75rem" }}>
              Manage installed software components.
            </p>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
              {components.map((comp) => {
                const isActing = componentAction === `uninstall-${comp.key}` || componentAction === `reinstall-${comp.key}`;
                const installedCount = components.filter(c => c.installed).length;
                const canUninstall = comp.installed && installedCount > 1;

                return (
                  <div
                    key={comp.key}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "0.5rem",
                      borderRadius: "0.5rem",
                      backgroundColor: "var(--bg-card-secondary)",
                      border: "1px solid var(--border-color)",
                    }}
                  >
                    <div style={{ display: "flex", flexDirection: "column", gap: "0.125rem" }}>
                      <span style={{ fontSize: "0.875rem", fontWeight: 500 }}>{comp.displayName}</span>
                      {comp.installed && comp.version ? (
                        <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)" }}>v{comp.version}</span>
                      ) : (
                        <span style={{ fontSize: "0.75rem", color: "var(--color-warning)", fontStyle: "italic" }}>Not installed</span>
                      )}
                    </div>
                    <div style={{ display: "flex", gap: "0.375rem", alignItems: "center" }}>
                      {isActing && (
                        <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)" }}>Working...</span>
                      )}
                      {comp.installed ? (
                        <>
                          <button
                            onClick={() => handleReinstall(comp.key)}
                            disabled={!!componentAction}
                            className="btn-secondary"
                            style={{ fontSize: "0.75rem", padding: "0.25rem 0.5rem" }}
                          >
                            Reinstall
                          </button>
                          <button
                            onClick={() => handleUninstall(comp.key)}
                            disabled={!!componentAction || !canUninstall}
                            className="btn-secondary"
                            title={!canUninstall ? "At least one component must remain installed" : undefined}
                            style={{
                              fontSize: "0.75rem",
                              padding: "0.25rem 0.5rem",
                              borderColor: "var(--color-error)",
                              color: "var(--color-error)",
                              opacity: !canUninstall ? 0.4 : 1,
                            }}
                          >
                            Uninstall
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => handleInstall(comp.key)}
                          disabled={!!componentAction}
                          className="btn-primary"
                          style={{ fontSize: "0.75rem", padding: "0.25rem 0.5rem" }}
                        >
                          Install
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: "0.75rem",
            padding: "1rem 1.5rem",
            borderTop: "1px solid var(--border-color)",
          }}
        >
          <button
            className="btn-secondary"
            onClick={onClose}
            disabled={saving}
          >
            Cancel
          </button>
          <button
            className="btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
