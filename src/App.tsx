import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Dashboard } from "./components/Dashboard";
import { FirstRunWizard } from "./components/FirstRunWizard";
import "./App.css";

function App() {
  const [isFirstRun, setIsFirstRun] = useState<boolean | null>(null);

  useEffect(() => {
    checkRuntimeInstalled();

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

  if (isFirstRun === null) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--text-secondary)",
        }}
      >
        <p style={{ fontSize: "1.25rem" }}>Loading...</p>
      </div>
    );
  }

  if (isFirstRun) {
    return <FirstRunWizard onComplete={handleWizardComplete} />;
  }

  return (
    <>
      <Dashboard />
    </>
  );
}

export default App;
