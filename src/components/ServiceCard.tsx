import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS } from "../types/services";

interface ServiceCardProps {
  serviceType: ServiceType;
  state: ServiceState;
  port?: number;
  error?: string;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
}

const statusColors = {
  [ServiceState.Stopped]: "gray",
  [ServiceState.Starting]: "blue",
  [ServiceState.Running]: "green",
  [ServiceState.Stopping]: "orange",
  [ServiceState.Error]: "red",
};

export function ServiceCard({
  serviceType,
  state,
  port = DEFAULT_PORTS[serviceType],
  error,
  onStart,
  onStop,
  onRestart,
}: ServiceCardProps) {
  const displayName = SERVICE_DISPLAY_NAMES[serviceType];
  const description = SERVICE_DESCRIPTIONS[serviceType];
  const statusColor = statusColors[state];

  const isRunning = state === ServiceState.Running;
  const isTransitioning = state === ServiceState.Starting || state === ServiceState.Stopping;
  const isError = state === ServiceState.Error;

  return (
    <div className={`service-card ${isError ? 'service-card-error' : ''}`} data-testid={`service-card-${serviceType}`}>
      <div className="service-header">
        <h3>{displayName}</h3>
        <span className={`status-indicator status-${statusColor}`} data-testid={`service-state-${serviceType}`}>{state}</span>
      </div>
      <p className="service-description">{description}</p>
      <div className="service-info">
        <span>Port: {port}</span>
      </div>
      {isError && error && (
        <div className="service-error" title={error}>
          <strong>Error:</strong> {error.length > 100 ? error.substring(0, 100) + '...' : error}
        </div>
      )}
      <div className="service-actions">
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
