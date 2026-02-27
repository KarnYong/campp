import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS } from "../types/services";

interface ServiceCardProps {
  serviceType: ServiceType;
  state: ServiceState;
  port?: number;
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
  onStart,
  onStop,
  onRestart,
}: ServiceCardProps) {
  const displayName = SERVICE_DISPLAY_NAMES[serviceType];
  const description = SERVICE_DESCRIPTIONS[serviceType];
  const statusColor = statusColors[state];

  const isRunning = state === ServiceState.Running;
  const isTransitioning = state === ServiceState.Starting || state === ServiceState.Stopping;

  return (
    <div className="service-card">
      <div className="service-header">
        <h3>{displayName}</h3>
        <span className={`status-indicator status-${statusColor}`}>{state}</span>
      </div>
      <p className="service-description">{description}</p>
      <div className="service-info">
        <span>Port: {port}</span>
      </div>
      <div className="service-actions">
        {!isRunning && (
          <button
            onClick={onStart}
            disabled={isTransitioning}
            className="btn-start"
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
            >
              Stop
            </button>
            <button
              onClick={onRestart}
              disabled={isTransitioning}
              className="btn-restart"
            >
              Restart
            </button>
          </>
        )}
      </div>
    </div>
  );
}
