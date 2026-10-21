import { useState, useEffect } from "react";
import { PasswordInput } from "./PasswordInput";
import { invoke } from "@tauri-apps/api/core";
import { PackagesConfig, PackageSelection, PhpPackage, MySQLPackage, PhpMyAdminPackage, getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";
import { LOGOS } from "../assets/logos";

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
      ngrok: "ngrok-3",
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
      ngrok: false,
    }
  );

  useEffect(() => {
    loadPackages();
    setCurrentPlatform(detectPlatform());
  }, []);

  useEffect(() => {
    if (initialEnabled) {
      setEnabled(initialEnabled);
    }
  }, [initialEnabled]);

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
    if (component === "caddy" || component === "php") return; // Caddy and PHP are always required
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
      <div className="wiz-loading">
        <span className="spinner" /> Loading available packages...
      </div>
    );
  }

  if (!packages) {
    return (
      <div className="error-box error-box-text">
        Failed to load available packages
      </div>
    );
  }

  const enabledCount = Object.values(enabled).filter(Boolean).length;
  const dbName = getDatabaseDisplayName(currentPlatform);
  const dbKey = currentPlatform === "linux" ? "mariadb" : "mysql";

  type Opt = { id: string; display_name: string; eol?: boolean; lts?: boolean; recommended?: boolean };
  const optLabel = (pkg: Opt) =>
    `${pkg.display_name}${pkg.eol ? " (EOL)" : ""}${pkg.lts ? " (LTS)" : ""}${pkg.recommended ? " ★" : ""}`;

  const rows: {
    key: string;
    name: string;
    tag?: string;
    required?: boolean;
    options?: Opt[];
    value?: string;
    onChange?: (v: string) => void;
    password?: { value: string; onChange?: (v: string) => void };
  }[] = [
    { key: "caddy", name: "Caddy", tag: "Web server", required: true },
    { key: "php", name: "PHP", tag: "PHP-FPM", required: true, options: packages.php as PhpPackage[], value: selection.php, onChange: handlePhpChange },
    {
      key: dbKey,
      name: dbName,
      tag: "Database",
      options: (dbKey === "mariadb" ? packages.mariadb : packages.mysql) as MySQLPackage[],
      value: dbKey === "mariadb" ? selection.mariadb : selection.mysql,
      onChange: (v) => setSelection({ ...selection, [dbKey]: v }),
      password: { value: mysqlPassword || "", onChange: onMysqlPasswordChange },
    },
    { key: "phpmyadmin", name: "phpMyAdmin", tag: "DB admin", options: packages.phpmyadmin as PhpMyAdminPackage[], value: selection.phpmyadmin, onChange: handlePhpMyAdminChange },
    {
      key: "postgresql",
      name: "PostgreSQL",
      tag: "Database",
      options: packages.postgresql as MySQLPackage[],
      value: selection.postgresql,
      onChange: (v) => setSelection({ ...selection, postgresql: v }),
      password: { value: postgresPassword || "", onChange: onPostgresPasswordChange },
    },
    { key: "adminer", name: "Adminer", tag: "DB admin", options: packages.adminer as PhpMyAdminPackage[], value: selection.adminer, onChange: (v) => setSelection({ ...selection, adminer: v }) },
    // ngrok — Windows only (upcoming on Linux/macOS)
    ...(currentPlatform === "windows"
      ? [{ key: "ngrok", name: "ngrok", tag: "Dev tunnel", options: (packages.ngrok || []) as MySQLPackage[], value: selection.ngrok, onChange: (v: string) => setSelection({ ...selection, ngrok: v }) }]
      : []),
  ];

  return (
    <div className="wiz-stack">
      <div className="list-box">
        {rows.map((r) => {
          const on = r.required || !!enabled[r.key];
          return (
            <div key={r.key} className={`list-row wiz-pkg${on ? "" : " wiz-pkg--off"}`}>
              <span className="logo-tile logo-tile--sm"><img src={LOGOS[r.key]} alt="" /></span>
              <div className="list-row__main">
                <span>{r.name}</span>
                <span className="wiz-muted">{r.required ? "Required" : r.tag}</span>
                {r.password && (
                  <PasswordInput
                    aria-label={`${r.name} root password`}
                    value={r.password.value}
                    onChange={(e) => r.password!.onChange?.(e.target.value)}
                    placeholder="Root password"
                    disabled={!on}
                    className="wiz-pkg__pw"
                  />
                )}
              </div>
              <div className="wiz-pkg__controls">
                {r.options ? (
                  <select
                    aria-label={`${r.name} version`}
                    value={r.value}
                    onChange={(e) => r.onChange?.(e.target.value)}
                    disabled={!on}
                    className="input wiz-pkg__select"
                  >
                    {r.options.map((pkg) => (
                      <option key={pkg.id} value={pkg.id}>{optLabel(pkg)}</option>
                    ))}
                  </select>
                ) : (
                  <span className="wiz-muted mono">v2.11.3</span>
                )}
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={on}
                aria-label={`Install ${r.name}`}
                className={`switch${r.required ? " switch--locked" : ""}`}
                disabled={r.required}
                title={r.required ? "Required, cannot be turned off" : undefined}
                onClick={() => handleToggle(r.key, !on)}
              >
                <span className="switch__knob" />
              </button>
            </div>
          );
        })}
      </div>

      <p className="wiz-muted wiz-small">
        {enabledCount} of {Object.keys(enabled).length} components selected · ★ recommended · EOL versions may have security vulnerabilities.
      </p>
    </div>
  );
}
