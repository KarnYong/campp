import { useState, useEffect } from "react";
import { showAlert } from "./Dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  DownloadProgress as DownloadProgressType,
  PackageSelection,
  DependencyCheckResult,
  getDatabaseDisplayName,
} from "../types/services";
import { PackageSelector } from "./PackageSelector";
import { NgrokAntivirusNotice } from "./NgrokAntivirusNotice";
import { detectPlatform } from "../utils/platform";
import { isAntivirusError, addDefenderExclusion } from "../utils/antivirus";
import pkg from "../../package.json";
import { LOGOS } from "../assets/logos";
import { formatBytes } from "../utils/format";
import logoUrl from "../assets/melivecode-logo.png";
import { Icon } from "./icons";

interface FirstRunWizardProps {
  onComplete: () => void;
  [key: string]: any; // Allow additional props like data-testid
}

type WizardStep = "welcome" | "packages" | "dependencies" | "confirm" | "download" | "complete";

interface ExistingComponent {
  name: string;
  version: string;
  displayName: string;
  isExisting: boolean;
}

// Package versions for display
const packages = {
  php: [
    { id: "php-8.5", version: "8.5.12" },
    { id: "php-8.4", version: "8.4.27" },
    { id: "php-8.3", version: "8.3.36" },
    { id: "php-8.2", version: "8.2.35" },
    { id: "php-7.4", version: "7.4.33" },
  ],
  mysql: [
    { id: "mysql-8.4", version: "8.4.11" },
    { id: "mysql-8.0", version: "8.0.46" },
  ],
  mariadb: [
    { id: "mariadb-12.3", version: "12.3.3" },
  ],
  phpmyadmin: [
    { id: "phpmyadmin-5.2", version: "5.2.3" },
  ],
  postgresql: [
    { id: "postgresql-18.3", version: "18.6.1" },
  ],
  adminer: [
    { id: "adminer-5.1", version: "5.1.0" },
  ],
  ngrok: [
    { id: "ngrok-3", version: "3.19.2" },
  ],
};

