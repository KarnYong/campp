/**
 * Service types for CAMPP application
 */

export enum ServiceType {
  Caddy = "caddy",
  PhpFpm = "php-fpm",
  MySQL = "mysql",
  PostgreSQL = "postgresql",
  Ngrok = "ngrok",
}

export enum ServiceState {
  Stopped = "stopped",
  Starting = "starting",
  Running = "running",
  Stopping = "stopping",
  Error = "error",
}

export interface ServiceInfo {
  service_type: ServiceType;
  state: ServiceState;
  port: number;
  error_message?: string;
}

export type ServiceMap = Record<ServiceType, ServiceInfo>;

export interface AppSettings {
  web_port: number;
  mysql_port: number;
  php_port: number;
  postgres_port: number;
  project_root: string;
  ngrok_authtoken?: string;
  mysql_root_password: string;
  postgres_root_password: string;
  package_selection?: PackageSelection;
  listen_all_interfaces?: boolean;
  /** UI theme: "system" | "light" | "dark". */
  theme?: string;
}

export interface DownloadProgress {
  step: "downloading" | "extracting" | "installing" | "complete" | "error";
  percent: number;
  currentComponent: string;
  componentDisplay: string;
  version: string;
  totalComponents: number;
  downloadedBytes: number;
  totalBytes: number;
  /** Non-fatal warning (e.g. ngrok skipped by antivirus). Optional. */
  warning?: string;
}

// Package selection types
export interface PhpPackage {
  id: string;
  version: string;
  display_name: string;
  windowsX64: string;
  windowsArm64: string;
  linuxX64: string;
  linuxArm64: string;
  macOSX64: string;
  macOSArm64: string;
  eol: boolean;
  lts: boolean;
  recommended: boolean;
}

export interface MySQLPackage {
  id: string;
  version: string;
  display_name: string;
  windowsX64: string;
  windowsArm64: string;
  linuxX64: string;
  linuxArm64: string;
  macOSX64: string;
  macOSArm64: string;
  eol: boolean;
  lts: boolean;
  recommended: boolean;
}

export interface PhpMyAdminPackage {
  id: string;
  version: string;
  display_name: string;
  url: string;
  eol: boolean;
  lts: boolean;
  recommended: boolean;
}

export interface PackagesConfig {
  php: PhpPackage[];
  mysql: MySQLPackage[];
  mariadb: MySQLPackage[];
  postgresql: MySQLPackage[];
  phpmyadmin: PhpMyAdminPackage[];
  adminer: PhpMyAdminPackage[];
  ngrok: MySQLPackage[];
  composer: PhpMyAdminPackage[];
}

export interface PackageSelection {
  php: string;
  mysql: string;
  mariadb: string;
  phpmyadmin: string;
  postgresql: string;
  adminer: string;
  ngrok: string;
  composer: string;
  /** Windows only: which engine fills the database slot. Absent = "mysql". */
  db_engine?: DbEngine;
}

export type DbEngine = "mysql" | "mariadb";

// The active Windows database engine, set wherever the package selection is loaded
// or changed. ponytail: module state so every label helper stays synchronous; move
// to context if components ever need to re-render on change without a reload.
let activeDbEngine: DbEngine = "mysql";
export const setActiveDbEngine = (engine: DbEngine | undefined) => {
  activeDbEngine = engine === "mariadb" ? "mariadb" : "mysql";
};

const detect = () => {
  const ua = window.navigator.userAgent.toLowerCase();
  if (ua.includes("win")) return "windows";
  if (ua.includes("mac")) return "darwin";
  return "linux";
};

/** Runtime component key of the database engine: Linux MariaDB, macOS MySQL, Windows the user's choice. */
export const dbKeyFor = (platform?: string, engine: DbEngine | undefined = activeDbEngine): DbEngine => {
  const p = platform || detect();
  if (p === "linux") return "mariadb";
  if (p === "windows" && engine === "mariadb") return "mariadb";
  return "mysql";
};

export const DEFAULT_PORTS = {
  [ServiceType.Caddy]: 8080,
  [ServiceType.PhpFpm]: 9000,
  [ServiceType.MySQL]: 3307,
  [ServiceType.PostgreSQL]: 5433,
  [ServiceType.Ngrok]: 4040,
} as const;

export const SERVICE_DISPLAY_NAMES = {
  [ServiceType.Caddy]: "Caddy",
  [ServiceType.PhpFpm]: "PHP-FPM",
  [ServiceType.MySQL]: "MariaDB",
  [ServiceType.PostgreSQL]: "PostgreSQL",
  [ServiceType.Ngrok]: "ngrok",
} as const;

// Display name of the active database engine (see dbKeyFor)
export const getDatabaseDisplayName = (platform?: string, engine?: DbEngine): string =>
  dbKeyFor(platform, engine ?? activeDbEngine) === "mariadb" ? "MariaDB" : "MySQL";

export const SERVICE_DESCRIPTIONS = {
  [ServiceType.Caddy]: "Web Server",
  [ServiceType.PhpFpm]: "PHP Runtime",
  [ServiceType.MySQL]: "Database Server",
  [ServiceType.PostgreSQL]: "Database Server",
  [ServiceType.Ngrok]: "Tunnel (Dev Only)",
} as const;

// System dependency types
export interface InstallCommand {
  distribution: string;
  command: string;
}

export interface Dependency {
  name: string;
  installed: boolean;
  description: string;
  install_commands: InstallCommand[];
}

export interface DependencyCheckResult {
  dependencies: Dependency[];
  all_satisfied: boolean;
  platform_notes: string;
}

export interface ComponentStatus {
  installed: boolean;
  version: string | null;
  serviceRunning: boolean;
}

export type ComponentStatusMap = Record<string, ComponentStatus>;
