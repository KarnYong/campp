import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";
import { showConfirm } from "./Dialog";

interface BackupInfo {
  name: string;
  size: number;
  created_unix: number;
}

const formatDate = (unix: number) => new Date(unix * 1000).toLocaleString();
const formatSize = (bytes: number) =>
  bytes >= 1 << 30 ? `${(bytes / (1 << 30)).toFixed(1)} GB` : `${Math.max(1, Math.round(bytes / (1 << 20)))} MB`;

/** Back up / restore the projects folder and all database data. Services must be stopped. */
export function BackupPage({ servicesActive }: { servicesActive: boolean }) {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  // "backup" | name of the backup being restored | null
  const [action, setAction] = useState<string | null>(null);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  const loadBackups = useCallback(async () => {
    try {
      setBackups(await invoke<BackupInfo[]>("list_backups"));
    } catch (e) {
      console.error("Failed to list backups:", e);
    }
  }, []);

  useEffect(() => {
    loadBackups();
  }, [loadBackups]);

  const handleBackup = async () => {
    setAction("backup");
    setMsg(null);
    try {
      const name = await invoke<string>("create_backup");
      setMsg({ ok: true, text: `Backup saved: ${name}` });
      await loadBackups();
    } catch (e) {
      setMsg({ ok: false, text: `Backup failed: ${e}` });
    } finally {
      setAction(null);
    }
  };

  const handleRestore = async (b: BackupInfo) => {
    const ok = await showConfirm(
      "Restore this backup?",
      `Your projects folder and all databases will be replaced with the backup from ${formatDate(b.created_unix)}. ` +
        "CAMPP saves a backup of your current state first, so you can undo this.",
      "Restore",
      true
    );
    if (!ok) return;
    setAction(b.name);
    setMsg(null);
    try {
      const safety = await invoke<string>("restore_backup", { name: b.name });
      setMsg({ ok: true, text: `Restored. Your previous state was saved as ${safety}.` });
      await loadBackups();
    } catch (e) {
      setMsg({ ok: false, text: `Restore failed: ${e}` });
    } finally {
      setAction(null);
    }
  };

  const handleDelete = async (b: BackupInfo) => {
    const ok = await showConfirm(
      "Delete this backup?",
      `The backup from ${formatDate(b.created_unix)} (${formatSize(b.size)}) will be permanently deleted.`,
      "Delete",
      true
    );
    if (!ok) return;
    setMsg(null);
    try {
      await invoke("delete_backup", { name: b.name });
      await loadBackups();
    } catch (e) {
      setMsg({ ok: false, text: `Delete failed: ${e}` });
    }
  };

  const blocked = servicesActive || !!action;

  return (
    <div className="page">
      <header className="main__header">
        <div>
          <h2>Backup</h2>
          <p>Projects folder and all database data in one zip.</p>
        </div>
        <div className="main__actions">
          <button type="button" className="btn-secondary btn-sm" onClick={() => invoke("open_backups_folder").catch(console.error)}>
            Open folder
          </button>
          <button
            type="button"
            className="btn-primary btn-sm"
            onClick={handleBackup}
            disabled={blocked}
            title={servicesActive ? "Stop all services first" : undefined}
          >
            {action === "backup" ? "Backing up…" : "Back up now"}
          </button>
        </div>
      </header>

      {servicesActive && (
        <div className="warn-box">
          <span>Stop all services to back up or restore. Database files can't be copied safely while they're running.</span>
        </div>
      )}
      {msg && (
        <div className={msg.ok ? "success-box" : "error-box"}>
          <p className={msg.ok ? "success-box-text" : "error-box-text"} style={{ wordBreak: "break-word" }}>{msg.text}</p>
        </div>
      )}

      <section className="card-section" aria-labelledby="s-backups">
        <div className="card-section__head">
          <h3 id="s-backups">Backups</h3>
          <p>Restoring takes a backup of your current state first. Database passwords stay as set in Settings.</p>
        </div>
        {backups.length > 0 ? (
          <div className="list-box">
            {backups.map((b) => (
              <div key={b.name} className="list-row">
                <div className="list-row__main">
                  <span>{formatDate(b.created_unix)}{b.name.includes("pre-restore") ? " · before restore" : b.name.includes("pre-delete") ? " · before delete" : ""}</span>
                  <span className="mono">{b.name} · {formatSize(b.size)}</span>
                </div>
                {action === b.name && (
                  <span className="list-row__busy">
                    <span className="btn-spinner" aria-hidden="true" />
                    Restoring…
                  </span>
                )}
                <button
                  type="button"
                  onClick={() => handleRestore(b)}
                  disabled={blocked}
                  title={servicesActive ? "Stop all services first" : undefined}
                  className="btn-secondary btn-xs btn-danger"
                >
                  Restore
                </button>
                <button
                  type="button"
                  onClick={() => handleDelete(b)}
                  disabled={!!action}
                  className="btn-secondary btn-xs btn-danger"
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        ) : (
          <p className="wiz-muted wiz-small">No backups yet. To restore a backup from another computer, put its zip in the backups folder.</p>
        )}
      </section>
    </div>
  );
}
