import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { addDefenderExclusion } from "../utils/antivirus";

/**
 * Proactive heads-up shown before installing ngrok on Windows. ngrok is a
 * legitimate tunneling tool that Windows Defender frequently quarantines as
 * "potentially unwanted software" (a false positive, os error 225), which
 * makes the install fail mid-way.
 *
 * This gives the user two ways to handle it, up front (rather than only after
 * a failure):
 *   1. A one-click "Add exclusion automatically" button — elevates via UAC and
 *      adds the CAMPP folders to Defender's exclusion list (no manual steps).
 *   2. The manual Windows Security steps + the exact folders to exclude, for
 *      users who prefer to do it themselves or whose UAC was declined.
 *
 * Showing it proactively means it works regardless of how (or whether) an
 * antivirus error later surfaces, so it doesn't depend on fragile error
 * detection in the install/reinstall flows.
 */
export function NgrokAntivirusNotice() {
  const [folders, setFolders] = useState<string[]>([]);
  const [status, setStatus] = useState<"idle" | "adding" | "added" | "error">(
    "idle"
  );
  const [message, setMessage] = useState<string>("");
  const [showManual, setShowManual] = useState(false);

  useEffect(() => {
    // The two folders Defender scans during a CAMPP install: the download
    // temp dir (where the zip is quarantined) and the runtime dir.
    Promise.allSettled([
      invoke<string>("get_download_dir"),
      invoke<string>("get_runtime_dir"),
    ]).then((results) => {
      const paths = results
        .map((r) => (r.status === "fulfilled" ? r.value : null))
        .filter((p): p is string => !!p);
      const seen = new Set<string>();
      setFolders(paths.filter((p) => (seen.has(p) ? false : (seen.add(p), true))));
    });
  }, []);

  const handleAddExclusion = async () => {
    setStatus("adding");
    setMessage("");
    try {
      await addDefenderExclusion();
      setStatus("added");
      setMessage("Exclusion added. You can now install ngrok.");
    } catch (e) {
      setStatus("error");
      setMessage(
        `Could not add the exclusion automatically (did you decline the UAC prompt?). ${e}`
      );
    }
  };

  return (
    <div className="av-notice">
      <div className="av-notice__title">⚠ ngrok may be blocked by Windows Defender</div>
      <p>
        ngrok is sometimes flagged as <em>"potentially unwanted software"</em> (a
        false positive). If the install fails with a virus or permission error,
        add the CAMPP folders to Defender's exclusions first, then retry.
      </p>

      {/* One-click automatic exclusion */}
      <button
        type="button"
        onClick={handleAddExclusion}
        disabled={status === "adding"}
        className="btn-primary btn-xs av-notice__btn"
      >
        {status === "adding"
          ? "Adding exclusion… (accept the UAC prompt)"
          : status === "added"
          ? "✓ Exclusion added"
          : "Add Defender exclusion automatically"}
      </button>
      {message && (
        <p className={status === "error" ? "text-bad wiz-small" : "text-ok wiz-small"} style={{ whiteSpace: "pre-wrap" }}>
          {message}
        </p>
      )}

      {/* Manual fallback steps */}
      <div className="av-notice__manual">
        <button
          type="button"
          className="av-notice__toggle"
          onClick={() => setShowManual((v) => !v)}
          aria-expanded={showManual}
        >
          {showManual ? "▾" : "▸"} Add the exclusion manually
        </button>
        {showManual && (
          <>
            <ol>
              <li>Open <strong>Windows Security</strong>.</li>
              <li>Go to <strong>Virus &amp; threat protection</strong>.</li>
              <li>
                Click <strong>Manage settings</strong> under "Virus &amp; threat
                protection settings".
              </li>
              <li>
                Scroll down to <strong>Exclusions</strong> and click{" "}
                <strong>Add or remove exclusions</strong>.
              </li>
              <li>
                Click <strong>Add an exclusion</strong> → <strong>Folder</strong>.
              </li>
              <li>
                Select the folder, then repeat for each of these:
              </li>
            </ol>
            {folders.length > 0 && (
              <div className="wiz-stack">
                {folders.map((f) => (
                  <code key={f} className="av-notice__path mono">
                    {f}
                  </code>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
