//! "Open terminal": a terminal window in the projects folder with CAMPP's `php` and
//! `composer` first on PATH, for that session only (no system PATH change, no admin).
//!
//! Launchers live in `<base>/bin` and locate PHP relative to themselves, so install paths
//! with spaces or non-ASCII characters never pass through a code page.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::runtime::locator::{get_app_data_paths, locate_runtime_binaries, RuntimePaths};

/// Forward-slash path of `target` relative to `base`, if it is inside it.
fn rel(base: &Path, target: &Path) -> Option<String> {
    let r = target.strip_prefix(base).ok()?;
    Some(r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
}

/// Command-line PHP: `php.exe` next to `php-cgi.exe` on Windows, `runtime/php` elsewhere.
fn php_cli_path(paths: &RuntimePaths, runtime_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        paths.php_cgi.with_file_name("php.exe")
    } else {
        runtime_dir.join("php")
    }
}

/// Windows `.cmd` launcher body. Paths are relative to the launcher (`%~dp0`).
fn windows_launcher(php_rel: &str, ini_rel: &str, phar_rel: Option<&str>) -> String {
    let w = |p: &str| p.replace('/', "\\");
    let phar = phar_rel.map(|p| format!(" \"%~dp0..\\{}\"", w(p))).unwrap_or_default();
    format!("@\"%~dp0..\\{}\" -c \"%~dp0..\\{}\"{} %*\r\n", w(php_rel), w(ini_rel), phar)
}

/// Unix shell launcher body. Paths are relative to `<base>`.
fn unix_launcher(php_rel: &str, ini_rel: &str, phar_rel: Option<&str>) -> String {
    let phar = phar_rel.map(|p| format!(" \"$d/{p}\"")).unwrap_or_default();
    format!(
        "#!/bin/sh\nd=\"$(cd \"$(dirname \"$0\")/..\" && pwd)\"\nexec \"$d/{php_rel}\" -c \"$d/{ini_rel}\"{phar} \"$@\"\n"
    )
}

/// Single-quote a value for POSIX sh (`'` becomes `'\''`).
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Unix bootstrap every terminal runs: PATH first, cd to projects, then the user's shell.
fn shell_bootstrap(bin: &Path, projects: &Path) -> String {
    format!(
        "#!/bin/sh\nexport PATH={}:\"$PATH\"\ncd {} || exit 1\necho \"CAMPP: php and composer are on PATH in this window.\"\nexec \"${{SHELL:-/bin/sh}}\" -i\n",
        sh_quote(&bin.to_string_lossy()),
        sh_quote(&projects.to_string_lossy())
    )
}

fn write_file(path: &Path, body: &str, executable: bool) -> Result<(), String> {
    fs::write(path, body).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to make {} executable: {}", path.display(), e))?;
    }
    #[cfg(not(unix))]
    let _ = executable;
    Ok(())
}

/// (Re)write `php` and `composer` launchers in `<base>/bin`. The composer launcher is
/// removed when Composer isn't installed, so `composer` is "not found" in the terminal.
fn write_launchers(base: &Path, php: &Path, ini: &Path, phar: Option<&Path>) -> Result<PathBuf, String> {
    let bin = base.join("bin");
    fs::create_dir_all(&bin).map_err(|e| format!("Failed to create {}: {}", bin.display(), e))?;
    let outside = || format!("PHP is outside the CAMPP folder ({})", php.display());
    let php_rel = rel(base, php).ok_or_else(outside)?;
    let ini_rel = rel(base, ini).ok_or_else(outside)?;
    let phar_rel = phar.and_then(|p| rel(base, p));
    let (ext, make): (&str, fn(&str, &str, Option<&str>) -> String) = if cfg!(target_os = "windows") {
        (".cmd", windows_launcher)
    } else {
        ("", unix_launcher)
    };
    write_file(&bin.join(format!("php{ext}")), &make(&php_rel, &ini_rel, None), true)?;
    let composer = bin.join(format!("composer{ext}"));
    match phar_rel {
        Some(p) => write_file(&composer, &make(&php_rel, &ini_rel, Some(&p)), true)?,
        None => {
            let _ = fs::remove_file(&composer);
        }
    }
    Ok(bin)
}

fn no_terminal(bin: &Path) -> String {
    format!("No terminal app found. Add {} to PATH to use php and composer.", bin.display())
}

