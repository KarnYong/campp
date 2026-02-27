import { ServiceMap } from "../types/services";

interface StatusBarProps {
  services: Partial<ServiceMap>;
}

export function StatusBar({ services }: StatusBarProps) {
  const runningCount = Object.values(services).filter((s) => s?.state === "running").length;
  const totalCount = Object.keys(services).length;

  return (
    <div className="status-bar">
      <div className="status-bar-left">
        <span className="status-item">
          Services: {runningCount}/{totalCount} running
        </span>
      </div>
      <div className="status-bar-right">
        <span className="status-item">CAMPP v0.1.0</span>
      </div>
    </div>
  );
}
