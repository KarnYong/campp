import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PackagesConfig, PackageSelection, PhpPackage, MySQLPackage, PhpMyAdminPackage, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";

interface PackageSelectorProps {
  onSelectionChange: (selection: PackageSelection) => void;
  initialSelection?: PackageSelection;
  initialEnabled?: Record<string, boolean>;
  onEnabledChange?: (enabled: Record<string, boolean>) => void;
  mysqlPassword?: string;
  onMysqlPasswordChange?: (password: string) => void;
  postgresPassword?: string;
  onPostgresPasswordChange?: (password: string) => void;
}

export function PackageSelector({ onSelectionChange, initialSelection, initialEnabled, onEnabledChange, mysqlPassword, onMysqlPasswordChange, postgresPassword, onPostgresPasswordChange }: PackageSelectorProps) {
  const [packages, setPackages] = useState<PackagesConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [currentPlatform, setCurrentPlatform] = useState<string>("");
  const [selection, setSelection] = useState<PackageSelection>(
    initialSelection || {
      php: "php-8.4",
      mysql: "mysql-8.4",
      mariadb: "mariadb-12.3",
      phpmyadmin: "phpmyadmin-5.2",
      postgresql: "postgresql-18.3",
      adminer: "adminer-5.1",
      pgvector: "pgvector-0.8.2",
    }
  );
  const [enabled, setEnabled] = useState<Record<string, boolean>>(
    initialEnabled || {
      caddy: true,
      php: true,
      mysql: true,
      mariadb: true,
      phpmyadmin: true,
      postgresql: false,
      adminer: false,
      pgvector: false,
    }
  );

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
    alignItems: "center",
    gap: "0.375rem",
    opacity: isEnabled ? 1 : 0.5,
  });

  const selectStyle = (isEnabled: boolean): React.CSSProperties => ({
    padding: "0.25rem 0.375rem",
    fontSize: "0.8125rem",
    flex: 1,
    minWidth: 0,
    cursor: isEnabled ? "pointer" : "not-allowed",
  });

  const pwStyle = (isEnabled: boolean): React.CSSProperties => ({
    padding: "0.25rem 0.375rem",
    fontSize: "0.8125rem",
    width: "6.5rem",
    cursor: isEnabled ? "text" : "not-allowed",
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      {/* Caddy (always enabled) */}
      <div style={rowStyle(true)}>
        <input type="checkbox" checked={true} disabled={true} style={checkboxStyle(true)} />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          Caddy
        </label>
        <span style={{ fontSize: "0.7rem", color: "var(--text-secondary)", fontWeight: 400 }}>
          (Required)
        </span>
        <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)", marginLeft: "auto" }}>
          v2.11.3
        </span>
      </div>

      {/* PHP Version */}
      <div style={rowStyle(enabled.php)}>
        <input
          type="checkbox"
          checked={enabled.php}
          onChange={(e) => handleToggle("php", e.target.checked)}
          style={checkboxStyle(false)}
        />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          PHP
        </label>
        <select
          value={selection.php}
          onChange={(e) => handlePhpChange(e.target.value)}
          disabled={!enabled.php}
          className="input"
          style={selectStyle(enabled.php)}
        >
          {packages.php.map((pkg: PhpPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}{pkg.eol && " (EOL)"}{pkg.recommended && " ★"}
            </option>
          ))}
        </select>
      </div>

      {/* Database Version + Password */}
      <div style={rowStyle(isDbEnabled)}>
        <input
          type="checkbox"
          checked={isDbEnabled}
          onChange={(e) => handleToggle(dbKey, e.target.checked)}
          style={checkboxStyle(false)}
        />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          {dbName}
        </label>
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
          style={selectStyle(isDbEnabled)}
        >
          {(dbKey === "mariadb" ? packages.mariadb : packages.mysql).map((pkg: MySQLPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}{pkg.lts && " (LTS)"}{pkg.recommended && " ★"}
            </option>
          ))}
        </select>
        <input
          type="password"
          value={mysqlPassword || ""}
          onChange={(e) => onMysqlPasswordChange?.(e.target.value)}
          placeholder="Password"
          disabled={!isDbEnabled}
          className="input"
          style={pwStyle(isDbEnabled)}
        />
      </div>

      {/* phpMyAdmin Version */}
      <div style={rowStyle(enabled.phpmyadmin)}>
        <input
          type="checkbox"
          checked={enabled.phpmyadmin}
          onChange={(e) => handleToggle("phpmyadmin", e.target.checked)}
          style={checkboxStyle(false)}
        />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          phpMyAdmin
        </label>
        <select
          value={selection.phpmyadmin}
          onChange={(e) => handlePhpMyAdminChange(e.target.value)}
          disabled={!enabled.phpmyadmin}
          className="input"
          style={selectStyle(enabled.phpmyadmin)}
        >
          {packages.phpmyadmin.map((pkg: PhpMyAdminPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}{pkg.recommended && " ★"}
            </option>
          ))}
        </select>
      </div>

      {/* PostgreSQL Version + Password */}
      <div style={rowStyle(enabled.postgresql)}>
        <input
          type="checkbox"
          checked={enabled.postgresql}
          onChange={(e) => handleToggle("postgresql", e.target.checked)}
          style={checkboxStyle(false)}
        />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          PostgreSQL
        </label>
        <select
          value={selection.postgresql}
          onChange={(e) => setSelection({ ...selection, postgresql: e.target.value })}
          disabled={!enabled.postgresql}
          className="input"
          style={selectStyle(enabled.postgresql)}
        >
          {packages.postgresql.map((pkg: MySQLPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}{pkg.recommended && " ★"}
            </option>
          ))}
        </select>
        <input
          type="password"
          value={postgresPassword || ""}
          onChange={(e) => onPostgresPasswordChange?.(e.target.value)}
          placeholder="Password"
          disabled={!enabled.postgresql}
          className="input"
          style={pwStyle(enabled.postgresql)}
        />
      </div>

      {/* Adminer Version */}
      <div style={rowStyle(enabled.adminer)}>
        <input
          type="checkbox"
          checked={enabled.adminer}
          onChange={(e) => handleToggle("adminer", e.target.checked)}
          style={checkboxStyle(false)}
        />
        <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
          Adminer
        </label>
        <select
          value={selection.adminer}
          onChange={(e) => setSelection({ ...selection, adminer: e.target.value })}
          disabled={!enabled.adminer}
          className="input"
          style={selectStyle(enabled.adminer)}
        >
          {packages.adminer.map((pkg: PhpMyAdminPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}{pkg.recommended && " ★"}
            </option>
          ))}
        </select>
      </div>

      {/* pgvector (only shown when PostgreSQL is enabled) */}
      {enabled.postgresql && packages.pgvector && packages.pgvector.length > 0 && (
        <div style={rowStyle(enabled.pgvector)}>
          <input
            type="checkbox"
            checked={enabled.pgvector}
            onChange={(e) => handleToggle("pgvector", e.target.checked)}
            style={checkboxStyle(false)}
          />
          <label style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)" }}>
            pgvector
          </label>
          <select
            value={selection.pgvector}
            onChange={(e) => setSelection({ ...selection, pgvector: e.target.value })}
            disabled={!enabled.pgvector}
            className="input"
            style={selectStyle(enabled.pgvector)}
          >
            {packages.pgvector.map((pkg: MySQLPackage) => (
              <option key={pkg.id} value={pkg.id}>
                {pkg.display_name}{pkg.recommended && " ★"}
              </option>
            ))}
          </select>
          <span style={{ fontSize: "0.7rem", color: "var(--text-secondary)", fontWeight: 400 }}>
            (Ext)
          </span>
        </div>
      )}

      {/* Package Info Box */}
      <div className="info-box" style={{ padding: "0.5rem", fontSize: "0.875rem" }}>
        <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", margin: "0 0 0.375rem 0" }}>
          {enabledCount} of {Object.keys(enabled).length} components selected.
        </p>
        <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)", margin: 0 }}>
          <strong>Note:</strong> EOL versions may have security vulnerabilities.
        </p>
      </div>
    </div>
  );
}
