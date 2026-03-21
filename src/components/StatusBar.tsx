import { ServiceMap } from "../types/services";
import pkg from "../../package.json";

interface StatusBarProps {
  services: Partial<ServiceMap>;
  [key: string]: any; // Allow additional props like data-testid
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
        fontSize: "0.75rem",
        color: "var(--text-secondary)",
      }}
      {...props}
    >
      <div style={{ display: "flex", gap: "1.5rem" }}>
        <span style={{ display: "flex", alignItems: "center" }}>
          Services: {runningCount}/{totalCount} running
        </span>
      </div>
      <div style={{ display: "flex", gap: "1.5rem", alignItems: "center" }}>
        <span style={{ display: "flex", alignItems: "center" }}>
          <kbd className="kbd">Ctrl</kbd>+<kbd className="kbd">Shift</kbd>+<kbd className="kbd">D</kbd> Debug
        </span>
        <span>CAMPP v{pkg.version}</span>
      </div>
    </div>
  );
}
