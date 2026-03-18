import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS } from "../types/services";

interface ServiceCardProps {
  serviceType: ServiceType;
  state: ServiceState;
  port?: number;
  error?: string;
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
  onStart,
  onStop,
  onRestart,
  ...props
}: ServiceCardProps) {
  const displayName = SERVICE_DISPLAY_NAMES[serviceType];
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
      className="card-hover"
      style={{
        backgroundColor: isError ? "var(--error-box-bg)" : "var(--bg-card)",
        border: `1px solid ${isError ? "var(--error-box-border)" : "var(--border-color)"}`,
        borderRadius: "0.5rem",
        padding: "1rem",
        boxShadow: "0 1px 3px rgba(0, 0, 0, 0.1)",
      }}
      data-testid={`service-card-${serviceType}`}
      {...props}
    >
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <h3 style={{ fontSize: "1rem", fontWeight: 600, margin: 0 }}>{displayName}</h3>
        <span
          style={{
            padding: "0.125rem 0.5rem",
            borderRadius: "9999px",
            fontSize: "0.65rem",
            fontWeight: 600,
            textTransform: "uppercase",
            backgroundColor: statusStyles.bg,
            color: statusStyles.text,
          }}
          data-testid={`service-state-${serviceType}`}
        >
          {state}
        </span>
      </div>

      {/* Description */}
      <p style={{ fontSize: "0.75rem", color: "var(--text-secondary)", marginBottom: "0.5rem", margin: "0 0 0.5rem 0" }}>
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

      {/* Error Message */}
      {isError && error && (
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
        {!isRunning && (
          <button
            onClick={onStart}
            disabled={isTransitioning}
            className="btn-start"
            data-testid={`start-button-${serviceType}`}
          >
            Start
          </button>
        )}
        {isRunning && (
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
