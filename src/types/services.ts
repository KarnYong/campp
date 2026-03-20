/**
 * Service types for CAMPP application
 */

export enum ServiceType {
  Caddy = "caddy",
  PhpFpm = "php-fpm",
  MySQL = "mysql",
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
  project_root: string;
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
  phpmyadmin: PhpMyAdminPackage[];
}

export interface PackageSelection {
  php: string;
  mysql: string;
  phpmyadmin: string;
}

export const DEFAULT_PORTS = {
  [ServiceType.Caddy]: 8080,
  [ServiceType.PhpFpm]: 9000,
  [ServiceType.MySQL]: 3307,
} as const;

export const SERVICE_DISPLAY_NAMES = {
  [ServiceType.Caddy]: "Caddy",
  [ServiceType.PhpFpm]: "PHP-FPM",
  [ServiceType.MySQL]: "MariaDB",
} as const;

export const SERVICE_DESCRIPTIONS = {
  [ServiceType.Caddy]: "Web Server",
  [ServiceType.PhpFpm]: "PHP Runtime",
  [ServiceType.MySQL]: "Database Server",
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
