import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { SiteInfo, SitesResponse } from "../types/services";

/** Why a folder has no address (contracts/list-sites.md). */
function reason(site: SiteInfo, port: number): string {
  switch (site.status) {
    case "invalid_name":
      return `Rename using letters, digits and hyphens to get an address. Still works at http://localhost:${port}/${site.name}/`;
    case "uppercase":
      return "Rename to lowercase (Linux folder names are case-sensitive).";
    case "conflict":
      return "Two folders differ only in capitals; rename one.";
    default:
      return "";
  }
}

/** Every project folder with its http://<name>.localhost:<port> address (spec 005 US2).
 *  Re-reads the folder when shown and whenever the window gets focus, so folders added
 *  in Explorer/Finder appear without a refresh button. */
export function SitesPage({ webRunning }: { webRunning: boolean }) {
  const [data, setData] = useState<SitesResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setData(await invoke<SitesResponse>("list_sites"));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    load();
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, [load]);

  const openFolder = () => {
    if (data) revealItemInDir(data.projects_dir).catch(console.error);
  };
  const port = data?.web_port ?? 8080;

  return (
    <div className="page">
      <header className="main__header">
        <div>
          <h2>Sites</h2>
          <p>Each project folder has its own address.</p>
        </div>
        <div className="main__actions">
          <button type="button" className="btn-secondary btn-sm" onClick={openFolder} disabled={!data}>
            Open projects folder
          </button>
        </div>
      </header>

      {!webRunning && (
        <div className="warn-box">
          <span>Start the web server to open sites.</span>
        </div>
      )}
      {error && (
        <div className="error-box">
          <p className="error-box-text">{error}</p>
        </div>
      )}

      <section className="card-section" aria-labelledby="s-sites">
        <div className="card-section__head">
          <h3 id="s-sites">Projects</h3>
          <p>
            Addresses work in Chrome, Edge and Firefox. If one doesn't open, use{" "}
            <span className="mono">http://localhost:{port}/&lt;folder&gt;/</span>
          </p>
        </div>
        {data && data.sites.length === 0 ? (
          <div className="list-box">
            <div className="list-row">
              <div className="list-row__main">
                <span>Put each project in its own folder in the projects folder.</span>
                <span className="mono">{data.projects_dir}</span>
              </div>
              <button type="button" className="btn-secondary btn-xs" onClick={openFolder}>
                Open projects folder
              </button>
            </div>
          </div>
        ) : (
          <div className="list-box">
            {data?.sites.map((site) => (
              <div key={site.name} className="list-row">
                <div className="list-row__main">
                  <span>
                    {site.name}
                    {site.kind === "framework" && <span className="lan-badge">Laravel/Symfony</span>}
                  </span>
                  {site.url ? (
                    <span className="mono">{site.url}</span>
                  ) : (
                    <span className="service-row__note">{reason(site, port)}</span>
                  )}
                </div>
                {site.url && (
                  <button
                    type="button"
                    className="btn-secondary btn-xs"
                    aria-label={`Open ${site.name}`}
                    onClick={() => openUrl(site.url!).catch(console.error)}
                    disabled={!webRunning}
                    title={webRunning ? site.url : "Start the web server first"}
                  >
                    Open
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
