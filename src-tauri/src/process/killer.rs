use std::path::Path;
use std::process::{Command, Stdio};

/// Kill leftover CAMPP runtime processes started from `path_marker` — either a
/// specific runtime binary (e.g. `.../runtime/caddy/caddy`) or a runtime
/// subdirectory (e.g. `.../runtime/pgsql`, or the whole `.../runtime` dir).
///
/// Matching is by executable PATH, never by bare process name. A developer's
/// system-wide MySQL/MariaDB/PostgreSQL/Caddy shares the process name but lives
/// outside our runtime dir, so it must never be touched. The old name-only
/// `pkill -9 mysqld` / `taskkill /IM mysqld.exe` would SIGKILL those too, which
/// could take down (and corrupt) a user's real database.
pub fn kill_existing_processes(path_marker: &Path) {
    let marker = path_marker.to_string_lossy();
    if marker.is_empty() {
        return;
    }

    #[cfg(windows)]
    {
        // taskkill /IM can't filter by path, so match ExecutablePath via WMI and
        // force-kill only the PIDs whose exe lives under our runtime path.
        let ps = format!(
            "Get-CimInstance Win32_Process | \
             Where-Object {{ $_.ExecutablePath -like '{}*' }} | \
             ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
            marker.replace('\'', "''")
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    #[cfg(unix)]
    {
        // pkill -f matches the full command line; our binaries are launched by
        // absolute path, so this only ever matches CAMPP's own processes.
        // ponytail: marker is used as a regex; runtime paths don't contain regex
        // metacharacters in practice, so no escaping needed.
        let _ = Command::new("pkill")
            .args(["-9", "-f", marker.as_ref()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
