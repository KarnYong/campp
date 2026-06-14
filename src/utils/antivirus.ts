import { invoke } from "@tauri-apps/api/core";

/**
 * Heads-up shown before installing ngrok on Windows. ngrok is a legitimate
 * tunneling tool that Defender frequently quarantines as "potentially unwanted
 * software" (a false positive, os error 225), which makes the install fail.
 * Telling the user up front avoids a confusing mid-install failure.
 */
export const NGROK_ANTIVIRUS_NOTICE =
  "Heads up: ngrok is sometimes blocked by Windows Defender (flagged as " +
  "'potentially unwanted software' — a false positive). If the install fails " +
  "with a virus or permission error, temporarily disable real-time protection " +
  "in Windows Security, or add a Defender exclusion for the CAMPP folders, " +
  "then retry.";

/**
 * Heuristic to detect errors caused by antivirus (almost always Windows
 * Defender) blocking a downloaded binary. ngrok in particular is flagged as
 * "potentially unwanted software" (PUA) — os error 225. This is almost always
 * a false positive.
 *
 * Matched loosely against the error string the backend reports (see
 * `maybe_av_hint` in src-tauri/src/runtime/downloader.rs), so the same check
 * works for both the first-run wizard and the Settings reinstall flow.
 */
export function isAntivirusError(message: string | null | undefined): boolean {
  if (!message) return false;
  const lower = message.toLowerCase();
  return (
    lower.includes("virus") ||
    lower.includes("unwanted software") ||
    lower.includes("os error 225") ||
    lower.includes("add-mppreference") ||
    lower.includes("antivirus")
  );
}

/**
 * Add Windows Defender real-time scanning exclusions for the CAMPP folders.
 * Triggers a UAC prompt on Windows; resolves once the exclusions are applied.
 * On non-Windows this rejects (the UI only offers the button on Windows).
 */
export async function addDefenderExclusion(): Promise<void> {
  await invoke("add_windows_defender_exclusion");
}
