/**
 * Service types for CAMPP application
 */

export enum ServiceType {
  Caddy = "caddy",
  PhpFpm = "php-fpm",
  MySQL = "mysql",
  PostgreSQL = "postgresql",
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
  mysql_root_password: string;
  postgres_root_password: string;
  package_selection?: PackageSelection;
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
}

export interface PackageSelection {
  php: string;
  mysql: string;
  mariadb: string;
  phpmyadmin: string;
  postgresql: string;
  adminer: string;
}

export const DEFAULT_PORTS = {
  [ServiceType.Caddy]: 8080,
  [ServiceType.PhpFpm]: 9000,
  [ServiceType.MySQL]: 3307,
  [ServiceType.PostgreSQL]: 5433,
} as const;

export const SERVICE_DISPLAY_NAMES = {
  [ServiceType.Caddy]: "Caddy",
  [ServiceType.PhpFpm]: "PHP-FPM",
  [ServiceType.MySQL]: "MariaDB",
  [ServiceType.PostgreSQL]: "PostgreSQL",
} as const;

// Platform-specific display name for MySQL/MariaDB
export const getDatabaseDisplayName = (platform?: string): string => {
  // Auto-detect platform if not provided
  const p = platform || (() => {
    const ua = window.navigator.userAgent.toLowerCase();
    if (ua.includes("win")) return "windows";
    if (ua.includes("mac")) return "darwin";
    return "linux";
  })();
  // Show "MySQL" for Windows and macOS, "MariaDB" for Linux
  if (p === "windows" || p === "darwin") {
    return "MySQL";
  }
  return "MariaDB";
};

export const SERVICE_DESCRIPTIONS = {
  [ServiceType.Caddy]: "Web Server",
  [ServiceType.PhpFpm]: "PHP Runtime",
  [ServiceType.MySQL]: "Database Server",
  [ServiceType.PostgreSQL]: "Database Server",
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
