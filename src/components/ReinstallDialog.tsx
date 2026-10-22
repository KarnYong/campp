import { useEffect, useRef, useState } from "react";
import { useCaddyVersion } from "../utils/useCaddyVersion";
import { PasswordInput } from "./PasswordInput";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AppSettings, DownloadProgress, PackageSelection, PackagesConfig } from "../types/services";
import { detectPlatform } from "../utils/platform";
import { isAntivirusError, addDefenderExclusion } from "../utils/antivirus";
import { formatBytes } from "../utils/format";
import { LOGOS } from "../assets/logos";

interface ReinstallDialogProps {
  componentKey: string;
  displayName: string;
  /** Installed version, or null when this is a fresh install. */
  currentVersion: string | null;
  onClose: (changed: boolean) => void;
}

const DEFAULT_SELECTION: PackageSelection = {
  php: "php-8.5",
  mysql: "mysql-8.4",
  mariadb: "mariadb-12.3",
  phpmyadmin: "phpmyadmin-5.2",
  postgresql: "postgresql-18.3",
  adminer: "adminer-5.1",
  ngrok: "ngrok-3",
};

// All but the target go in the skip list. The backend still installs
// Caddy/PHP if they are missing.
const ALL_COMPONENTS = ["caddy", "php", "mysql", "mariadb", "phpmyadmin", "postgresql", "adminer", "ngrok"];

type Phase = "select" | "running" | "done" | "error";