export function FirstRunWizard({ onComplete, ...props }: FirstRunWizardProps) {
  const [step, setStep] = useState<WizardStep>("welcome");
  const [currentPlatform, setCurrentPlatform] = useState<string>("linux");
  const [progress, setProgress] = useState<DownloadProgressType>({
    step: "downloading",
    percent: 0,
    currentComponent: "",
    componentDisplay: "",
    version: "",
    totalComponents: 4,
    downloadedBytes: 0,
    totalBytes: 0,
  });
  const [error, setError] = useState<string | null>(null);
  // Remember the skip list used for the last download attempt so the AV
  // "fix & retry" button can replay the exact same install.
  const [lastSkipList, setLastSkipList] = useState<string[]>([]);
  const [isFixingAv, setIsFixingAv] = useState(false);
  const [packageSelection, setPackageSelection] = useState<PackageSelection>({
    php: "php-8.5",
    mysql: "mysql-8.4",
    mariadb: "mariadb-12.3",
    phpmyadmin: "phpmyadmin-5.2",
    postgresql: "postgresql-18.3",
    adminer: "adminer-5.1",
    ngrok: "ngrok-3",
  });
  const [existingComponents, setExistingComponents] = useState<ExistingComponent[]>([]);
  const [hasExistingOnWelcome, setHasExistingOnWelcome] = useState(false);
  const [dependencyCheckResult, setDependencyCheckResult] = useState<DependencyCheckResult | null>(null);
  const [enabledComponents, setEnabledComponents] = useState<Record<string, boolean>>({
    caddy: true,
    php: true,
    mysql: true,
    mariadb: true,
    phpmyadmin: true,
    postgresql: false,
    adminer: false,
    ngrok: false,
  });
  const [mysqlPassword, setMysqlPassword] = useState("");
  const [postgresPassword, setPostgresPassword] = useState("");

  // Check for existing components when welcome step loads
  useEffect(() => {
    const checkExisting = async () => {
      try {
        const existing = await invoke<Record<string, string>>("check_existing_components");
        const hasExisting = Object.keys(existing).length > 0;
        setHasExistingOnWelcome(hasExisting);

        // Auto-enable optional components if already installed
        if (existing.postgresql || existing.adminer || existing.ngrok) {
          setEnabledComponents((prev) => ({
            ...prev,
            postgresql: !!existing.postgresql || prev.postgresql,
            adminer: !!existing.adminer || prev.adminer,
            ngrok: !!existing.ngrok || prev.ngrok,
          }));
        }
      } catch (err) {
        console.error("Failed to check existing components:", err);
        setHasExistingOnWelcome(false);
      }
    };

    if (step === "welcome") {
      checkExisting();
    }
  }, [step]);

  // Detect platform on mount
  useEffect(() => {
    setCurrentPlatform(detectPlatform());
  }, []);

  // Load debug info
  useEffect(() => {
    invoke<Record<string, unknown>>("get_debug_info").catch(() => {});
  }, []);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgressType>("download-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.step === "complete") {
        setStep("complete");
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Keyboard shortcut handler for Ctrl+Shift+D / Cmd+Shift+D
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'd') {
        e.preventDefault();
        try {
          const downloadDir = await invoke<string>("get_download_dir");
          await invoke("open_folder", { path: downloadDir });
        } catch (err) {
          console.error("Failed to open download folder:", err);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const startDownload = async () => {
    setError(null);

    const dbKey = currentPlatform === "linux" ? "mariadb" : "mysql";
    if (enabledComponents[dbKey] && !mysqlPassword.trim()) {
      setError("MySQL password is required.");
      return;
    }
    if (enabledComponents.postgresql && !postgresPassword.trim()) {
      setError("PostgreSQL password is required.");
      return;
    }

    // First, check system dependencies
    try {
      const depsResult = await invoke<DependencyCheckResult>("check_system_dependencies");
      setDependencyCheckResult(depsResult);

      if (!depsResult.all_satisfied) {
        setStep("dependencies");
        return;
      }
    } catch (err) {
      console.error("Failed to check dependencies:", err);
      // Continue anyway if dependency check fails
    }

    // Then, check for existing components
    try {
      const existing = await invoke<Record<string, string>>("check_existing_components");

      // Determine which database component to show based on platform
      const dbKey = currentPlatform === "linux" ? "mariadb" : "mysql";

      // All components that should be shown
      const allComponents: ExistingComponent[] = [
        {
          name: "caddy",
          version: existing.caddy || "",
          displayName: "Caddy",
          isExisting: !!existing.caddy,
        },
        {
          name: "php",
          version: existing.php || "",
          displayName: "PHP",
          isExisting: !!existing.php,
        },
        {
          name: dbKey,
          version: existing[dbKey] || "",
          displayName: getDatabaseDisplayName(currentPlatform),
          isExisting: !!existing[dbKey],
        },
        {
          name: "phpmyadmin",
          version: existing.phpmyadmin || "",
          displayName: "phpMyAdmin",
          isExisting: !!existing.phpmyadmin,
        },
        {
          name: "postgresql",
          version: existing.postgresql || "",
          displayName: "PostgreSQL",
          isExisting: !!existing.postgresql,
        },
        {
          name: "adminer",
          version: existing.adminer || "",
          displayName: "Adminer",
          isExisting: !!existing.adminer,
        },
        {
          name: "ngrok",
          version: existing.ngrok || "",
          displayName: "ngrok",
          isExisting: !!existing.ngrok,
        },
      ];

      const hasAnyExisting = allComponents.some((c) => c.isExisting);

      if (!hasAnyExisting) {
        proceedWithDownload([]);
        return;
      }

      setExistingComponents(allComponents);
      setStep("confirm");
      return;
    } catch (err) {
      console.error("Failed to check existing components:", err);
    }

    // No existing components, proceed with download
    proceedWithDownload([]);
  };

  const getDisabledComponents = (): string[] => {
    return Object.entries(enabledComponents)
      .filter(([_, v]) => !v)
      .map(([k]) => k);
  };

  const proceedWithDownload = async (existingSkipList: string[]) => {
    setError(null);
    setStep("download");

    // Merge existing skip list + user-disabled components
    const disabled = getDisabledComponents();
    const skipList = [...new Set([...existingSkipList, ...disabled])];
    setLastSkipList(skipList);

    try {
      // Save the package selection and database passwords to settings
      await invoke("update_package_selection", { packageSelection });
      await invoke("update_db_passwords", {
        mysqlPassword,
        postgresPassword,
      });

      if (skipList.length > 0) {
        const result = await invoke<string>("download_runtime_with_skip", {
          packageSelection,
          skipList,
        });
        console.log(result);
      } else {
        const result = await invoke<string>("download_runtime_with_packages", { packageSelection });
        console.log(result);
      }
    } catch (err) {
      console.error("Download error:", err);
      setError(err as string);
      setStep("confirm");
    }
  };

  // Called from the "Add Defender exclusion & retry" button when an antivirus
  // block is detected. Adds the exclusions (UAC prompt) then replays the last
  // download attempt.
  const handleAvFixAndRetry = async () => {
    setError(null);
    setIsFixingAv(true);
    try {
      await addDefenderExclusion();
    } catch (e) {
      setError(`Failed to add Defender exclusion: ${e}`);
      setIsFixingAv(false);
      return;
    }
    setIsFixingAv(false);
    await proceedWithDownload(lastSkipList);
  };

  // Called from the complete-step banner after a bulk install where ngrok was
  // skipped by the antivirus. Adds the exclusion, then reinstalls ngrok only.
  const handleRetryNgrokFromBanner = async () => {
    setError(null);
    setIsFixingAv(true);
    try {
      await addDefenderExclusion();
    } catch (e) {
      setError(`Failed to add Defender exclusion: ${e}`);
      setIsFixingAv(false);
      return;
    }
    setIsFixingAv(false);
    setStep("download");
    // Reinstall ngrok only — skip every other component. Caddy/PHP are required
    // so they come along, but the DB/phpMyAdmin/etc. are skipped.
    const skipList = ["caddy", "php", "mysql", "mariadb", "phpmyadmin", "postgresql", "adminer"];
    try {
      await invoke("download_runtime_with_skip", { packageSelection, skipList });
    } catch (e) {
      setError(`Failed to reinstall ngrok: ${e}`);
      setStep("confirm");
    }
  };

  // "Add Defender exclusion & retry" — shown inside error boxes only when the
  // failure looks like an antivirus block on Windows.
  const avFixButton =
    error && isAntivirusError(error) && currentPlatform === "windows" ? (
      <button
        onClick={handleAvFixAndRetry}
        disabled={isFixingAv}
        className="btn-primary btn-xs"
      >
        {isFixingAv ? "Adding exclusion…" : "Add Defender exclusion & retry"}
      </button>
    ) : null;

  const handleOverwriteAll = () => {
    proceedWithDownload([]);
  };

  const handleSkipExisting = () => {
    const skipList = existingComponents.filter(c => c.isExisting).map(c => c.name);
    proceedWithDownload(skipList);
  };

  const handleCancel = () => {
    setStep("packages");
    setExistingComponents([]);
  };

  const handleSkipToDashboard = () => {
    onComplete();
  };

  const handleSkipFromWelcome = async () => {
    try {
      const existing = await invoke<Record<string, string>>("check_existing_components");
      const existingList = Object.keys(existing);

      if (existingList.length > 0) {
        onComplete();
      } else {
        void showAlert("No existing installation found", "Please download the runtime components to continue.");
      }
    } catch (err) {
      console.error("Failed to check existing components:", err);
    }
  };

  const handleNext = () => {
    if (step === "welcome") {
      setStep("packages");
    }
  };

  const handleBack = () => {
    if (step === "packages") {
      setStep("welcome");
    } else if (step === "dependencies") {
      setStep("packages");
    }
  };

  const handlePackageChange = (selection: PackageSelection) => {
    setPackageSelection(selection);
  };

  const getStepLabel = () => {
    switch (progress.step) {
      case "downloading":
        return progress.componentDisplay
          ? `Download ${progress.componentDisplay}`
          : "Downloading...";
      case "extracting":
        return progress.componentDisplay
          ? `Extract ${progress.componentDisplay}`
          : "Extracting...";
      case "installing":
        return "Installing...";
      case "complete":
        return "Installation complete!";
      case "error":
        return "An error occurred";
      default:
        return "Preparing...";
    }
  };

  const getStepNumber = () => {
    switch (step) {
      case "welcome": return 1;
      case "packages": return 2;
      case "dependencies": return 3;
      case "confirm": return 4;
      case "download": return 5;
      case "complete": return 5;
      default: return 1;
    }
  };

  const STEPS = ["Welcome", "Components", "Review", "Install", "Done"];
  const currentStepNum = getStepNumber();
  const dbKey = currentPlatform === "linux" ? "mariadb" : "mysql";
  const versionOf = (key: string): string | undefined => {
    if (key === "caddy") return "2.11.3";
    const id = packageSelection[key as keyof PackageSelection];
    return packages[key as keyof typeof packages]?.find((p) => p.id === id)?.version;
  };

  const errorBox = (dismissable: boolean) =>
    error && (
      <div className="error-box wiz-error">
        <p className="error-box-text">
          <strong>Error:</strong> {error}
        </p>
        {(dismissable || avFixButton) && (
          <div className="wiz-error__actions">
            {dismissable && (
              <button onClick={() => setError(null)} className="btn-secondary btn-xs btn-danger">
                Dismiss
              </button>
            )}
            {avFixButton}
          </div>
        )}
      </div>
    );

  return (
    <div className="wiz" {...props}>
      <div className="wiz__card animate-slide-in">
        {/* Header */}
        <header className="wiz__head">
          <div className="wiz__brand">
            <img src={logoUrl} alt="" />
            <div>
              <h1>CAMPP</h1>
              <span>
                Local web development stack · v{pkg.version} · {detectPlatform()}
              </span>
            </div>
          </div>
          <button
            onClick={async () => {
              try {
                await invoke("open_manual");
              } catch (err) {
                console.error("Failed to open manual:", err);
              }
            }}
            className="icon-btn"
            title="Read User Manual"
            aria-label="Read User Manual"
          >
            <Icon name="help" />
          </button>
        </header>

        {/* Step indicator */}
        <ol className="wiz-steps" aria-label="Setup progress">
          {STEPS.map((label, i) => {
            const n = i + 1;
            const state = currentStepNum > n || step === "complete" ? "done" : currentStepNum === n ? "active" : "todo";
            return (
              <li
                key={label}
                className={`wiz-steps__item wiz-steps__item--${state}`}
                aria-current={state === "active" ? "step" : undefined}
              >
                <span className="wiz-steps__dot">{state === "done" ? "✓" : n}</span>
                <span className="wiz-steps__label">{label}</span>
              </li>
            );
          })}
        </ol>

        {/* Content */}
        <div className="wiz__body">
          {/* Welcome Step */}
          {step === "welcome" && (
            <>
              <div className="wiz__intro">
                <h2>Welcome to CAMPP</h2>
                <p>
                  CAMPP needs a few runtime binaries: Caddy, PHP-FPM, {getDatabaseDisplayName(currentPlatform)} and
                  phpMyAdmin. You pick the versions on the next screen. Everything installs into the CAMPP folder, so
                  no admin rights are needed.
                </p>
              </div>
              <div className="wiz-logos" aria-hidden="true">
                {["caddy", "php", dbKey, "phpmyadmin", "postgresql", "adminer"].map((k) => (
                  <span key={k} className="logo-tile">
                    <img src={LOGOS[k]} alt="" />
                  </span>
                ))}
              </div>
              {hasExistingOnWelcome ? (
                <div className="success-box success-box-text">
                  <strong>Existing installation detected.</strong> Use it as-is, or download fresh binaries.
                </div>
              ) : (
                <div className="info-box">
                  <strong>Estimated download size:</strong> ~150 MB
                </div>
              )}
            </>
          )}

          {/* Package Selection Step */}
          {step === "packages" && (
            <>
              <div className="wiz__intro">
                <h2>Choose components</h2>
                <p>Select versions and set root passwords for the databases you want to install.</p>
              </div>
              <PackageSelector
                onSelectionChange={handlePackageChange}
                initialSelection={packageSelection}
                initialEnabled={enabledComponents}
                onEnabledChange={setEnabledComponents}
                mysqlPassword={mysqlPassword}
                onMysqlPasswordChange={setMysqlPassword}
                postgresPassword={postgresPassword}
                onPostgresPasswordChange={setPostgresPassword}
              />
              {enabledComponents.ngrok && currentPlatform === "windows" && <NgrokAntivirusNotice />}
              {errorBox(false)}
            </>
          )}

          {/* Dependencies Step */}
          {step === "dependencies" && dependencyCheckResult && (
            <>
              <div className="wiz__intro">
                <h2 className="text-bad">Missing system dependencies</h2>
                <p>{dependencyCheckResult.platform_notes}</p>
              </div>
              <div className="wiz-stack">
                {dependencyCheckResult.dependencies
                  .filter((dep) => !dep.installed)
                  .map((dep) => (
                    <div key={dep.name} className="wiz-dep">
                      <div className="wiz-dep__name">{dep.name}</div>
                      <div className="wiz-muted wiz-small">{dep.description}</div>
                      {dep.install_commands.map((cmd) => (
                        <div key={cmd.distribution} className="wiz-cmd">
                          <span>{cmd.distribution}</span>
                          <code className="mono">{cmd.command}</code>
                        </div>
                      ))}
                    </div>
                  ))}
              </div>
              <p className="wiz-muted wiz-small">After installing the dependencies, click "Retry Check" to continue.</p>
            </>
          )}

          {/* Confirm Overwrite Step */}
          {step === "confirm" && (
            <>
              <div className="wiz__intro">
                <h2>Review installation</h2>
                <p>Some components are already installed. Keep them, or replace everything.</p>
              </div>
              <div className="list-box">
                {existingComponents.map((component) => {
                  const isDisabled = !enabledComponents[component.name];
                  const newVersion = versionOf(component.name) ?? component.version;
                  return (
                    <div key={component.name} className={`list-row${isDisabled ? " wiz-pkg--off" : ""}`}>
                      <span className="logo-tile logo-tile--sm">
                        <img src={LOGOS[component.name]} alt="" />
                      </span>
                      <div className="list-row__main">
                        <span>{component.displayName}</span>
                        <span className="wiz-muted">
                          {isDisabled ? "Skipped" : component.isExisting ? "Installed, will be replaced" : "New"}
                        </span>
                      </div>
                      <div className="wiz-ver mono">
                        {isDisabled ? (
                          <span className="wiz-muted">—</span>
                        ) : component.isExisting ? (
                          <>
                            <s className="wiz-muted">{component.version}</s>
                            <span className="wiz-muted">→</span>
                            <span className="text-ok">{newVersion || "Unknown"}</span>
                          </>
                        ) : (
                          <span className="text-ok">{newVersion || "Unknown"}</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
              {errorBox(true)}
            </>
          )}

          {/* Download Step */}
          {step === "download" && (
            <>
              <div className="wiz-dl">
                <div className="spinner spinner-lg" />
                <div className="wiz-dl__text">
                  <h2>
                    {getStepLabel()}
                    {progress.version && <span className="port-chip mono">{progress.version}</span>}
                  </h2>
                  <p className="wiz-muted">
                    {progress.step === "downloading" && "Downloading files to your computer..."}
                    {progress.step === "extracting" && "Extracting archive files..."}
                    {progress.step === "installing" && "Setting up components..."}
                    {progress.step === "error" && "Something went wrong."}
                  </p>
                </div>
                {progress.step === "downloading" && progress.totalBytes > 0 && (
                  <span className="wiz-dl__pct">{progress.percent}%</span>
                )}
              </div>

              <div className="progress-container wiz-progress">
                <div
                  className={
                    progress.step === "extracting" || progress.step === "installing"
                      ? "progress-bar-animated"
                      : "progress-bar"
                  }
                  style={{
                    width:
                      progress.step === "extracting" || progress.step === "installing"
                        ? "100%"
                        : `${progress.percent}%`,
                  }}
                />
              </div>

              {progress.step === "downloading" && progress.totalBytes > 0 && (
                <p className="wiz-muted wiz-small wiz-center mono">
                  {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
                </p>
              )}

              {errorBox(true)}
            </>
          )}

          {/* Complete Step */}
          {step === "complete" && (
            <>
              <div className="wiz-done">
                <span className="wiz-done__check" aria-hidden="true">✓</span>
                <h2>You're all set</h2>
                <p className="wiz-muted">Runtime binaries installed successfully.</p>
              </div>
              {progress.warning && (
                <div className="warn-box">
                  <div className="wiz-stack">
                    <span>⚠ {progress.warning}</span>
                    {isAntivirusError(progress.warning) && currentPlatform === "windows" && (
                      <button onClick={handleRetryNgrokFromBanner} disabled={isFixingAv} className="btn-primary btn-xs">
                        {isFixingAv ? "Adding exclusion…" : "Add Defender exclusion & retry ngrok"}
                      </button>
                    )}
                  </div>
                </div>
              )}
              <div className="tile-grid">
                {[
                  { name: "Caddy", key: "caddy" },
                  { name: "PHP", key: "php" },
                  { name: getDatabaseDisplayName(currentPlatform), key: dbKey },
                  { name: "phpMyAdmin", key: "phpmyadmin" },
                  { name: "PostgreSQL", key: "postgresql" },
                  { name: "Adminer", key: "adminer" },
                  { name: "ngrok", key: "ngrok" },
                ]
                  .filter((c) => enabledComponents[c.key] !== false)
                  .map((c) => (
                    <div key={c.key} className="tile">
                      <span className="logo-tile logo-tile--sm">
                        <img src={LOGOS[c.key]} alt="" />
                      </span>
                      <span className="tile__label">
                        <span>{c.name}</span>
                      </span>
                      <span className="mono text-ok">{versionOf(c.key)}</span>
                    </div>
                  ))}
              </div>
            </>
          )}
        </div>

        {/* Footer actions */}
        {step !== "download" && (
          <footer className="wiz__foot">
            {step === "welcome" && (
              <>
                {hasExistingOnWelcome && (
                  <button onClick={handleSkipFromWelcome} className="btn-secondary btn-sm">
                    Use Existing
                  </button>
                )}
                <button onClick={handleNext} className="btn-primary btn-sm">
                  {hasExistingOnWelcome ? "Download Fresh" : "Get Started"}
                </button>
              </>
            )}
            {step === "packages" && (
              <>
                <button onClick={handleBack} className="btn-secondary btn-sm wiz__back">
                  Back
                </button>
                <button onClick={startDownload} className="btn-primary btn-sm">
                  Download & Install
                </button>
              </>
            )}
            {step === "dependencies" && (
              <>
                <button onClick={handleBack} className="btn-secondary btn-sm wiz__back">
                  Back
                </button>
                <button onClick={startDownload} className="btn-primary btn-sm">
                  Retry Check
                </button>
              </>
            )}
            {step === "confirm" && (
              <>
                <button onClick={handleCancel} className="btn-secondary btn-sm wiz__back">
                  Back
                </button>
                <button onClick={handleSkipToDashboard} className="btn-secondary btn-sm">
                  Use Existing
                </button>
                <button onClick={handleSkipExisting} className="btn-secondary btn-sm">
                  Keep & Install
                </button>
                <button onClick={handleOverwriteAll} className="btn-primary btn-sm">
                  Install All
                </button>
              </>
            )}
            {step === "complete" && (
              <button onClick={onComplete} className="btn-primary btn-sm">
                Continue to Dashboard
              </button>
            )}
          </footer>
        )}
      </div>
    </div>
  );
}
