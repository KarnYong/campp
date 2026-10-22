import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

/** Caddy version CAMPP installs (from runtime-config.json). Undefined until loaded. */
export function useCaddyVersion(): string | undefined {
  const [version, setVersion] = useState<string>();
  useEffect(() => {
    invoke<string>("get_caddy_version")
      .then((v) => setVersion(v || undefined))
      .catch(() => {});
  }, []);
  return version;
}
