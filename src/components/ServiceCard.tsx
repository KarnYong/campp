import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS, getDatabaseDisplayName } from "../types/services";
import { LOGOS } from "../assets/logos";

// Semantic color class per state (drives styling + is asserted by tests).
const STATUS_COLOR_CLASS: Record<ServiceState, string> = {
  [ServiceState.Stopped]: "status-gray",
  [ServiceState.Starting]: "status-blue",
  [ServiceState.Running]: "status-green",
  [ServiceState.Stopping]: "status-orange",
  [ServiceState.Error]: "status-red",
};

const BUSY_LABEL = { start: "starting", stop: "stopping", restart: "restarting" } as const;

interface ServiceCardProps {
  serviceType: ServiceType;
  state: ServiceState;
  port?: number;
  error?: string;
  notInstalled?: boolean;
  version?: string;
  /** Action currently in flight (set optimistically on click) — drives the
   *  spinner/disabled state immediately, before the backend reports back. */
  busyAction?: "start" | "stop" | "restart";
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
  version,
  busyAction,
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
  const logo = {
    [ServiceType.Caddy]: LOGOS.caddy,
    [ServiceType.PhpFpm]: LOGOS.php,
    [ServiceType.MySQL]: displayName === "MariaDB" ? LOGOS.mariadb : LOGOS.mysql,
    [ServiceType.PostgreSQL]: LOGOS.postgresql,
    [ServiceType.Ngrok]: LOGOS.ngrok,
  }[serviceType];

  const isRunning = state === ServiceState.Running;
  const isTransitioning = state === ServiceState.Starting || state === ServiceState.Stopping;
  const isError = state === ServiceState.Error;
  // Loading = a click is in flight (busyAction) OR the backend reports a
  // transition. Disables the controls and shows a spinner on the status.
  const loading = !!busyAction || isTransitioning;

  return (
    <div
      className={`service-row${notInstalled ? " service-row--missing" : ""}`}
      data-testid={`service-card-${serviceType}`}
      {...props}
    >
      <span className="logo-tile">
        <img src={logo} alt="" />
      </span>

      <div className="service-row__main">
        <div className="service-row__title">
          <h3>{displayName}</h3>
          <span>{description}</span>
          {version && <span className="mono">{version}</span>}
        </div>
        {serviceType === ServiceType.Ngrok && isRunning && (
          tunnelUrl ? (
            <a className="service-row__url mono" href={tunnelUrl} target="_blank" rel="noopener noreferrer">{tunnelUrl}</a>
          ) : (
            <span className="service-row__note">Connecting…</span>
          )
        )}
        {!notInstalled && isError && error && (
          <p className="service-row__error" title={error}>
            {error.length > 100 ? error.substring(0, 100) + "..." : error}
          </p>
        )}
      </div>

      <span className="port-chip mono">:{port}</span>

      <span
        className={`service-status ${notInstalled ? "status-gray" : STATUS_COLOR_CLASS[state]}`}
        data-testid={`service-state-${serviceType}`}
      >
        {loading ? <span className="btn-spinner" aria-hidden="true" /> : <span className="status-dot" />}
        {notInstalled ? "Not Installed" : busyAction ? BUSY_LABEL[busyAction] : state}
      </span>

      {notInstalled ? (
        <span className="service-row__note">Install from Settings</span>
      ) : (
        <>
          {isRunning ? (
            <button
              type="button"
              className="icon-btn"
              onClick={onRestart}
              disabled={loading}
              aria-label={`Restart ${displayName}`}
              title="Restart"
              data-testid={`restart-button-${serviceType}`}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" />
              </svg>
            </button>
          ) : (
            <span className="icon-btn-spacer" />
          )}
          <button
            type="button"
            role="switch"
            aria-checked={isRunning}
            aria-label={`${isRunning ? "Stop" : "Start"} ${displayName}`}
            className="switch"
            onClick={isRunning ? onStop : onStart}
            disabled={loading}
            data-testid={`${isRunning ? "stop" : "start"}-button-${serviceType}`}
          >
            <span className="switch__knob" />
          </button>
        </>
      )}
    </div>
  );
}
