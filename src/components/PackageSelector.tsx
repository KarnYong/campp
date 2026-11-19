import { useState, useEffect } from "react";
import { useCaddyVersion } from "../utils/useCaddyVersion";
import { PasswordInput } from "./PasswordInput";
import { invoke } from "@tauri-apps/api/core";
import { PackagesConfig, PackageSelection, PhpPackage, MySQLPackage, PhpMyAdminPackage, DbEngine, getDatabaseDisplayName, dbKeyFor } from "../types/services";
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
  const caddyVersion = useCaddyVersion();
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
      db_engine: "mysql",
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
  const dbName = getDatabaseDisplayName(currentPlatform, selection.db_engine);
  const dbKey = dbKeyFor(currentPlatform, selection.db_engine);
  const isWindows = currentPlatform === "windows";

  type Opt = { id: string; display_name: string; eol?: boolean; lts?: boolean; recommended?: boolean };
  const optLabel = (pkg: Opt) =>
    `${pkg.display_name}${pkg.eol ? " (EOL)" : ""}${pkg.lts ? " (LTS)" : ""}${pkg.recommended ? " ★" : ""}`;

  type Group = { label: string; prefix: string; options: Opt[] };
  const rows: {
    key: string;
    name: string;
    tag?: string;
    required?: boolean;
    options?: Opt[];
    /** Optgroups whose option values are `${prefix}:${id}` (Windows engine + version in one list). */
    groups?: Group[];
    value?: string;
    onChange?: (v: string) => void;
    password?: { value: string; onChange?: (v: string) => void };
  }[] = [
    { key: "caddy", name: "Caddy", tag: "Web server", required: true },
    { key: "php", name: "PHP", tag: "PHP-FPM", required: true, options: packages.php as PhpPackage[], value: selection.php, onChange: handlePhpChange },
    isWindows
      ? {
          key: dbKey,
          name: dbName,
          tag: "Database",
          groups: [
            { label: "MySQL", prefix: "mysql", options: packages.mysql as MySQLPackage[] },
            { label: "MariaDB", prefix: "mariadb", options: packages.mariadb as MySQLPackage[] },
          ],
          value: `${dbKey}:${selection[dbKey]}`,
          onChange: (v) => {
            const [engine, id] = v.split(":") as [DbEngine, string];
            setSelection({ ...selection, db_engine: engine, [engine]: id });
          },
          password: { value: mysqlPassword || "", onChange: onMysqlPasswordChange },
        }
      : {
          key: dbKey,
          name: dbName,
          tag: "Database",
          options: (dbKey === "mariadb" ? packages.mariadb : packages.mysql) as MySQLPackage[],
          value: selection[dbKey],
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
      <div className="list-box wiz-pkgs">
        {rows.map((r) => {
          const on = r.required || !!enabled[r.key];
          return (
            <div key={r.key} className={`wiz-pkg${on ? "" : " wiz-pkg--off"}`}>
              <button
                type="button"
                role="switch"
                aria-checked={on}
                aria-label={`Install ${r.name}`}
                className={`switch switch--sm${r.required ? " switch--locked" : ""}`}
                disabled={r.required}
                title={r.required ? "Required, cannot be turned off" : undefined}
                onClick={() => handleToggle(r.key, !on)}
              >
                <span className="switch__knob" />
              </button>
              <img className="wiz-pkg__logo" src={LOGOS[r.key]} alt="" />
              <div className="wiz-pkg__name">
                <span>{r.name}</span>
                <span className="wiz-muted">{r.required ? "Required" : r.tag}</span>
              </div>
              <div className="wiz-pkg__ver">
                {r.groups ? (
                  <select
                    aria-label={`${r.name} engine and version`}
                    value={r.value}
                    onChange={(e) => r.onChange?.(e.target.value)}
                    disabled={!on}
                    className="input wiz-pkg__select"
                    title="MySQL is the default. MariaDB is a drop-in alternative; each keeps its own databases."
                  >
                    {r.groups.map((g) => (
                      <optgroup key={g.prefix} label={g.label}>
                        {g.options.map((pkg) => (
                          <option key={pkg.id} value={`${g.prefix}:${pkg.id}`}>{optLabel(pkg)}</option>
                        ))}
                      </optgroup>
                    ))}
                  </select>
                ) : r.options ? (
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
                  <span className="wiz-muted mono">{caddyVersion ? `v${caddyVersion}` : "…"}</span>
                )}
              </div>
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
          );
        })}
      </div>

      <p className="wiz-muted wiz-small">
        {enabledCount}/{Object.keys(enabled).length} selected · ★ recommended · EOL = no more security fixes
      </p>
    </div>
  );
}
