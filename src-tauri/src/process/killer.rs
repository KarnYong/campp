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
        let pkill = |signal: &str| {
            Command::new("pkill")
                .args([signal, "-f", marker.as_ref()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success()) // true = something matched
                .unwrap_or(false)
        };
        // TERM first so php-fpm/postgres masters stop their own workers. A SIGKILLed
        // php-fpm master leaves its workers ("php-fpm: pool www", no path in the
        // title) holding the port, and nothing path-scoped could find them after.
        if pkill("-TERM") {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            pkill("-KILL");
        }

        #[cfg(target_os = "linux")]
        kill_by_executable(path_marker);

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::kill_existing_processes;
    use std::process::Command;

    #[test]
    fn kills_processes_by_executable_even_with_a_rewritten_title() {
        let dir = tempfile::TempDir::new().unwrap();
        let bin = dir.path().join("fake-php-fpm");
        std::fs::copy("/bin/sleep", &bin).unwrap();

        // Launched by path: found by the command line.
        let mut direct = Command::new(&bin).arg("300").spawn().unwrap();
        // exec -a renames argv[0] like php-fpm's "php-fpm: pool www": only /proc/<pid>/exe still points at our binary.
        let mut renamed = Command::new("bash")
            .args(["-c", &format!("exec -a 'php-fpm: pool www' '{}' 300", bin.display())])
            .spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        kill_existing_processes(dir.path());

        for (name, child) in [("direct", &mut direct), ("renamed", &mut renamed)] {
            let mut gone = false;
            for _ in 0..30 {
                if child.try_wait().unwrap().is_some() { gone = true; break; }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            if !gone { let _ = child.kill(); }
            assert!(gone, "{name} process survived");
        }
    }
}

/// Linux: SIGKILL processes whose real executable (/proc/<pid>/exe) is under
/// `path_marker`. Process titles can be rewritten (php-fpm does); the exe link can't,
/// so this also catches orphaned workers, and still only CAMPP's own binaries.
#[cfg(target_os = "linux")]
fn kill_by_executable(path_marker: &Path) {
    let Ok(entries) = std::fs::read_dir("/proc") else { return };
    let me = std::process::id().to_string();
    for entry in entries.flatten() {
        let pid = entry.file_name().to_string_lossy().to_string();
        if pid == me || !pid.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(exe) = std::fs::read_link(entry.path().join("exe")) {
            if exe.starts_with(path_marker) {
                let _ = Command::new("kill").args(["-9", &pid]).stdout(Stdio::null()).stderr(Stdio::null()).status();
            }
        }
    }
}
