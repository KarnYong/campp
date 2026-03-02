import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Dashboard } from "./components/Dashboard";
import { FirstRunWizard } from "./components/FirstRunWizard";
import "./App.css";

function App() {
  const [isFirstRun, setIsFirstRun] = useState<boolean | null>(null);
  const [showDebugMenu, setShowDebugMenu] = useState(false);

  useEffect(() => {
    checkRuntimeInstalled();

    // Debug mode: press Ctrl+Shift+D to toggle debug menu
    const handleKeyPress = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === "D") {
        setShowDebugMenu((prev) => !prev);
      }
    };

    window.addEventListener("keydown", handleKeyPress);

    // Listen for show-wizard event from menu
    const unlisten = listen("show-wizard", () => {
      setIsFirstRun(true);
    });

    // Cleanup all services when window closes
    const handleBeforeUnload = () => {
      invoke("cleanup_all_services").catch((error) => {
        console.error("Failed to cleanup services:", error);
      });
    };

    window.addEventListener("beforeunload", handleBeforeUnload);

    return () => {
      window.removeEventListener("keydown", handleKeyPress);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      unlisten.then((fn) => fn());
    };
  }, []);

  const checkRuntimeInstalled = async () => {
    try {
      const installed = await invoke<boolean>("check_runtime_installed");
      setIsFirstRun(!installed);
    } catch (error) {
      console.error("Failed to check runtime status:", error);
      // Default to showing wizard if check fails
      setIsFirstRun(true);
    }
  };

  const handleWizardComplete = () => {
    setIsFirstRun(false);
  };

  const handleResetInstallation = async () => {
    if (confirm("Reset installation? This will stop all services and delete runtime binaries.")) {
      try {
        // Stop all services first
        await invoke("cleanup_all_services");
        await invoke("reset_installation");
        setIsFirstRun(true);
        setShowDebugMenu(false);
      } catch (error) {
        console.error("Failed to reset:", error);
        alert("Failed to reset: " + error);
      }
    }
  };

  const handleOpenRuntimeFolder = async () => {
    try {
      const runtimeDir = await invoke<string>("get_runtime_dir");
      await revealItemInDir(runtimeDir);
    } catch (error) {
      console.error("Failed to open folder:", error);
      alert("Failed to open folder: " + error);
    }
  };

  const handleOpenDownloadFolder = async () => {
    try {
      const downloadDir = await invoke<string>("get_download_dir");
      await invoke("open_folder", { path: downloadDir });
    } catch (error) {
      console.error("Failed to open download folder:", error);
      alert("Failed to open download folder: " + error);
    }
  };

  if (isFirstRun === null) {
    return (
      <div className="loading-screen">
        <p>Loading...</p>
      </div>
    );
  }

  if (isFirstRun) {
    return <FirstRunWizard onComplete={handleWizardComplete} />;
  }

  return (
    <>
      {showDebugMenu && (
        <div className="debug-menu">
          <div className="debug-header">
            <span>Debug Menu</span>
            <button onClick={() => setShowDebugMenu(false)}>×</button>
          </div>
          <div className="debug-items">
            <button onClick={handleOpenRuntimeFolder}>Open Runtime Folder</button>
            <button onClick={handleOpenDownloadFolder}>View Download Folder</button>
            <button onClick={handleResetInstallation}>Reset Installation</button>
            <button onClick={() => setIsFirstRun(true)}>Show First-Run Wizard</button>
          </div>
        </div>
      )}
      <Dashboard />
    </>
  );
}

export default App;
