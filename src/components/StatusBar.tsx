import { ServiceMap, ServiceType, SERVICE_DISPLAY_NAMES, getDatabaseDisplayName } from "../types/services";
import pkg from "../../package.json";

interface StatusBarProps {
  services: Partial<ServiceMap>;
  [key: string]: any; // Allow additional props like data-testid
}

function serviceName(t: ServiceType): string {
  return t === ServiceType.MySQL ? getDatabaseDisplayName() : SERVICE_DISPLAY_NAMES[t];
}

export function StatusBar({ services, ...props }: StatusBarProps) {
  const runningCount = Object.values(services).filter((s) => s?.state === "running").length;
  const totalCount = Object.keys(services).length;

  return (
    <div className="stack-card" {...props}>
      <div className="stack-card__head">
        <span>Stack</span>
        <span className="mono">{runningCount}/{totalCount} running</span>
      </div>
      <div className="stack-card__bars">
        {(Object.keys(services) as ServiceType[]).map((t) => (
          <span
            key={t}
            className={`stack-card__bar stack-card__bar--${services[t]?.state ?? "stopped"}`}
            title={`${serviceName(t)}: ${services[t]?.state ?? "unknown"}`}
          />
        ))}
      </div>
      <span className="stack-card__version mono">CAMPP v{pkg.version}</span>
    </div>
  );
}