/// Open the platform terminal in the projects folder with `<base>/bin` first on PATH.
pub fn open_terminal() -> Result<String, String> {
    let app = get_app_data_paths()?;
    let paths = locate_runtime_binaries().map_err(|_| "Install PHP first to use the terminal.".to_string())?;
    if !paths.php_cgi.exists() {
        return Err("Install PHP first to use the terminal.".to_string());
    }
    let php = php_cli_path(&paths, &app.runtime_dir);
    if !php.exists() {
        return Err("Command-line PHP isn't available for this PHP version on this platform. \
                    Reinstall PHP, or choose PHP 8.2 or newer."
            .to_string());
    }
    if !paths.php_ini.exists() {
        crate::config::generator::generate_php_ini(&paths.php_ini, &paths)?;
    }
    crate::config::generator::ensure_php_ini_extensions(&paths.php_ini)?;
    let phar = app.runtime_dir.join("composer").join("composer.phar");
    let bin = write_launchers(&app.base_dir, &php, &paths.php_ini, phar.exists().then_some(phar.as_path()))?;
    let projects = &paths.projects_dir;
    let _ = fs::create_dir_all(projects);
    spawn(&bin, projects)?;
    Ok(bin.to_string_lossy().into_owned())
}

#[cfg(target_os = "windows")]
fn spawn(bin: &Path, projects: &Path) -> Result<(), String> {
    let path = format!("{};{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    use std::os::windows::process::CommandExt;
    // `start` opens a new console window that inherits PATH from this (hidden) cmd.
    // Its window title must be quoted, or `start` runs it as a program: raw_arg keeps the quotes.
    crate::process::manager::configure_no_window(Command::new("cmd"))
        .args(["/C", "start"])
        .raw_arg("\"CAMPP\"")
        .arg("/D")
        .arg(projects)
        .args(["cmd", "/K", "echo CAMPP: php and composer are on PATH in this window."])
        .env("PATH", path)
        .spawn()
        .map(|_| ())
        .map_err(|_| no_terminal(bin))
}

#[cfg(target_os = "macos")]
fn spawn(bin: &Path, projects: &Path) -> Result<(), String> {
    let script = bin.join("campp-terminal.command");
    write_file(&script, &shell_bootstrap(bin, projects), true)?;
    Command::new("open").args(["-a", "Terminal"]).arg(&script).spawn().map(|_| ()).map_err(|_| no_terminal(bin))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(bin: &Path, projects: &Path) -> Result<(), String> {
    let script = bin.join("campp-shell");
    write_file(&script, &shell_bootstrap(bin, projects), true)?;
    // First terminal that starts wins; each runs the same bootstrap script.
    let candidates: [(&str, &[&str]); 5] = [
        ("x-terminal-emulator", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("xterm", &["-e"]),
    ];
    for (term, flag) in candidates {
        if Command::new(term).args(flag).arg(&script).current_dir(projects).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(no_terminal(bin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_launcher_is_relative_to_itself() {
        let php = windows_launcher("runtime/php-8.5.12-Win32-vs17-x64/php.exe", "runtime/php-8.5.12-Win32-vs17-x64/php.ini", None);
        assert_eq!(php, "@\"%~dp0..\\runtime\\php-8.5.12-Win32-vs17-x64\\php.exe\" -c \"%~dp0..\\runtime\\php-8.5.12-Win32-vs17-x64\\php.ini\" %*\r\n");
        let composer = windows_launcher("runtime/p/php.exe", "runtime/p/php.ini", Some("runtime/composer/composer.phar"));
        assert!(composer.contains("\"%~dp0..\\runtime\\composer\\composer.phar\" %*"));
        assert!(!composer.contains(":\\"), "no absolute drive paths");
    }

    #[test]
    fn unix_launcher_runs_bundled_php_with_campp_ini() {
        let c = unix_launcher("runtime/php", "runtime/php.ini", Some("runtime/composer/composer.phar"));
        assert!(c.starts_with("#!/bin/sh\n"));
        assert!(c.contains("exec \"$d/runtime/php\" -c \"$d/runtime/php.ini\" \"$d/runtime/composer/composer.phar\" \"$@\""));
    }

    #[test]
    fn bootstrap_quotes_spaces_and_apostrophes() {
        let s = shell_bootstrap(Path::new("/home/a b/.local/share/campp/bin"), Path::new("/home/o'neil/projects"));
        assert!(s.contains("export PATH='/home/a b/.local/share/campp/bin':\"$PATH\""));
        assert!(s.contains("cd '/home/o'\\''neil/projects'"));
    }

    #[test]
    fn composer_launcher_follows_installation() {
        let base = tempfile::TempDir::new().unwrap();
        let rt = base.path().join("runtime");
        fs::create_dir_all(rt.join("composer")).unwrap();
        let (php, ini, phar) = (rt.join("php"), rt.join("php.ini"), rt.join("composer/composer.phar"));
        let ext = if cfg!(target_os = "windows") { ".cmd" } else { "" };
        let bin = write_launchers(base.path(), &php, &ini, Some(&phar)).unwrap();
        assert!(bin.join(format!("php{ext}")).exists());
        assert!(bin.join(format!("composer{ext}")).exists());
        write_launchers(base.path(), &php, &ini, None).unwrap();
        assert!(!bin.join(format!("composer{ext}")).exists(), "removed when Composer isn't installed");
        assert!(write_launchers(Path::new("/elsewhere"), &php, &ini, None).is_err());
    }
}
