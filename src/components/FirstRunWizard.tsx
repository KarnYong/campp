import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DownloadProgress as DownloadProgressType } from "../types/services";

interface FirstRunWizardProps {
  onComplete: () => void;
}

export function FirstRunWizard({ onComplete }: FirstRunWizardProps) {
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
  const [isDownloading, setIsDownloading] = useState(false);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgressType>("download-progress", (event) => {
      setProgress(event.payload);
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
    setIsDownloading(true);
    setError(null);

    try {
      const result = await invoke<string>("download_runtime");
      console.log(result);
      onComplete();
    } catch (err) {
      setError(err as string);
      setIsDownloading(false);
    }
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
        </div>

        <div className="wizard-content">
          <div className="wizard-message">
            <p>
              CAMPP requires runtime binaries (Caddy, PHP-FPM, MariaDB, and phpMyAdmin)
              to be installed on your system. These will be downloaded and extracted to
              your local data directory.
            </p>
            <p className="wizard-info">
              <strong>Estimated download size:</strong> ~150 MB
            </p>
          </div>

          {error && (
            <div className="error-message">
              <p>Error: {error}</p>
              <button onClick={() => setError(null)}>Dismiss</button>
            </div>
          )}

          {!isDownloading && !error && (
            <div className="wizard-actions">
              <button onClick={startDownload} className="btn-primary">
                Download and Install
              </button>
            </div>
          )}

          {isDownloading && (
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

              {progress.step === "complete" && (
                <div className="complete-message">
                  <p>Runtime binaries installed successfully!</p>
                  <div className="installed-components">
                    <div className="installed-component">
                      <span className="installed-name">Caddy</span>
                      <span className="installed-version">2.8.4</span>
                    </div>
                    <div className="installed-component">
                      <span className="installed-name">PHP</span>
                      <span className="installed-version">8.5.1</span>
                    </div>
                    <div className="installed-component">
                      <span className="installed-name">MariaDB</span>
                      <span className="installed-version">12.2.2</span>
                    </div>
                    <div className="installed-component">
                      <span className="installed-name">phpMyAdmin</span>
                      <span className="installed-version">5.2.2</span>
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
          )}
        </div>
      </div>
    </div>
  );
}
