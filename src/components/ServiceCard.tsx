import type { CSSProperties } from "react";
import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS, getDatabaseDisplayName } from "../types/services";
import { Icon, type IconName } from "./icons";

// One icon per service type for visual scanning.
const SERVICE_ICON: Record<ServiceType, IconName> = {
  [ServiceType.Caddy]: "globe",
  [ServiceType.PhpFpm]: "code",
  [ServiceType.MySQL]: "database",
  [ServiceType.PostgreSQL]: "layers",
  [ServiceType.Ngrok]: "share",
};

// Semantic color class per state (drives styling + is asserted by tests).
const STATUS_COLOR_CLASS: Record<ServiceState, string> = {
  [ServiceState.Stopped]: "status-gray",
  [ServiceState.Starting]: "status-blue",
  [ServiceState.Running]: "status-green",
  [ServiceState.Stopping]: "status-orange",
  [ServiceState.Error]: "status-red",
};

interface ServiceCardProps {
  serviceType: ServiceType;
  state: ServiceState;
  port?: number;
  error?: string;
  notInstalled?: boolean;
  tunnelUrl?: string | null;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  [key: string]: any; // Allow additional props like data-testid
}

export function ServiceCard({
  serviceType,
  state,
  port = DEFAULT_PORTS[serviceType],
  error,
  notInstalled = false,
  tunnelUrl,
  onStart,
  onStop,
  onRestart,
  ...props
}: ServiceCardProps) {
  const displayName = serviceType === ServiceType.MySQL
    ? getDatabaseDisplayName()
    : SERVICE_DISPLAY_NAMES[serviceType];
  const description = SERVICE_DESCRIPTIONS[serviceType];

  const isRunning = state === ServiceState.Running;
  const isTransitioning = state === ServiceState.Starting || state === ServiceState.Stopping;
  const isError = state === ServiceState.Error;

  const getStatusStyles = () => {
    switch (state) {
      case ServiceState.Stopped:
        return { bg: "var(--status-stopped-bg)", text: "var(--status-stopped-text)" };
      case ServiceState.Starting:
        return { bg: "var(--status-starting-bg)", text: "var(--status-starting-text)" };
      case ServiceState.Running:
        return { bg: "var(--status-running-bg)", text: "var(--status-running-text)" };
      case ServiceState.Stopping:
        return { bg: "var(--status-stopping-bg)", text: "var(--status-stopping-text)" };
      case ServiceState.Error:
        return { bg: "var(--status-error-bg)", text: "var(--status-error-text)" };
      default:
        return { bg: "var(--status-stopped-bg)", text: "var(--status-stopped-text)" };
    }
  };

  const statusStyles = getStatusStyles();

  return (
    <div
      className={`service-card${notInstalled ? "" : " card-hover"}`}
      style={{
        ["--status-accent" as string]: notInstalled ? "transparent" : statusStyles.text,
        backgroundColor: isError ? "var(--error-box-bg)" : "var(--bg-card)",
        border: `1px solid ${isError ? "var(--error-box-border)" : "var(--border-color)"}`,
        borderRadius: "0.5rem",
        padding: "1rem",
        boxShadow: "0 1px 3px rgba(0, 0, 0, 0.1)",
        opacity: notInstalled ? 0.5 : 1,
      } as CSSProperties}
      data-testid={`service-card-${serviceType}`}
      {...props}
    >
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span className="service-icon">
            <Icon name={SERVICE_ICON[serviceType]} size={16} />
          </span>
          <h3 style={{ fontSize: "1rem", fontWeight: 600, margin: 0 }}>{displayName}</h3>
        </div>
        <span
          className={notInstalled ? "status-gray" : STATUS_COLOR_CLASS[state]}
          style={{
            padding: "0.125rem 0.5rem",
            borderRadius: "9999px",
            fontSize: "0.65rem",
            fontWeight: 600,
            textTransform: "uppercase",
            backgroundColor: notInstalled ? "var(--bg-card-secondary)" : statusStyles.bg,
            color: notInstalled ? "var(--text-secondary)" : statusStyles.text,
          }}
          data-testid={`service-state-${serviceType}`}
        >
          {notInstalled ? "Not Installed" : state}
        </span>
      </div>

      {/* Description */}
      <p style={{ fontSize: "0.75rem", color: "var(--text-secondary)", margin: "0 0 0.5rem 0" }}>
        {description}
      </p>

      {/* Port Info */}
      <div
        style={{
          display: "flex",
          gap: "0.75rem",
          padding: "0.5rem 0",
          borderTop: "1px solid var(--border-color)",
          borderBottom: "1px solid var(--border-color)",
          fontSize: "0.75rem",
          color: "var(--text-secondary)",
        }}
      >
        <span>Port: {port}</span>
      </div>

      {/* Tunnel URL (ngrok only) */}
      {serviceType === ServiceType.Ngrok && isRunning && (
        <div style={{ marginTop: "0.5rem" }}>
          {tunnelUrl ? (
            <div style={{
              padding: "0.5rem",
              backgroundColor: "rgba(16, 185, 129, 0.1)",
              border: "1px solid rgba(16, 185, 129, 0.3)",
              borderRadius: "0.375rem",
              fontSize: "0.75rem",
              wordBreak: "break-all",
            }}>
              <strong style={{ color: "var(--text-primary)" }}>Public URL:</strong>{" "}
              <a href={tunnelUrl} target="_blank" rel="noopener noreferrer"
                style={{ color: "var(--color-primary, #3b82f6)", textDecoration: "underline" }}>
                {tunnelUrl}
              </a>
            </div>
          ) : (
            <div style={{
              padding: "0.5rem",
              backgroundColor: "var(--bg-card-secondary)",
              borderRadius: "0.375rem",
              fontSize: "0.75rem",
              color: "var(--text-secondary)",
              fontStyle: "italic",
            }}>
              Connecting...
            </div>
          )}
        </div>
      )}

      {/* Error Message */}
      {!notInstalled && isError && error && (
        <div
          style={{
            marginTop: "0.5rem",
            padding: "0.5rem",
            borderRadius: "0.375rem",
            backgroundColor: "var(--status-error-bg)",
            border: "1px solid var(--status-error-bg)",
            color: "var(--status-error-text)",
            fontSize: "0.75rem",
            wordBreak: "break-word",
          }}
          title={error}
        >
          <strong style={{ display: "block", marginBottom: "0.125rem" }}>Error:</strong>
          {error.length > 100 ? error.substring(0, 100) + "..." : error}
        </div>
      )}

      {/* Action Buttons */}
      <div style={{ display: "flex", gap: "0.375rem", marginTop: "0.5rem" }}>
        {notInstalled ? (
          <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)", fontStyle: "italic" }}>
            Install from Settings
          </span>
        ) : !isRunning ? (
          <button
            onClick={onStart}
            disabled={isTransitioning}
            className="btn-start"
            data-testid={`start-button-${serviceType}`}
          >
            Start
          </button>
        ) : (
          <>
            <button
              onClick={onStop}
              disabled={isTransitioning}
              className="btn-stop"
              data-testid={`stop-button-${serviceType}`}
            >
              Stop
            </button>
            <button
              onClick={onRestart}
              disabled={isTransitioning}
              className="btn-restart"
              data-testid={`restart-button-${serviceType}`}
            >
              Restart
            </button>
          </>
        )}
      </div>
    </div>
  );
}
