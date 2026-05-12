import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PackagesConfig, PackageSelection, PhpPackage, MySQLPackage, PhpMyAdminPackage, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";

interface PackageSelectorProps {
  onSelectionChange: (selection: PackageSelection) => void;
  initialSelection?: PackageSelection;
  onEnabledChange?: (enabled: Record<string, boolean>) => void;
}

export function PackageSelector({ onSelectionChange, initialSelection, onEnabledChange }: PackageSelectorProps) {
  const [packages, setPackages] = useState<PackagesConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [currentPlatform, setCurrentPlatform] = useState<string>("");
  const [selection, setSelection] = useState<PackageSelection>(
    initialSelection || {
      php: "php-8.4",
      mysql: "mysql-8.4",
      mariadb: "mariadb-12.3",
      phpmyadmin: "phpmyadmin-5.2",
    }
  );
  const [enabled, setEnabled] = useState<Record<string, boolean>>({
    caddy: true,
    php: true,
    mysql: true,
    mariadb: true,
    phpmyadmin: true,
  });

  useEffect(() => {
    loadPackages();
    setCurrentPlatform(detectPlatform());
  }, []);

  useEffect(() => {
    if (packages) {
      onSelectionChange(selection);
    }
  }, [selection, packages]);

  useEffect(() => {
    onEnabledChange?.(enabled);
  }, [enabled]);

  const loadPackages = async () => {
    try {
      const data = await invoke<PackagesConfig>("get_available_packages_cmd");
      setPackages(data);
    } catch (err) {
      console.error("Failed to load packages:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = (component: string, checked: boolean) => {
    if (component === "caddy") return; // Caddy is always required
    const newEnabled = { ...enabled, [component]: checked };
    setEnabled(newEnabled);
  };

  const handlePhpChange = (value: string) => {
    setSelection({ ...selection, php: value });
  };

  const handlePhpMyAdminChange = (value: string) => {
    setSelection({ ...selection, phpmyadmin: value });
  };

  if (loading) {
    return (
      <div style={{ textAlign: "center", color: "var(--text-secondary)", fontSize: "0.875rem", padding: "1rem" }}>
        Loading available packages...
      </div>
    );
  }

  if (!packages) {
    return (
      <div style={{ textAlign: "center", color: "var(--color-error)", fontSize: "0.875rem" }}>
        Failed to load available packages
      </div>
    );
  }

  const enabledCount = Object.values(enabled).filter(Boolean).length;
  const dbName = getDatabaseDisplayName(currentPlatform);
  const dbKey = currentPlatform === "linux" ? "mariadb" : "mysql";
  const isDbEnabled = enabled[dbKey];

  const checkboxStyle = (isDisabled: boolean): React.CSSProperties => ({
    width: "1rem",
    height: "1rem",
    cursor: isDisabled ? "default" : "pointer",
    accentColor: "var(--color-primary)",
  });

  const rowStyle = (isEnabled: boolean): React.CSSProperties => ({
    display: "flex",
    flexDirection: "column",
    gap: "0.25rem",
    opacity: isEnabled ? 1 : 0.5,
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
      {/* Caddy (always enabled) */}
      <div style={rowStyle(true)}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input
            type="checkbox"
            checked={true}
            disabled={true}
            style={checkboxStyle(true)}
          />
          <label style={{ fontSize: "0.875rem", fontWeight: 500, color: "var(--text-primary)" }}>
            Caddy Web Server
          </label>
          <span style={{ fontSize: "0.7rem", color: "var(--text-secondary)", fontWeight: 400 }}>
            (Required)
          </span>
          <span style={{
            fontSize: "0.75rem",
            color: "var(--text-secondary)",
            marginLeft: "auto",
          }}>
            v2.8.4
          </span>
        </div>
      </div>

      {/* PHP Version Selector */}
      <div style={rowStyle(enabled.php)}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input
            type="checkbox"
            checked={enabled.php}
            onChange={(e) => handleToggle("php", e.target.checked)}
            style={checkboxStyle(false)}
          />
          <label style={{ fontSize: "0.875rem", fontWeight: 500, color: "var(--text-primary)" }}>
            PHP Version
          </label>
        </div>
        <select
          value={selection.php}
          onChange={(e) => handlePhpChange(e.target.value)}
          disabled={!enabled.php}
          className="input"
          style={{ cursor: enabled.php ? "pointer" : "not-allowed", padding: "0.375rem 0.5rem", fontSize: "0.875rem", width: "100%" }}
        >
          {packages.php.map((pkg: PhpPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.eol && " (EOL)"}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      {/* Database Version Selector */}
      <div style={rowStyle(isDbEnabled)}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input
            type="checkbox"
            checked={isDbEnabled}
            onChange={(e) => handleToggle(dbKey, e.target.checked)}
            style={checkboxStyle(dbKey === "mariadb" ? false : false)}
          />
          <label style={{ fontSize: "0.875rem", fontWeight: 500, color: "var(--text-primary)" }}>
            {dbName} Version
          </label>
        </div>
        <select
          value={dbKey === "mariadb" ? selection.mariadb : selection.mysql}
          onChange={(e) => {
            if (dbKey === "mariadb") {
              setSelection({ ...selection, mariadb: e.target.value });
            } else {
              setSelection({ ...selection, mysql: e.target.value });
            }
          }}
          disabled={!isDbEnabled}
          className="input"
          style={{ cursor: isDbEnabled ? "pointer" : "not-allowed", padding: "0.375rem 0.5rem", fontSize: "0.875rem", width: "100%" }}
        >
          {(dbKey === "mariadb" ? packages.mariadb : packages.mysql).map((pkg: MySQLPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.lts && " (LTS)"}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      {/* phpMyAdmin Version Selector */}
      <div style={rowStyle(enabled.phpmyadmin)}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input
            type="checkbox"
            checked={enabled.phpmyadmin}
            onChange={(e) => handleToggle("phpmyadmin", e.target.checked)}
            style={checkboxStyle(false)}
          />
          <label style={{ fontSize: "0.875rem", fontWeight: 500, color: "var(--text-primary)" }}>
            phpMyAdmin Version
          </label>
        </div>
        <select
          value={selection.phpmyadmin}
          onChange={(e) => handlePhpMyAdminChange(e.target.value)}
          disabled={!enabled.phpmyadmin}
          className="input"
          style={{ cursor: enabled.phpmyadmin ? "pointer" : "not-allowed", padding: "0.375rem 0.5rem", fontSize: "0.875rem", width: "100%" }}
        >
          {packages.phpmyadmin.map((pkg: PhpMyAdminPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      {/* Package Info Box */}
      <div className="info-box" style={{ padding: "0.5rem", fontSize: "0.875rem" }}>
        <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", margin: "0 0 0.375rem 0" }}>
          {enabledCount} of 4 components selected.
        </p>
        <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", margin: 0 }}>
          <strong>Note:</strong> EOL versions may have security vulnerabilities.
        </p>
      </div>
    </div>
  );
}
