import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ServiceType, ServiceState, SERVICE_DISPLAY_NAMES, SERVICE_DESCRIPTIONS, DEFAULT_PORTS, getDatabaseDisplayName } from "../types/services";
import type { AppSettings } from "../types/services";
import { LOGOS } from "../assets/logos";
import { buildEnvBlock } from "../lib/envBlock";

/** Copy the Laravel `.env` DB lines for CAMPP's database (spec 006 US4). Settings are
 *  read on click, so a changed port/password is always current. The password is
 *  never shown unless the user presses Show (only the clipboard-failure box shows it). */
function CopyEnvButton() {
  const [status, setStatus] = useState<"idle" | "copied" | "manual">("idle");
  const [block, setBlock] = useState("");
  const [password, setPassword] = useState("");
  const [revealed, setRevealed] = useState(false);

  const copy = async () => {
    const s = await invoke<AppSettings>("get_settings");
    const text = buildEnvBlock({ port: s.mysql_port, password: s.mysql_root_password || "" });
    setBlock(text);
    setPassword(s.mysql_root_password || "");
    setRevealed(false);
    try {
      await navigator.clipboard.writeText(text);
      setStatus("copied");
    } catch {
      setStatus("manual");
    }
  };

  const masked = password ? block.replace(/^DB_PASSWORD=.*$/m, "DB_PASSWORD=••••") : block;
  return (
    <div className="env-copy">
      <button type="button" className="env-copy__btn" onClick={copy} data-testid="copy-env-button">
        Copy .env settings
      </button>
      {status === "copied" && (
        <span className="service-row__note" role="status">
          Copied. Create the database first (e.g. in phpMyAdmin) and change DB_DATABASE.
        </span>
      )}
      {status === "manual" && (
        <>
          <span className="service-row__note">Couldn't use the clipboard. Select and copy:</span>
          <textarea className="env-copy__box mono" readOnly rows={6} value={revealed ? block : masked} data-testid="env-block" />
          {password && (
            <button type="button" className="env-copy__btn" onClick={() => setRevealed((r) => !r)}>
              {revealed ? "Hide password" : "Show password"}
            </button>
          )}
        </>
      )}
    </div>
  );
}

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
  /** Switch this service to another port and start it (reserved-port fix, spec 006 US3). */
  onUsePort?: (port: number) => void;
  [key: string]: any; // Allow additional props like data-testid
}

/** The suggested port from the backend's reserved-port message (process/manager.rs
 *  `reserved_port_message`, contracts/check-ports.md), or null if not that error. */
export function reservedPortSuggestion(error?: string): number | null {
  if (!error?.includes("reserved by Windows")) return null;
  const m = error.match(/Switch to port (\d+)/);
  return m ? Number(m[1]) : null;
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
  onUsePort,
  ...props
}: ServiceCardProps) {
  const reserved = !!error?.includes("reserved by Windows");
  const suggestedPort = reservedPortSuggestion(error);
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
        {serviceType === ServiceType.MySQL && !notInstalled && <CopyEnvButton />}
        {!notInstalled && isError && error && (
          <p className="service-row__error" title={error}>
            {/* The reserved-port message is the fix itself, so it is never cut short. */}
            {!reserved && error.length > 100 ? error.substring(0, 100) + "..." : error}
          </p>
        )}
        {!notInstalled && isError && suggestedPort !== null && onUsePort && (
          <button
            type="button"
            className="env-copy__btn"
            onClick={() => onUsePort(suggestedPort)}
            disabled={loading}
            data-testid={`use-port-button-${serviceType}`}
          >
            Use port {suggestedPort}
          </button>
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
        <span className="service-row__note">Install from Components</span>
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
