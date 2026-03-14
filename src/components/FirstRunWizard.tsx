import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DownloadProgress as DownloadProgressType, PackageSelection } from "../types/services";
import { PackageSelector } from "./PackageSelector";

interface FirstRunWizardProps {
  onComplete: () => void;
}

type WizardStep = "welcome" | "packages" | "confirm" | "download" | "complete";

interface ExistingComponent {
  name: string;
  version: string;
  displayName: string;
}

export function FirstRunWizard({ onComplete }: FirstRunWizardProps) {
  const [step, setStep] = useState<WizardStep>("welcome");
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
  const [packageSelection, setPackageSelection] = useState<PackageSelection>({
    php: "php-8.4",
    mariadb: "mariadb-12",
    phpmyadmin: "phpmyadmin-5.2",
  });
  const [existingComponents, setExistingComponents] = useState<ExistingComponent[]>([]);

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

    // First, check for existing components
    try {
      const existing = await invoke<Record<string, string>>("check_existing_components");

      // Convert to array of ExistingComponent
      const existingList: ExistingComponent[] = Object.entries(existing).map(([name, version]) => ({
        name,
        version,
        displayName: name === "caddy" ? "Caddy" :
                      name === "php" ? "PHP" :
                      name === "mariadb" ? "MariaDB" :
                      name === "phpmyadmin" ? "phpMyAdmin" : name,
      }));

      if (existingList.length > 0) {
        setExistingComponents(existingList);
        setStep("confirm");
        return;
      }
    } catch (err) {
      console.error("Failed to check existing components:", err);
    }

    // No existing components, proceed with download
    proceedWithDownload([]);
  };

  const proceedWithDownload = async (skipList: string[]) => {
    setError(null); // Clear any previous errors
    setStep("download");

    try {
      // Save the package selection to settings
      await invoke("update_package_selection", { packageSelection });

      if (skipList.length > 0) {
        // Download with skip list
        const result = await invoke<string>("download_runtime_with_skip", {
          packageSelection,
          skipList,
        });
        console.log(result);
      } else {
        // Download all
        const result = await invoke<string>("download_runtime_with_packages", { packageSelection });
        console.log(result);
      }
    } catch (err) {
      console.error("Download error:", err);
      setError(err as string);
      // Keep user on confirm step to see error and retry, don't reset to packages
      setStep("confirm");
    }
  };

  const handleOverwriteAll = () => {
    proceedWithDownload([]);
  };

  const handleSkipExisting = () => {
    const skipList = existingComponents.map(c => c.name);
    proceedWithDownload(skipList);
  };

  const handleCancel = () => {
    setStep("packages");
    setExistingComponents([]);
  };

  const handleNext = () => {
    if (step === "welcome") {
      setStep("packages");
    }
  };

  const handleBack = () => {
    if (step === "packages") {
      setStep("welcome");
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

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  };

  return (
    <div className="first-run-wizard">
      <div className="wizard-container">
        <div className="wizard-header">
          <h1>Welcome to CAMPP</h1>
          <p className="wizard-subtitle">
            Local Web Development Stack
          </p>
          {/* Step indicator */}
          <div className="wizard-steps">
            <div className={`wizard-step ${step === "welcome" || step === "packages" || step === "confirm" || step === "download" || step === "complete" ? "active" : ""} ${step === "download" || step === "complete" ? "completed" : ""}`}>
              <div className="step-number">1</div>
              <div className="step-label">Welcome</div>
            </div>
            <div className="wizard-step-line"></div>
            <div className={`wizard-step ${step === "packages" || step === "confirm" || step === "download" || step === "complete" ? "active" : ""} ${step === "download" || step === "complete" ? "completed" : ""}`}>
              <div className="step-number">2</div>
              <div className="step-label">Packages</div>
            </div>
            <div className="wizard-step-line"></div>
            <div className={`wizard-step ${step === "confirm" || step === "download" || step === "complete" ? "active" : ""} ${step === "download" || step === "complete" ? "completed" : ""}`}>
              <div className="step-number">3</div>
              <div className="step-label">Confirm</div>
            </div>
            <div className="wizard-step-line"></div>
            <div className={`wizard-step ${step === "download" || step === "complete" ? "active" : ""} ${step === "complete" ? "completed" : ""}`}>
              <div className="step-number">4</div>
              <div className="step-label">Download</div>
            </div>
          </div>
        </div>

        <div className="wizard-content">
          {/* Welcome Step */}
          {step === "welcome" && (
            <div className="wizard-message">
              <p>
                CAMPP requires runtime binaries (Caddy, PHP-FPM, MariaDB, and phpMyAdmin)
                to be installed on your system. These will be downloaded and extracted to
                your local data directory.
              </p>
              <p className="wizard-info">
                <strong>Estimated download size:</strong> ~150 MB
              </p>
              <div className="wizard-actions">
                <button onClick={handleNext} className="btn-primary">
                  Get Started
                </button>
                <button
                  onClick={async () => {
                    try {
                      await invoke("open_manual");
                    } catch (err) {
                      console.error("Failed to open manual:", err);
                    }
                  }}
                  className="btn-help"
                  title="Read User Manual"
                >
                  ?
                </button>
              </div>
            </div>
          )}

          {/* Package Selection Step */}
          {step === "packages" && (
            <div className="wizard-message">
              <p>
                Select the versions of PHP, MariaDB, and phpMyAdmin you want to install.
                The latest stable versions are recommended for new projects.
              </p>
              <PackageSelector
                onSelectionChange={handlePackageChange}
                initialSelection={packageSelection}
              />
              <div className="wizard-actions">
                <button onClick={handleBack} className="btn-secondary">
                  Back
                </button>
                <button onClick={startDownload} className="btn-primary">
                  Download & Install
                </button>
              </div>
            </div>
          )}

          {/* Confirm Overwrite Step */}
          {step === "confirm" && (
            <div className="wizard-message">
              <p>
                The following components are already installed and will be replaced:
              </p>
              <div className="existing-components-list">
                {existingComponents.map((component) => {
                  // Find the new version for this component
                  const newVersion = component.name === "php"
                    ? packages.php.find(p => p.id === packageSelection.php)?.version
                    : component.name === "mariadb"
                    ? packages.mariadb.find(p => p.id === packageSelection.mariadb)?.version
                    : component.name === "phpmyadmin"
                    ? packages.phpmyadmin.find(p => p.id === packageSelection.phpmyadmin)?.version
                    : component.name === "caddy"
                    ? "2.8.4"
                    : component.version;

                  return (
                    <div key={component.name} className="existing-component-item">
                      <span className="existing-component-name">{component.displayName}</span>
                      <span className="version-replacement">
                        <span className="existing-component-version">{component.version}</span>
                        <span className="version-arrow"> → </span>
                        <span className="new-component-version">{newVersion || "Unknown"}</span>
                      </span>
                    </div>
                  );
                })}
              </div>
              <p className="wizard-info">
                <strong>What would you like to do?</strong>
              </p>
              {error && (
                <div className="error-message">
                  <p><strong>Error:</strong> {error}</p>
                  <button onClick={() => setError(null)} className="btn-dismiss">Dismiss</button>
                </div>
              )}
              <div className="wizard-actions">
                <button onClick={handleCancel} className="btn-secondary">
                  Cancel
                </button>
                <button onClick={handleSkipExisting} className="btn-secondary">
                  Skip Existing
                </button>
                <button onClick={handleOverwriteAll} className="btn-primary">
                  Overwrite All
                </button>
              </div>
            </div>
          )}

          {/* Download Step */}
          {step === "download" && (
            <div className="download-progress">
              <div className="progress-header">
                <h3>{getStepLabel()}</h3>
                {progress.step === "downloading" && (
                  <span className="progress-percent">{progress.percent}%</span>
                )}
              </div>

              {progress.componentDisplay && (
                <div className="current-component-info">
                  <div className="component-info-main">
                    <span className="component-name">
                      {progress.currentComponent || progress.componentDisplay}
                    </span>
                    {progress.version && (
                      <span className="component-version">{progress.version}</span>
                    )}
                  </div>
                </div>
              )}

              <div className="progress-bar-container">
                <div
                  className={`progress-bar ${progress.step === "extracting" || progress.step === "installing" ? "progress-animated" : ""}`}
                  style={{
                    width: progress.step === "extracting" || progress.step === "installing"
                      ? "100%"
                      : `${progress.percent}%`
                  }}
                />
              </div>

              {progress.step === "downloading" && progress.totalBytes > 0 && (
                <div className="download-details">
                  <span>
                    {formatBytes(progress.downloadedBytes)} /{" "}
                    {formatBytes(progress.totalBytes)}
                  </span>
                </div>
              )}

              {error && (
                <div className="error-message">
                  <p>Error: {error}</p>
                  <button onClick={() => setError(null)}>Dismiss</button>
                </div>
              )}
            </div>
          )}

          {/* Complete Step */}
          {step === "complete" && (
            <div className="complete-message">
              <p>Runtime binaries installed successfully!</p>
              <div className="installed-components">
                <div className="installed-component">
                  <span className="installed-name">Caddy</span>
                  <span className="installed-version">2.8.4</span>
                </div>
                <div className="installed-component">
                  <span className="installed-name">PHP</span>
                  <span className="installed-version">{packages.php.find(p => p.id === packageSelection.php)?.version || "8.5.1"}</span>
                </div>
                <div className="installed-component">
                  <span className="installed-name">MariaDB</span>
                  <span className="installed-version">{packages.mariadb.find(p => p.id === packageSelection.mariadb)?.version || "11.4.5"}</span>
                </div>
                <div className="installed-component">
                  <span className="installed-name">phpMyAdmin</span>
                  <span className="installed-version">{packages.phpmyadmin.find(p => p.id === packageSelection.phpmyadmin)?.version || "5.2.2"}</span>
                </div>
              </div>
              <p className="shortcut-hint">
                Press <kbd>Ctrl+Shift+D</kbd> (Mac: <kbd>Cmd+Shift+D</kbd>) to open download folder
              </p>
              <button onClick={onComplete} className="btn-primary">
                Continue to Dashboard
              </button>
            </div>
          )}
        </div>
      </div>

      <style>{`
        .wizard-steps {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          margin-top: 1rem;
        }
        .wizard-step {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.25rem;
          opacity: 0.5;
          transition: opacity 0.3s;
        }
        .wizard-step.active {
          opacity: 1;
        }
        .wizard-step.completed {
          opacity: 1;
        }
        .wizard-step.completed .step-number {
          background: #10b981;
        }
        .step-number {
          width: 28px;
          height: 28px;
          border-radius: 50%;
          background: #374151;
          color: #fff;
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 0.875rem;
          font-weight: 500;
        }
        .step-label {
          font-size: 0.75rem;
          color: #9ca3af;
        }
        .wizard-step.active .step-label {
          color: #fff;
        }
        .wizard-step-line {
          width: 40px;
          height: 2px;
          background: #374151;
        }
        .wizard-step.active + .wizard-step-line,
        .wizard-step.completed + .wizard-step-line {
          background: #10b981;
        }
        .package-selector {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
          margin: 1.5rem 0;
        }
        .package-group {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }
        .package-label {
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
          font-size: 0.875rem;
          font-weight: 500;
          color: #d1d5db;
        }
        .package-label-hint {
          font-size: 0.75rem;
          font-weight: 400;
          color: #9ca3af;
        }
        .package-select {
          padding: 0.75rem;
          border-radius: 0.5rem;
          border: 1px solid #374151;
          background: #1f2937;
          color: #f9fafb;
          font-size: 0.875rem;
          cursor: pointer;
        }
        .package-select:hover {
          border-color: #4b5563;
        }
        .package-select:focus {
          outline: none;
          border-color: #3b82f6;
          box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
        }
        .package-info {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
          padding: 1rem;
          background: #1f2937;
          border-radius: 0.5rem;
          border: 1px solid #374151;
        }
        .package-info-text {
          font-size: 0.875rem;
          color: #9ca3af;
          margin: 0;
        }
        .btn-secondary {
          padding: 0.75rem 1.5rem;
          border-radius: 0.5rem;
          border: 1px solid #374151;
          background: transparent;
          color: #f9fafb;
          font-size: 0.875rem;
          font-weight: 500;
          cursor: pointer;
          transition: background 0.2s;
        }
        .btn-secondary:hover {
          background: #374151;
        }
        .existing-components-list {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
          margin: 1.5rem 0;
          padding: 1rem;
          background: #1f2937;
          border-radius: 0.5rem;
          border: 1px solid #374151;
        }
        .existing-component-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.75rem;
          background: #374151;
          border-radius: 0.375rem;
        }
        .existing-component-name {
          font-size: 0.875rem;
          font-weight: 500;
          color: #f9fafb;
        }
        .version-replacement {
          display: flex;
          align-items: center;
          gap: 0.5rem;
        }
        .existing-component-version {
          font-size: 0.75rem;
          color: #ef4444;
          padding: 0.25rem 0.5rem;
          background: #1f2937;
          border-radius: 0.25rem;
          text-decoration: line-through;
        }
        .version-arrow {
          font-size: 0.75rem;
          color: #9ca3af;
        }
        .new-component-version {
          font-size: 0.75rem;
          color: #10b981;
          padding: 0.25rem 0.5rem;
          background: #1f2937;
          border-radius: 0.25rem;
          font-weight: 500;
        }
        .error-message {
          padding: 1rem;
          background: rgba(239, 68, 68, 0.2);
          border: 1px solid #ef4444;
          border-radius: 0.5rem;
          margin: 1rem 0;
        }
        .error-message p {
          margin: 0 0 0.5rem 0;
          color: #fca5a5;
          font-size: 0.875rem;
        }
        .btn-dismiss {
          padding: 0.375rem 0.75rem;
          border-radius: 0.375rem;
          border: 1px solid #ef4444;
          background: transparent;
          color: #fca5a5;
          font-size: 0.75rem;
          cursor: pointer;
          transition: background 0.2s;
        }
        .btn-dismiss:hover {
          background: rgba(239, 68, 68, 0.2);
        }
      `}</style>
    </div>
  );
}

// Import packages for displaying versions
const packages = {
  php: [
    { id: "php-8.5", version: "8.5.1" },
    { id: "php-8.4", version: "8.4.16" },
    { id: "php-8.3", version: "8.3.29" },
    { id: "php-8.2", version: "8.2.30" },
    { id: "php-7.4", version: "7.4.33" },
  ],
  mariadb: [
    { id: "mariadb-12", version: "12.3.1" },
    { id: "mariadb-11.8", version: "11.8.6" },
    { id: "mariadb-10.9", version: "10.9.8" },
  ],
  phpmyadmin: [
    { id: "phpmyadmin-5.2", version: "5.2.2" },
    { id: "phpmyadmin-5.1", version: "5.1.4" },
  ],
};
