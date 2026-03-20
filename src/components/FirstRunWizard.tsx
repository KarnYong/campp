import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  DownloadProgress as DownloadProgressType,
  PackageSelection,
  DependencyCheckResult,
  getDatabaseDisplayName,
} from "../types/services";
import { PackageSelector } from "./PackageSelector";

// Helper to detect platform
const detectPlatform = (): string => {
  const userAgent = window.navigator.userAgent.toLowerCase();
  if (userAgent.includes("win")) return "windows";
  if (userAgent.includes("mac")) return "darwin";
  return "linux";
};

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
    { id: "php-8.5", version: "8.5.1" },
    { id: "php-8.4", version: "8.4.16" },
    { id: "php-8.3", version: "8.3.29" },
    { id: "php-8.2", version: "8.2.30" },
  ],
  mysql: [
    { id: "mysql-8.4", version: "8.4.0" },
    { id: "mysql-8.0", version: "8.0.40" },
  ],
  phpmyadmin: [
    { id: "phpmyadmin-5.2", version: "5.2.2" },
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
  const [packageSelection, setPackageSelection] = useState<PackageSelection>({
    php: "php-8.5",
    mysql: "mysql-8.4",
    phpmyadmin: "phpmyadmin-5.2",
  });
  const [existingComponents, setExistingComponents] = useState<ExistingComponent[]>([]);
  const [hasExistingOnWelcome, setHasExistingOnWelcome] = useState(false);
  const [dependencyCheckResult, setDependencyCheckResult] = useState<DependencyCheckResult | null>(null);

  // Check for existing components when welcome step loads
  useEffect(() => {
    const checkExisting = async () => {
      try {
        const existing = await invoke<Record<string, string>>("check_existing_components");
        const hasExisting = Object.keys(existing).length > 0;
        setHasExistingOnWelcome(hasExisting);
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
          name: "mysql",
          version: existing.mysql || "",
          displayName: getDatabaseDisplayName(currentPlatform),
          isExisting: !!existing.mysql,
        },
        {
          name: "phpmyadmin",
          version: existing.phpmyadmin || "",
          displayName: "phpMyAdmin",
          isExisting: !!existing.phpmyadmin,
        },
      ];

      setExistingComponents(allComponents);
      setStep("confirm");
      return;
    } catch (err) {
      console.error("Failed to check existing components:", err);
    }

    // No existing components, proceed with download
    proceedWithDownload([]);
  };

  const proceedWithDownload = async (skipList: string[]) => {
    setError(null);
    setStep("download");

    try {
      // Save the package selection to settings
      await invoke("update_package_selection", { packageSelection });

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
        alert("No existing installation found. Please download the runtime components to continue.");
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

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
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

  const currentStepNum = getStepNumber();

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "1rem",
        backgroundColor: "var(--bg-app)",
        color: "var(--text-primary)",
      }}
      {...props}
    >
      <div
        style={{
          backgroundColor: "var(--bg-card)",
          border: "1px solid var(--border-color)",
          borderRadius: "0.75rem",
          padding: "1.25rem",
          maxWidth: "28rem",
          width: "100%",
          boxShadow: "0 4px 12px rgba(0, 0, 0, 0.1)",
        }}
      >
        {/* Header */}
        <div style={{ textAlign: "center", marginBottom: "1rem" }}>
          <h1 style={{ fontSize: "1.5rem", fontWeight: 600, marginBottom: "0.25rem" }}>
            Welcome to CAMPP
          </h1>
          <p style={{ fontSize: "0.875rem", color: "var(--text-secondary)" }}>
            Local Web Development Stack
          </p>

          {/* Compact Step Indicator */}
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "0.25rem", marginTop: "0.75rem" }}>
            {[1, 2, 3, 4, 5].map((stepNum) => (
              <div key={stepNum} style={{ display: "flex", alignItems: "center", gap: "0.25rem" }}>
                <div style={{
                  width: "1.25rem",
                  height: "1.25rem",
                  borderRadius: "50%",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: "0.625rem",
                  fontWeight: 500,
                  color: currentStepNum >= stepNum ? "white" : "var(--text-secondary)",
                  backgroundColor: currentStepNum > stepNum ? "var(--color-success)" : currentStepNum >= stepNum ? "var(--bg-card-secondary)" : "transparent",
                  border: currentStepNum < stepNum ? "1px solid var(--border-color)" : "none",
                }}>
                  {stepNum}
                </div>
                {stepNum < 5 && (
                  <div style={{ width: "1rem", height: "1px", backgroundColor: currentStepNum > stepNum ? "var(--color-success)" : "var(--border-color)" }} />
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Content */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          {/* Welcome Step */}
          {step === "welcome" && (
            <div>
              <p style={{ fontSize: "0.875rem", lineHeight: 1.5, color: "var(--text-primary)", marginBottom: "0.75rem" }}>
                CAMPP requires runtime binaries (Caddy, PHP-FPM, {getDatabaseDisplayName(currentPlatform)}, and phpMyAdmin)
                to be installed on your system.
              </p>
              {hasExistingOnWelcome ? (
                <div
                  style={{
                    padding: "0.5rem",
                    borderRadius: "0.375rem",
                    backgroundColor: "rgba(16, 185, 129, 0.1)",
                    border: "1px solid var(--color-success)",
                    marginBottom: "0.75rem",
                    fontSize: "0.875rem",
                  }}
                >
                  <strong>Existing installation detected!</strong>
                </div>
              ) : (
                <div className="info-box" style={{ marginBottom: "0.75rem", padding: "0.5rem", fontSize: "0.875rem" }}>
                  <strong>Estimated download size:</strong> ~150 MB
                </div>
              )}
              <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                {hasExistingOnWelcome && (
                  <button
                    onClick={handleSkipFromWelcome}
                    className="btn-secondary"
                    style={{ fontSize: "0.875rem", padding: "0.5rem 1rem", borderColor: "var(--color-primary)", color: "var(--color-primary)" }}
                  >
                    Use Existing
                  </button>
                )}
                <button onClick={handleNext} className="btn-primary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  {hasExistingOnWelcome ? "Download Fresh" : "Get Started"}
                </button>
                <button
                  onClick={async () => {
                    try {
                      await invoke("open_manual");
                    } catch (err) {
                      console.error("Failed to open manual:", err);
                    }
                  }}
                  className="btn-secondary"
                  title="Read User Manual"
                  style={{ fontSize: "0.875rem", padding: "0.5rem 0.75rem" }}
                >
                  ?
                </button>
              </div>
            </div>
          )}

          {/* Package Selection Step */}
          {step === "packages" && (
            <div>
              <p style={{ fontSize: "0.875rem", marginBottom: "0.75rem" }}>
                Select the versions of PHP, {getDatabaseDisplayName(currentPlatform)}, and phpMyAdmin.
              </p>
              <PackageSelector
                onSelectionChange={handlePackageChange}
                initialSelection={packageSelection}
              />
              <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem" }}>
                <button onClick={handleBack} className="btn-secondary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  Back
                </button>
                <button onClick={startDownload} className="btn-primary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  Download & Install
                </button>
              </div>
            </div>
          )}

          {/* Dependencies Step */}
          {step === "dependencies" && dependencyCheckResult && (
            <div>
              <p style={{ fontSize: "0.875rem", marginBottom: "0.5rem", color: "var(--color-error)", fontWeight: 600 }}>
                Missing System Dependencies
              </p>
              <p style={{ fontSize: "0.875rem", marginBottom: "0.75rem", color: "var(--text-secondary)" }}>
                {dependencyCheckResult.platform_notes}
              </p>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.5rem",
                  margin: "0.5rem 0",
                  padding: "0.5rem",
                  backgroundColor: "rgba(239, 68, 68, 0.1)",
                  borderRadius: "0.375rem",
                  border: "1px solid var(--color-error)",
                }}
              >
                {dependencyCheckResult.dependencies
                  .filter((dep) => !dep.installed)
                  .map((dep) => (
                    <div key={dep.name} style={{ marginBottom: "0.5rem" }}>
                      <div style={{ fontWeight: 600, marginBottom: "0.25rem", fontSize: "0.875rem" }}>
                        {dep.name}
                      </div>
                      <div style={{ fontSize: "0.8125rem", color: "var(--text-secondary)", marginBottom: "0.375rem" }}>
                        {dep.description}
                      </div>
                      <div style={{ fontSize: "0.8125rem", fontWeight: 500, marginBottom: "0.25rem" }}>
                        Install command for your distribution:
                      </div>
                      <div
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: "0.25rem",
                        }}
                      >
                        {dep.install_commands.map((cmd) => (
                          <div
                            key={cmd.distribution}
                            style={{
                              backgroundColor: "var(--bg-card)",
                              padding: "0.375rem 0.5rem",
                              borderRadius: "0.25rem",
                              fontSize: "0.75rem",
                            }}
                          >
                            <div style={{ fontWeight: 600, marginBottom: "0.125rem" }}>{cmd.distribution}:</div>
                            <code
                              style={{
                                fontFamily: "monospace",
                                fontSize: "0.75rem",
                                wordBreak: "break-all",
                              }}
                            >
                              {cmd.command}
                            </code>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
              </div>
              <p style={{ fontSize: "0.8125rem", color: "var(--text-secondary)", marginBottom: "0.75rem" }}>
                After installing the dependencies, click "Retry Check" to continue.
              </p>
              <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem" }}>
                <button onClick={handleBack} className="btn-secondary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  Back
                </button>
                <button onClick={startDownload} className="btn-primary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  Retry Check
                </button>
              </div>
            </div>
          )}

          {/* Confirm Overwrite Step */}
          {step === "confirm" && (
            <div>
              <p style={{ fontSize: "0.875rem", marginBottom: "0.5rem" }}>
                Installation summary:
              </p>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.375rem",
                  margin: "0.5rem 0",
                  padding: "0.5rem",
                  backgroundColor: "var(--bg-card-secondary)",
                  borderRadius: "0.375rem",
                  border: "1px solid var(--border-color)",
                }}
              >
                {existingComponents.map((component) => {
                  const newVersion = component.name === "php"
                    ? packages.php.find(p => p.id === packageSelection.php)?.version
                    : component.name === "mysql"
                    ? packages.mysql.find(p => p.id === packageSelection.mysql)?.version
                    : component.name === "phpmyadmin"
                    ? packages.phpmyadmin.find(p => p.id === packageSelection.phpmyadmin)?.version
                    : component.name === "caddy"
                    ? "2.8.4"
                    : component.version;

                  return (
                    <div
                      key={component.name}
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        padding: "0.375rem",
                        backgroundColor: "var(--bg-card)",
                        borderRadius: "0.25rem",
                        fontSize: "0.875rem",
                        border: component.isExisting ? "1px solid var(--color-warning)" : "1px solid transparent",
                      }}
                    >
                      <span style={{ fontWeight: 500 }}>
                        {component.displayName}
                        {!component.isExisting && (
                          <span style={{ fontSize: "0.7rem", color: "var(--color-success)", marginLeft: "0.375rem", fontWeight: 400 }}>
                            (New)
                          </span>
                        )}
                      </span>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.375rem" }}>
                        {component.isExisting ? (
                          <>
                            <span style={{ fontSize: "0.75rem", color: "var(--color-error)", textDecoration: "line-through" }}>
                              {component.version}
                            </span>
                            <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)" }}>→</span>
                            <span style={{ fontSize: "0.75rem", color: "var(--color-success)", fontWeight: 500 }}>
                              {newVersion || "Unknown"}
                            </span>
                          </>
                        ) : (
                          <span style={{ fontSize: "0.75rem", color: "var(--color-success)", fontWeight: 500 }}>
                            {newVersion || "Unknown"}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
              {error && (
                <div className="error-box" style={{ marginBottom: "0.5rem", padding: "0.5rem", fontSize: "0.875rem" }}>
                  <p className="error-box-text" style={{ margin: "0 0 0.375rem 0" }}>
                    <strong>Error:</strong> {error}
                  </p>
                  <button
                    onClick={() => setError(null)}
                    style={{
                      padding: "0.25rem 0.5rem",
                      borderRadius: "0.25rem",
                      border: "1px solid var(--color-error)",
                      backgroundColor: "transparent",
                      color: "var(--color-error)",
                      fontSize: "0.75rem",
                      cursor: "pointer",
                    }}
                  >
                    Dismiss
                  </button>
                </div>
              )}
              <div style={{ display: "flex", justifyContent: "center", gap: "0.375rem", flexWrap: "wrap" }}>
                <button onClick={handleCancel} className="btn-secondary" style={{ fontSize: "0.875rem", padding: "0.5rem 0.75rem" }}>
                  Back
                </button>
                <button onClick={handleSkipToDashboard} className="btn-secondary" style={{ fontSize: "0.875rem", padding: "0.5rem 0.75rem", borderColor: "var(--color-success)", color: "var(--color-success)" }}>
                  Use Existing
                </button>
                <button onClick={handleSkipExisting} className="btn-secondary" style={{ fontSize: "0.875rem", padding: "0.5rem 0.75rem" }}>
                  Keep & Install
                </button>
                <button onClick={handleOverwriteAll} className="btn-primary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                  Install All
                </button>
              </div>
            </div>
          )}

          {/* Download Step */}
          {step === "download" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              {/* Progress Header */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <h3 style={{ fontSize: "1rem", fontWeight: 600 }}>{getStepLabel()}</h3>
                {progress.step === "downloading" && (
                  <span style={{ fontSize: "1rem", fontWeight: 600, color: "var(--color-primary)" }}>
                    {progress.percent}%
                  </span>
                )}
              </div>

              {/* Current Component Info */}
              {progress.componentDisplay && (
                <div
                  style={{
                    backgroundColor: "var(--bg-card-secondary)",
                    borderRadius: "0.375rem",
                    padding: "0.5rem",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                    <span style={{ fontSize: "0.875rem", fontWeight: 600 }}>
                      {progress.currentComponent || progress.componentDisplay}
                    </span>
                    {progress.version && (
                      <span
                        style={{
                          padding: "0.125rem 0.375rem",
                          backgroundColor: "var(--color-primary)",
                          borderRadius: "0.25rem",
                          fontSize: "0.75rem",
                          fontWeight: 600,
                          color: "white",
                        }}
                      >
                        {progress.version}
                      </span>
                    )}
                  </div>
                </div>
              )}

              {/* Progress Bar */}
              <div className="progress-container">
                <div
                  className={progress.step === "extracting" || progress.step === "installing" ? "progress-bar-animated" : "progress-bar"}
                  style={{
                    width: progress.step === "extracting" || progress.step === "installing"
                      ? "100%"
                      : `${progress.percent}%`,
                  }}
                />
              </div>

              {/* Download Details */}
              {progress.step === "downloading" && progress.totalBytes > 0 && (
                <div style={{ textAlign: "center", fontSize: "0.875rem", color: "var(--text-secondary)" }}>
                  <span>
                    {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
                  </span>
                </div>
              )}

              {/* Error Display */}
              {error && (
                <div className="error-box" style={{ padding: "0.5rem", fontSize: "0.875rem" }}>
                  <p className="error-box-text" style={{ margin: "0 0 0.375rem 0" }}>Error: {error}</p>
                  <button
                    onClick={() => setError(null)}
                    style={{
                      padding: "0.25rem 0.5rem",
                      borderRadius: "0.25rem",
                      border: "1px solid var(--color-error)",
                      backgroundColor: "transparent",
                      color: "var(--color-error)",
                      fontSize: "0.75rem",
                      cursor: "pointer",
                    }}
                  >
                    Dismiss
                  </button>
                </div>
              )}
            </div>
          )}

          {/* Complete Step */}
          {step === "complete" && (
            <div style={{ textAlign: "center" }}>
              <p style={{ marginBottom: "0.75rem", color: "var(--color-success)", fontWeight: 500, fontSize: "0.875rem" }}>
                Runtime binaries installed successfully!
              </p>
              <div
                style={{
                  display: "flex",
                  flexWrap: "wrap",
                  gap: "0.5rem",
                  justifyContent: "center",
                  marginBottom: "1rem",
                }}
              >
                {[
                  { name: "Caddy", version: "2.8.4" },
                  { name: "PHP", version: packages.php.find(p => p.id === packageSelection.php)?.version || "8.5.1" },
                  { name: getDatabaseDisplayName(currentPlatform), version: packages.mysql.find(p => p.id === packageSelection.mysql)?.version || "8.4.0" },
                  { name: "phpMyAdmin", version: packages.phpmyadmin.find(p => p.id === packageSelection.phpmyadmin)?.version || "5.2.2" },
                ].map((pkg) => (
                  <div
                    key={pkg.name}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "0.375rem",
                      padding: "0.375rem 0.5rem",
                      backgroundColor: "var(--bg-card-secondary)",
                      borderRadius: "0.375rem",
                      fontSize: "0.875rem",
                    }}
                  >
                    <span style={{ fontWeight: 500 }}>{pkg.name}</span>
                    <span
                      style={{
                        padding: "0.125rem 0.375rem",
                        backgroundColor: "var(--color-success)",
                        borderRadius: "0.25rem",
                        fontSize: "0.75rem",
                        fontWeight: 500,
                        color: "white",
                      }}
                    >
                      {pkg.version}
                    </span>
                  </div>
                ))}
              </div>
              <button onClick={onComplete} className="btn-primary" style={{ fontSize: "0.875rem", padding: "0.5rem 1rem" }}>
                Continue to Dashboard
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
