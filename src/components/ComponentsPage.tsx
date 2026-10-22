import { invoke } from "@tauri-apps/api/core";
import { showAlert, showConfirm } from "./Dialog";
import { useState, useEffect, useCallback } from "react";
import { getDatabaseDisplayName } from "../types/services";
import { detectPlatform } from "../utils/platform";
import { NgrokAntivirusNotice } from "./NgrokAntivirusNotice";
import { ReinstallDialog } from "./ReinstallDialog";
import { LOGOS } from "../assets/logos";

interface ComponentInfo {
  key: string;
  displayName: string;
  installed: boolean;
  version: string | null;
}

const COMPONENT_ORDER: { key: string; getDisplayName: () => string }[] = (() => {
  const platform = detectPlatform();
  const dbKey = platform === "linux" ? "mariadb" : "mysql";
  return [
    { key: "caddy", getDisplayName: () => "Caddy" },
    { key: "php", getDisplayName: () => "PHP" },
    { key: dbKey, getDisplayName: () => getDatabaseDisplayName(platform) },
    { key: "phpmyadmin", getDisplayName: () => "phpMyAdmin" },
    { key: "postgresql", getDisplayName: () => "PostgreSQL" },
    { key: "adminer", getDisplayName: () => "Adminer" },
    { key: "ngrok", getDisplayName: () => "ngrok" },
  ];
})();

/** Install, reinstall (switch version) or uninstall runtime components. */
export function ComponentsPage({ servicesActive, onChanged }: { servicesActive: boolean; onChanged?: () => void }) {
  const [components, setComponents] = useState<ComponentInfo[]>([]);
  const [componentAction, setComponentAction] = useState<string | null>(null);
  // Last uninstall error (install errors are shown inside ReinstallDialog).
  const [componentError, setComponentError] = useState<string | null>(null);
  const [installTarget, setInstallTarget] = useState<ComponentInfo | null>(null);

  const loadComponents = useCallback(async () => {
    try {
      const versions = await invoke<Record<string, string>>("check_existing_components");
      setComponents(COMPONENT_ORDER.map(({ key, getDisplayName }) => ({
        key,
        displayName: getDisplayName(),
        installed: !!versions[key],
        version: versions[key] || null,
      })));
    } catch (e) {
      console.error("Failed to load components:", e);
    }
  }, []);

  useEffect(() => {
    loadComponents();
  }, [loadComponents]);

  const refreshComponents = async () => {
    await loadComponents();
    onChanged?.();
  };

  // Clear a stale error when a new action starts.
  const clearComponentError = () => setComponentError(null);

  const handleUninstall = async (componentKey: string) => {
    clearComponentError();
    if (componentKey === "php" || componentKey === "mysql" || componentKey === "mariadb") {
      const name = componentKey === "php" ? "PHP" : getDatabaseDisplayName();
      const msg = componentKey === "php"
        ? "PHP is required by Caddy for serving PHP files and phpMyAdmin. Uninstall anyway?"
        : `${getDatabaseDisplayName()} is required by phpMyAdmin. Uninstall anyway?`;
      if (!(await showConfirm(`Uninstall ${name}?`, msg, "Uninstall", true))) return;
    }

    // Prevent uninstalling last component
    if (components.filter((c) => c.installed).length <= 1) {
      await showAlert("Can't uninstall", "At least one component must remain installed.");
      return;
    }

    setComponentAction(`uninstall-${componentKey}`);
    try {
      await invoke("uninstall_component", { component: componentKey });
      await refreshComponents();
    } catch (e) {
      setComponentError(`Failed to uninstall: ${e}`);
    } finally {
      setComponentAction(null);
    }
  };

  // Install/reinstall happen in ReinstallDialog (version picker + progress).
  const openInstallDialog = (componentKey: string) => {
    clearComponentError();
    setInstallTarget(components.find((c) => c.key === componentKey) ?? null);
  };

  const busy = !!componentAction || servicesActive;
  const busyTitle = servicesActive ? "Stop all services first" : undefined;

  return (
    <div className="page">
      <header className="main__header">
        <div>
          <h2>Components</h2>
          <p>Install, reinstall or remove the software CAMPP runs.</p>
        </div>
      </header>

      {servicesActive && (
        <div className="warn-box">
          <span>Stop all services to install, reinstall or remove components.</span>
        </div>
      )}

      <section className="card-section" aria-label="Components">
        {componentError && (
          <div className="error-box">
            <p className="error-box-text" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", marginBottom: "0.5rem" }}>
              {componentError}
            </p>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <button type="button" onClick={clearComponentError} className="btn-secondary btn-xs btn-danger">
                Dismiss
              </button>
            </div>
          </div>
        )}
        <div className="list-box">
          {components.map((comp) => {
            const isActing = componentAction === `uninstall-${comp.key}`;
            const canUninstall = comp.installed && components.filter((c) => c.installed).length > 1;
            return (
              <div key={comp.key} className="list-row">
                <span className="logo-tile logo-tile--sm"><img src={LOGOS[comp.key]} alt="" /></span>
                <div className="list-row__main">
                  <span>{comp.displayName}</span>
                  {comp.installed && comp.version ? (
                    <span className="mono">v{comp.version}</span>
                  ) : (
                    <span className="list-row__warn">Not installed</span>
                  )}
                </div>
                {isActing && (
                  <span className="list-row__busy">
                    <span className="btn-spinner" aria-hidden="true" />
                    Working…
                  </span>
                )}
                {comp.installed ? (
                  <>
                    <button
                      type="button"
                      onClick={() => openInstallDialog(comp.key)}
                      disabled={busy}
                      title={busyTitle}
                      className="btn-secondary btn-xs"
                    >
                      Reinstall
                    </button>
                    <button
                      type="button"
                      onClick={() => handleUninstall(comp.key)}
                      disabled={busy || !canUninstall}
                      className="btn-secondary btn-xs btn-danger"
                      title={busyTitle ?? (!canUninstall ? "At least one component must remain installed" : undefined)}
                    >
                      Uninstall
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={() => openInstallDialog(comp.key)}
                    disabled={busy}
                    title={busyTitle}
                    className="btn-primary btn-xs"
                  >
                    Install
                  </button>
                )}
              </div>
            );
          })}
        </div>
        {detectPlatform() === "windows" && <NgrokAntivirusNotice />}
      </section>

      {installTarget && (
        <ReinstallDialog
          componentKey={installTarget.key}
          displayName={installTarget.displayName}
          currentVersion={installTarget.installed ? installTarget.version : null}
          onClose={(changed) => {
            setInstallTarget(null);
            if (changed) refreshComponents();
          }}
        />
      )}
    </div>
  );
}
