import { ServiceMap, ServiceType, SERVICE_DISPLAY_NAMES, getDatabaseDisplayName } from "../types/services";
import pkg from "../../package.json";

interface StatusBarProps {
  services: Partial<ServiceMap>;
  [key: string]: any; // Allow additional props like data-testid
}

function dotClass(state?: string): string {
  switch (state) {
    case "running": return "status-dot--running";
    case "starting": return "status-dot--starting";
    case "stopping": return "status-dot--stopping";
    case "error": return "status-dot--error";
    default: return "";
  }
}

function serviceName(t: ServiceType): string {
  return t === ServiceType.MySQL ? getDatabaseDisplayName() : SERVICE_DISPLAY_NAMES[t];
}

export function StatusBar({ services, ...props }: StatusBarProps) {
  const runningCount = Object.values(services).filter((s) => s?.state === "running").length;
  const totalCount = Object.keys(services).length;

  return (
    <div
      style={{
        backgroundColor: "var(--bg-card)",
        borderTop: "1px solid var(--border-color)",
        padding: "0.5rem 1.5rem",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        gap: "1rem",
        fontSize: "0.75rem",
        color: "var(--text-secondary)",
      }}
      {...props}
    >
      <div style={{ display: "flex", gap: "0.875rem", alignItems: "center", flexWrap: "wrap" }}>
        {(Object.keys(services) as ServiceType[]).map((t) => {
          const s = services[t];
          const label = serviceName(t);
          return (
            <span
              key={t}
              style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
              title={`${label}: ${s?.state ?? "unknown"}`}
            >
              <span className={`status-dot ${dotClass(s?.state)}`} />
              {label}
            </span>
          );
        })}
        <span style={{ display: "flex", alignItems: "center" }}>
          {runningCount}/{totalCount} running
        </span>
      </div>
      <div style={{ display: "flex", gap: "1.5rem", alignItems: "center" }}>
        <span>CAMPP v{pkg.version}</span>
      </div>
    </div>
  );
}