export function ReinstallDialog({ componentKey, displayName, currentVersion, onClose }: ReinstallDialogProps) {
  const caddyVersion = useCaddyVersion();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [phase, setPhase] = useState<Phase>("select");
  const [packages, setPackages] = useState<PackagesConfig | null>(null);
  const [selection, setSelection] = useState<PackageSelection>(DEFAULT_SELECTION);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isFixingAv, setIsFixingAv] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [password, setPassword] = useState("");

  // MySQL/MariaDB re-apply the saved root password on every start, so it can be set on
  // install and reinstall. PostgreSQL only applies it when first initialized (fresh install).
  const isPostgres = componentKey === "postgresql";
  const needsPassword = componentKey === "mysql" || componentKey === "mariadb" || (isPostgres && !currentVersion);

  const options = packages?.[componentKey as keyof PackagesConfig] ?? [];
  const selectedId = selection[componentKey as keyof PackageSelection];

  useEffect(() => {
    dialogRef.current?.showModal();
    Promise.all([invoke<PackagesConfig>("get_available_packages_cmd"), invoke<AppSettings>("get_settings")])
      .then(([pkgs, loaded]) => {
        setPackages(pkgs);
        setSettings(loaded);
        setSelection({ ...DEFAULT_SELECTION, ...loaded.package_selection });
        setPassword((isPostgres ? loaded.postgres_root_password : loaded.mysql_root_password) || "");
      })
      .catch((e) => setError(`Failed to load available versions: ${e}`));

    const unlisten = listen<DownloadProgress>("download-progress", (e) => setProgress(e.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const run = async () => {
    if (needsPassword && !password.trim()) {
      setError(`${displayName} root password is required.`);
      return;
    }
    setError(null);
    setProgress(null);
    setPhase("running");
    try {
      // Persist the chosen version so later reinstalls/updates use it too.
      await invoke("update_package_selection", { packageSelection: selection });
      if (needsPassword && settings) {
        await invoke("update_db_passwords", {
          mysqlPassword: isPostgres ? settings.mysql_root_password : password,
          postgresPassword: isPostgres ? password : settings.postgres_root_password,
        });
      }
      await invoke("download_runtime_with_skip", {
        packageSelection: selection,
        skipList: ALL_COMPONENTS.filter((c) => c !== componentKey),
      });
      setPhase("done");
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  };

  const fixAvAndRetry = async () => {
    setIsFixingAv(true);
    try {
      await addDefenderExclusion();
    } catch (e) {
      setError(`Failed to add Defender exclusion: ${e}`);
      setIsFixingAv(false);
      return;
    }
    setIsFixingAv(false);
    await run();
  };

  const close = () => {
    if (phase === "running") return;
    dialogRef.current?.close();
    onClose(phase === "done");
  };

  const indeterminate = progress?.step === "extracting" || progress?.step === "installing";
  const stepLabel = !progress
    ? "Preparing…"
    : progress.step === "downloading"
      ? `Downloading ${progress.componentDisplay}`
      : progress.step === "extracting"
        ? `Extracting ${progress.componentDisplay}`
        : progress.step === "installing"
          ? "Setting up components…"
          : "Finishing…";
  const verb = currentVersion ? "Reinstall" : "Install";

  return (
    <dialog
      ref={dialogRef}
      className="modal"
      aria-labelledby="reinstall-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
    >
      <header className="modal__head">
        <span className="logo-tile"><img src={LOGOS[componentKey]} alt="" /></span>
        <div>
          <h3 id="reinstall-title">
            {verb} {displayName}
          </h3>
          <p>{currentVersion ? <>Installed: <span className="mono">v{currentVersion}</span></> : "Not installed"}</p>
        </div>
      </header>

      <div className="modal__body">
        {phase === "select" && (
          <>
            {componentKey === "caddy" ? (
              <p className="wiz-muted">Caddy is installed at the bundled version{caddyVersion ? ` (v${caddyVersion})` : ""}.</p>
            ) : (
              <div className="field">
                <label htmlFor="reinstall-version">Version</label>
                <select
                  id="reinstall-version"
                  className="input"
                  value={selectedId}
                  disabled={!packages}
                  onChange={(e) => setSelection({ ...selection, [componentKey]: e.target.value })}
                >
                  {!packages && <option>Loading…</option>}
                  {options.map((pkg) => (
                    <option key={pkg.id} value={pkg.id}>
                      {pkg.display_name}
                      {pkg.eol ? " (EOL)" : ""}
                      {pkg.lts ? " (LTS)" : ""}
                      {pkg.recommended ? " ★" : ""}
                    </option>
                  ))}
                </select>
              </div>
            )}
            {needsPassword && (
              <div className="field">
                <label htmlFor="reinstall-password">Root password</label>
                <PasswordInput
                  id="reinstall-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={`Password for the ${displayName} root user`}
                  autoComplete="new-password"
                />
              </div>
            )}
          </>
        )}

        {phase === "running" && (
          <div className="wiz-stack" aria-live="polite">
            <div className="modal__status">
              <span className="spinner" />
              <span className="modal__status-text">
                {stepLabel}
                {progress?.version && <span className="port-chip mono">{progress.version}</span>}
              </span>
              {progress?.step === "downloading" && progress.totalBytes > 0 && (
                <span className="modal__pct">{progress.percent}%</span>
              )}
            </div>
            <div className="progress-container wiz-progress">
              <div
                className={indeterminate ? "progress-bar-animated" : "progress-bar"}
                style={{ width: indeterminate ? "100%" : `${progress?.percent ?? 0}%` }}
              />
            </div>
            {progress?.step === "downloading" && progress.totalBytes > 0 && (
              <p className="wiz-muted wiz-small mono">
                {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
              </p>
            )}
          </div>
        )}

        {phase === "done" && (
          <div className="success-box success-box-text">
            {displayName} {currentVersion ? "reinstalled" : "installed"} successfully.
            {progress?.warning && <div className="wiz-small">⚠ {progress.warning}</div>}
          </div>
        )}

        {error && (
          <div className="error-box wiz-error">
            <p className="error-box-text">{error}</p>
            {phase === "error" && isAntivirusError(error) && detectPlatform() === "windows" && (
              <div className="wiz-error__actions">
                <button type="button" onClick={fixAvAndRetry} disabled={isFixingAv} className="btn-primary btn-xs">
                  {isFixingAv ? "Adding exclusion…" : "Add Defender exclusion & retry"}
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      <footer className="modal__foot">
        {phase === "select" && (
          <>
            <button type="button" className="btn-secondary btn-sm" onClick={close}>
              Cancel
            </button>
            <button type="button" className="btn-primary btn-sm" onClick={run} disabled={!packages}>
              {verb}
            </button>
          </>
        )}
        {phase === "running" && (
          <button type="button" className="btn-secondary btn-sm" disabled>
            Installing…
          </button>
        )}
        {phase === "error" && (
          <>
            <button type="button" className="btn-secondary btn-sm" onClick={close}>
              Close
            </button>
            <button type="button" className="btn-primary btn-sm" onClick={run}>
              Retry
            </button>
          </>
        )}
        {phase === "done" && (
          <button type="button" className="btn-primary btn-sm" onClick={close}>
            Done
          </button>
        )}
      </footer>
    </dialog>
  );
}
