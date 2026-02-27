/**
 * Service types for CAMPP application
 */

export enum ServiceType {
  Caddy = "caddy",
  PhpFpm = "php-fpm",
  MariaDB = "mariadb",
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
  webPort: number;
  mysqlPort: number;
  phpPort: number;
  projectRoot: string;
}

export interface DownloadProgress {
  step: "downloading" | "extracting" | "installing" | "complete";
  percent: number;
  currentComponent: string;
  totalComponents: number;
}

export const DEFAULT_PORTS = {
  [ServiceType.Caddy]: 8080,
  [ServiceType.PhpFpm]: 9000,
  [ServiceType.MariaDB]: 3307,
} as const;

export const SERVICE_DISPLAY_NAMES = {
  [ServiceType.Caddy]: "Caddy",
  [ServiceType.PhpFpm]: "PHP-FPM 8.3",
  [ServiceType.MariaDB]: "MariaDB",
} as const;

export const SERVICE_DESCRIPTIONS = {
  [ServiceType.Caddy]: "Web Server",
  [ServiceType.PhpFpm]: "PHP Runtime",
  [ServiceType.MariaDB]: "Database Server",
} as const;
