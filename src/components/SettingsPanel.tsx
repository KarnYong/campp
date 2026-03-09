import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { AppSettings } from "../types/services";

interface SettingsPanelProps {
  onClose: () => void;
  onSettingsChanged?: () => void;
}

interface PortStatus {
  available: boolean;
  checking: boolean;
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
  const [portStatus, setPortStatus] = useState<{
    web: PortStatus;
    php: PortStatus;
    mysql: PortStatus;
  }>({
    web: { available: true, checking: false },
    php: { available: true, checking: false },
    mysql: { available: true, checking: false },
  });

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

  const checkPort = async (type: 'web' | 'php' | 'mysql') => {
    setPortStatus(prev => ({
      ...prev,
      [type]: { available: false, checking: true }
    }));

    try {
      const result = await invoke<any>("check_ports", {
        webPort: settings.web_port,
        phpPort: settings.php_port,
        mysqlPort: settings.mysql_port
      });

      setPortStatus(prev => ({
        ...prev,
        [type]: { available: result[type].available, checking: false }
      }));
    } catch {
      setPortStatus(prev => ({
        ...prev,
        [type]: { available: true, checking: false }
      }));
    }
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      checkPort('web');
      checkPort('php');
      checkPort('mysql');
    }, 500);
    return () => clearTimeout(timer);
  }, [settings.web_port, settings.php_port, settings.mysql_port]);

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
      setSuccess("Settings saved! Services have been restarted with new configuration.");
      onSettingsChanged?.();
      setTimeout(() => onClose(), 2000);
    } catch (e) {
      setError(`Failed to save settings: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const renderPortStatus = (type: 'web' | 'php' | 'mysql') => {
    const status = portStatus[type];
    if (status.checking) {
      return (
        <span className="port-status">
          <span className="port-status-dot" style={{ background: 'var(--color-warning)' }}></span>
          Checking...
        </span>
      );
    }
    return (
      <span className={`port-status ${status.available ? 'port-status-available' : 'port-status-unavailable'}`}>
        <span className="port-status-dot"></span>
        {status.available ? 'Available' : 'In use'}
      </span>
    );
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
          <h2>⚙️ Settings</h2>
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
              Configure the ports used by each service. Services will automatically restart when you save.
            </p>

            <div className="settings-row">
              <div>
                <label htmlFor="web-port">Web Server Port</label>
                {renderPortStatus('web')}
              </div>
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
              <div>
                <label htmlFor="php-port">PHP-FPM Port</label>
                {renderPortStatus('php')}
              </div>
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
              <div>
                <label htmlFor="mysql-port">MariaDB Port</label>
                {renderPortStatus('mysql')}
              </div>
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
        </div>

        <div className="settings-footer">
          <button className="btn-cancel" onClick={onClose} disabled={saving}>
            Cancel
          </button>
          <button
            className="btn-save"
            onClick={handleSave}
            disabled={saving || !portStatus.web.available || !portStatus.php.available || !portStatus.mysql.available}
          >
            {saving ? "Saving..." : "Save & Restart Services"}
          </button>
        </div>
      </div>
    </div>
  );
}
