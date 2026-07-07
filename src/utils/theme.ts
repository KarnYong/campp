export type ThemeMode = "system" | "light" | "dark";

/**
 * Apply a saved theme preference to the document root.
 * "system" clears the override so the OS preference (media query) governs;
 * "light"/"dark" force it via the [data-theme] attribute.
 */
export function applyTheme(mode?: string) {
  const root = document.documentElement;
  const m = (mode || "system") as ThemeMode;
  if (m === "light" || m === "dark") {
    root.dataset.theme = m;
  } else {
    delete root.dataset.theme;
  }
}

export function themeLabel(mode?: string): string {
  switch (mode) {
    case "light": return "Always light";
    case "dark": return "Always dark";
    default: return "Match your operating system";
  }
}
