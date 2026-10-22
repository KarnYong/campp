import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Icon } from "./icons";
import { applyTheme, themeLabel, type ThemeMode } from "../utils/theme";

const MODES: { mode: ThemeMode; icon: "monitor" | "sun" | "moon" }[] = [
  { mode: "system", icon: "monitor" },
  { mode: "light", icon: "sun" },
  { mode: "dark", icon: "moon" },
];

/** Sidebar theme switch. Applies and saves immediately; never touches services. */
export function ThemeSwitch() {
  const [theme, setTheme] = useState<ThemeMode>("system");

  useEffect(() => {
    invoke<{ theme?: ThemeMode }>("get_settings")
      .then((s) => setTheme(s.theme || "system"))
      .catch(() => {});
  }, []);

  const choose = (mode: ThemeMode) => {
    setTheme(mode);
    applyTheme(mode);
    invoke("set_theme", { theme: mode }).catch((e) => console.error("Failed to save theme:", e));
  };

  return (
    <div className="seg seg--block" role="group" aria-label="Theme">
      {MODES.map(({ mode, icon }) => (
        <button
          key={mode}
          type="button"
          aria-pressed={theme === mode}
          aria-label={themeLabel(mode)}
          title={themeLabel(mode)}
          onClick={() => choose(mode)}
        >
          <Icon name={icon} size={14} />
        </button>
      ))}
    </div>
  );
}
