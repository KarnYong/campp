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
    <div
      style={{
        marginTop: "0.75rem",
        padding: "0.75rem",
        borderRadius: "0.5rem",
        border: "1px solid #d97706",
        backgroundColor: "rgba(217, 119, 6, 0.12)",
        color: "var(--text-primary)",
        fontSize: "0.8125rem",
        lineHeight: 1.5,
        textAlign: "left",
      }}
    >
      <div style={{ fontWeight: 600, color: "#d97706", marginBottom: "0.375rem" }}>
        ⚠ ngrok may be blocked by Windows Defender
      </div>
      <p style={{ margin: "0 0 0.5rem 0" }}>
        ngrok is sometimes flagged as <em>"potentially unwanted software"</em> (a
        false positive). If the install fails with a virus or permission error,
        add the CAMPP folders to Defender's exclusions first, then retry.
      </p>

      {/* One-click automatic exclusion */}
      <button
        onClick={handleAddExclusion}
        disabled={status === "adding"}
        className="btn-primary"
        style={{
          width: "100%",
          marginBottom: "0.5rem",
          fontSize: "0.8125rem",
          padding: "0.5rem",
        }}
      >
        {status === "adding"
          ? "Adding exclusion… (accept the UAC prompt)"
          : status === "added"
          ? "✓ Exclusion added"
          : "Add Defender exclusion automatically"}
      </button>
      {message && (
        <p
          style={{
            margin: "0 0 0.5rem 0",
            fontSize: "0.75rem",
            color:
              status === "error" ? "var(--color-error, #e06c75)" : "#fbbf24",
            whiteSpace: "pre-wrap",
          }}
        >
          {message}
        </p>
      )}

      {/* Manual fallback steps */}
      <div style={{ borderTop: "1px solid rgba(217, 119, 6, 0.3)", paddingTop: "0.5rem", marginTop: "0.25rem" }}>
        <button
          type="button"
          onClick={() => setShowManual((v) => !v)}
          aria-expanded={showManual}
          style={{
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
            fontSize: "0.8125rem",
            fontWeight: 500,
            color: "#d97706",
            display: "inline-flex",
            alignItems: "center",
            gap: "0.25rem",
          }}
        >
          {showManual ? "▾" : "▸"} Add the exclusion manually
        </button>
        {showManual && (
          <>
            <ol style={{ margin: "0.5rem 0 0.5rem 1.25rem", padding: 0 }}>
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
              <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", marginTop: "0.25rem" }}>
                {folders.map((f) => (
                  <code
                    key={f}
                    style={{
                      display: "block",
                      padding: "0.25rem 0.375rem",
                      borderRadius: "0.25rem",
                      backgroundColor: "rgba(0, 0, 0, 0.25)",
                      fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                      fontSize: "0.75rem",
                      wordBreak: "break-all",
                    }}
                  >
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
