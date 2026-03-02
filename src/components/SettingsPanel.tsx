import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { AppSettings } from "../types/services";

interface SettingsPanelProps {
  onClose: () => void;
  onSettingsChanged?: () => void;
}

export function SettingsPanel({ onClose, onSettingsChanged }: SettingsPanelProps) {
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

  const handleOpenProjectFolder = async () => {
    try {
      await revealItemInDir(settings.project_root);
    } catch (e) {
      setError(`Failed to open folder: ${e}`);
    }
  };

  const handleSave = async () => {
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
    return (
      <div className="settings-panel-overlay">
        <div className="settings-panel">
          <div className="settings-loading">Loading settings...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="settings-panel-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2>Settings</h2>
          <button className="settings-close" onClick={onClose}>
            &times;
          </button>
        </div>

        <div className="settings-content">
          {error && <div className="settings-error">{error}</div>}
          {success && <div className="settings-success">{success}</div>}

          <div className="settings-section">
            <h3>Port Configuration</h3>
            <p className="settings-hint">
              Configure the ports used by each service. Changes require a service restart.
            </p>

            <div className="settings-row">
              <label htmlFor="web-port">Web Server Port</label>
              <input
                id="web-port"
                type="number"
                value={settings.web_port}
                onChange={(e) => handlePortChange("web_port", e.target.value)}
                min={1}
                max={65535}
              />
            </div>

            <div className="settings-row">
              <label htmlFor="php-port">PHP-FPM Port</label>
              <input
                id="php-port"
                type="number"
                value={settings.php_port}
                onChange={(e) => handlePortChange("php_port", e.target.value)}
                min={1}
                max={65535}
              />
            </div>

            <div className="settings-row">
              <label htmlFor="mysql-port">MariaDB Port</label>
              <input
                id="mysql-port"
                type="number"
                value={settings.mysql_port}
                onChange={(e) => handlePortChange("mysql_port", e.target.value)}
                min={1}
                max={65535}
              />
            </div>
          </div>

          <div className="settings-section">
            <h3>Project Settings</h3>

            <div className="settings-row">
              <label htmlFor="project-root">Project Root Directory</label>
              <div className="settings-input-with-button">
                <input
                  id="project-root"
                  type="text"
                  value={settings.project_root}
                  onChange={(e) =>
                    setSettings({ ...settings, project_root: e.target.value })
                  }
                  placeholder="C:\Users\...\AppData\Local\campp\projects"
                />
                <button
                  className="btn-open-folder"
                  onClick={handleOpenProjectFolder}
                  title="Open folder in Explorer"
                >
                  Open
                </button>
              </div>
            </div>
          </div>
        </div>

        <div className="settings-footer">
          <button className="btn-cancel" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn-save"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? "Saving..." : "Save Settings"}
          </button>
        </div>
      </div>
    </div>
  );
}
