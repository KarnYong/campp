import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { AppSettings } from "../types/services";

interface SettingsPanelProps {
  onClose: () => void;
  onSettingsChanged?: () => void;
  [key: string]: any; // Allow additional props like data-testid
}

export function SettingsPanel({ onClose, onSettingsChanged, ...props }: SettingsPanelProps) {
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

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

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
          <h2 style={{ fontSize: "1.25rem", fontWeight: 600, margin: 0 }}>⚙️ Settings</h2>
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
                className="input"
                style={{ width: "180px" }}
              />
            </div>

            {/* MySQL Port */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "0.5rem",
                borderRadius: "0.5rem",
                transition: "background-color 0.15s",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "var(--bg-card-secondary)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
            >
              <label htmlFor="mysql-port" style={{ fontSize: "0.875rem", fontWeight: 500 }}>
                MySQL Port
              </label>
              <input
                id="mysql-port"
                type="number"
                value={settings.mysql_port}
                onChange={(e) => handlePortChange("mysql_port", e.target.value)}
                min={1}
                max={65535}
                className="input"
                style={{ width: "180px" }}
              />
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
