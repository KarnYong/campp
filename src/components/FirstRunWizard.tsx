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

  useEffect(() => {
    // Listen for download progress events
    const unlisten = listen<DownloadProgressType>("download-progress", (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
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
        return "Downloading runtime binaries...";
      case "extracting":
        return "Extracting files...";
      case "installing":
        return "Installing components...";
      case "complete":
        return "Installation complete!";
      case "error":
        return "An error occurred";
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
                <span className="progress-percent">{progress.percent}%</span>
              </div>

              {progress.currentComponent && (
                <div className="current-component">
                  <span className="component-label">Downloading:</span>
                  <span className="component-name">
                    {progress.componentDisplay || progress.currentComponent}
                  </span>
                  {progress.version && (
                    <span className="component-version">{progress.version}</span>
                  )}
                </div>
              )}

              <div className="progress-bar-container">
                <div
                  className="progress-bar"
                  style={{ width: `${progress.percent}%` }}
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
