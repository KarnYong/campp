//! Backup / restore of the projects folder and the MySQL/MariaDB + PostgreSQL data
//! folders as one zip in `<base>/backups/`.
//!
//! ponytail: file-level snapshot, not SQL dumps. Fine because it only runs with every
//! service stopped, but a backup only restores into a compatible database version
//! (checked below). Add mysqldump/pg_dumpall export if cross-version moves are needed.

use crate::runtime::downloader::RuntimeDownloader;
use crate::runtime::locator::get_app_data_paths;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Folders captured, relative to the app base dir (same layout on every OS).
const DIRS: [&str; 3] = ["projects", "mysql/data", "pgsql/data"];
const MANIFEST: &str = "campp-backup.json";

#[derive(Serialize, serde::Deserialize)]
struct Manifest {
    campp_version: String,
    created_unix: u64,
    /// Installed component versions at backup time (component -> version).
    versions: HashMap<String, String>,
    /// std::env::consts::OS at backup time. Empty for backups made before it was recorded.
    #[serde(default)]
    os: String,
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    pub created_unix: u64,
}

// Public API works on the app's data folder; the *_in functions take the base folder
// explicitly so tests can run them against a temp dir.

fn app_base() -> Result<PathBuf, String> {
    Ok(get_app_data_paths()?.base_dir)
}

fn backups_dir_in(base: &Path) -> PathBuf {
    base.join("backups")
}

pub fn backups_dir() -> Result<PathBuf, String> {
    Ok(backups_dir_in(&app_base()?))
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn installed_versions() -> HashMap<String, String> {
    RuntimeDownloader::new().map(|d| d.get_installed_components()).unwrap_or_default()
}

pub fn list_backups() -> Result<Vec<BackupInfo>, String> {
    Ok(list_in(&backups_dir()?))
}

fn list_in(dir: &Path) -> Vec<BackupInfo> {
    let mut list = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return list };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".zip") {
            continue;
        }
        let meta = match e.metadata() { Ok(m) => m, Err(_) => continue };
        let created_unix = meta.modified().ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        list.push(BackupInfo { name, size: meta.len(), created_unix });
    }
    list.sort_by(|a, b| b.created_unix.cmp(&a.created_unix).then(b.name.cmp(&a.name)));
    list
}

/// Write a new backup zip. `suffix` tags automatic ones (e.g. "pre-restore").
pub fn create_backup(suffix: Option<&str>) -> Result<String, String> {
    create_in(&app_base()?, suffix, &installed_versions())
}

fn create_in(base: &Path, suffix: Option<&str>, versions: &HashMap<String, String>) -> Result<String, String> {
    let dir = backups_dir_in(base);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create backups folder: {}", e))?;

    let created_unix = now_unix();
    let stem = match suffix {
        Some(s) => format!("campp-backup-{}-{}", created_unix, s),
        None => format!("campp-backup-{}", created_unix),
    };
    // Two backups in the same second must not overwrite each other.
    let mut name = format!("{}.zip", stem);
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{}-{}.zip", stem, n);
        n += 1;
    }
    // Write to .part and rename, so a failed backup never shows up in the list.
    let part = dir.join(format!("{}.part", name));
    let result = write_zip(base, &part, created_unix, versions);
    if let Err(e) = result {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    fs::rename(&part, dir.join(&name)).map_err(|e| format!("Failed to finish backup: {}", e))?;
    Ok(name)
}

fn write_zip(base: &Path, out: &Path, created_unix: u64, versions: &HashMap<String, String>) -> Result<(), String> {
    let file = File::create(out).map_err(|e| format!("Failed to create backup file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);

    let manifest = Manifest {
        campp_version: env!("CARGO_PKG_VERSION").to_string(),
        created_unix,
        versions: versions.clone(),
        os: std::env::consts::OS.to_string(),
    };
    zip.start_file(MANIFEST, opts).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut zip, &manifest).map_err(|e| e.to_string())?;

    for rel in DIRS {
        let src = base.join(rel);
        if src.is_dir() {
            add_dir(&mut zip, &src, rel, opts)?;
        }
    }
    zip.finish().map_err(|e| format!("Failed to finish backup: {}", e))?;
    Ok(())
}

fn add_dir(
    zip: &mut zip::ZipWriter<File>,
    dir: &Path,
    name: &str,
    opts: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    // Directory entries too: PostgreSQL won't start without its empty subdirs (pg_notify, pg_tblspc, ...).
    zip.add_directory(format!("{}/", name), opts).map_err(|e| e.to_string())?;
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        // A leftover lock file from an unclean shutdown can stop PostgreSQL starting elsewhere.
        if file_name == "postmaster.pid" {
            continue;
        }
        let child = format!("{}/{}", name, file_name);
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            add_dir(zip, &path, &child, opts)?;
        } else if ft.is_file() {
            zip.start_file(child.as_str(), file_opts(opts, &entry)).map_err(|e| e.to_string())?;
            let mut f = File::open(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            io::copy(&mut f, zip).map_err(|e| format!("Failed to add {}: {}", path.display(), e))?;
        } else if ft.is_symlink() {
            // Projects use symlinks on macOS/Linux (e.g. node_modules/.bin). Stored as links;
            // Windows restores skip them (creating links there needs admin rights).
            let target = fs::read_link(&path).map_err(|e| format!("Failed to read link {}: {}", path.display(), e))?;
            zip.add_symlink(child.as_str(), target.to_string_lossy(), opts).map_err(|e| e.to_string())?;
        }
        // Sockets/devices are skipped: none belong in these folders once services are stopped.
    }
    Ok(())
}

/// Keep Unix permissions (executable scripts in projects), so a restore on macOS/Linux
/// gives back the same modes. No-op on Windows.
#[cfg(unix)]
fn file_opts(opts: zip::write::SimpleFileOptions, entry: &fs::DirEntry) -> zip::write::SimpleFileOptions {
    use std::os::unix::fs::PermissionsExt;
    match entry.metadata() {
        Ok(m) => opts.unix_permissions(m.permissions().mode()),
        Err(_) => opts,
    }
}

#[cfg(not(unix))]
fn file_opts(opts: zip::write::SimpleFileOptions, _entry: &fs::DirEntry) -> zip::write::SimpleFileOptions {
    opts
}

fn parse_ver(v: &str) -> Vec<u32> {
    v.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
}

fn os_name(os: &str) -> &str {
    match os {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    }
}

/// Reject backups the installed databases can't open. A file-level copy only works
/// on the same OS (MySQL's lower_case_table_names and PostgreSQL's locales differ per
/// OS), the same engine, the same PostgreSQL major, and a MySQL/MariaDB that is not older.
fn check_compatible(
    backup: &HashMap<String, String>,
    current: &HashMap<String, String>,
    has_mysql_data: bool,
    has_pg_data: bool,
    backup_os: &str,
    current_os: &str,
) -> Result<(), String> {
    if (has_mysql_data || has_pg_data) && !backup_os.is_empty() && backup_os != current_os {
        return Err(format!(
            "This backup was made on {}. Database files can't be moved to {}. \
             To move databases between computers with different systems, export them with \
             phpMyAdmin or Adminer and import them here.",
            os_name(backup_os), os_name(current_os)
        ));
    }
    if has_mysql_data {
        if let Some((engine, bver)) = ["mysql", "mariadb"].iter().find_map(|k| backup.get(*k).map(|v| (*k, v))) {
            let name = if engine == "mysql" { "MySQL" } else { "MariaDB" };
            let cur = current.get(engine).ok_or_else(|| {
                format!("This backup holds {} {} data, but {} is not installed here.", name, bver, name)
            })?;
            let (b, c) = (parse_ver(bver), parse_ver(cur));
            if b.iter().take(2).cmp(c.iter().take(2)) == std::cmp::Ordering::Greater {
                return Err(format!(
                    "This backup is from {} {}, which is newer than the installed {}. Install {} {} or later first.",
                    name, bver, cur, name, bver
                ));
            }
        }
    }
    if has_pg_data {
        if let Some(bver) = backup.get("postgresql") {
            let cur = current.get("postgresql").ok_or_else(|| {
                format!("This backup holds PostgreSQL {} data, but PostgreSQL is not installed here.", bver)
            })?;
            if parse_ver(bver).first() != parse_ver(cur).first() {
                return Err(format!(
                    "This backup is from PostgreSQL {}, but PostgreSQL {} is installed. The major version must match.",
                    bver, cur
                ));
            }
        }
    }
    Ok(())
}

/// Resolve a backup file name to its path, refusing anything outside the backups folder.
fn backup_path_in(base: &Path, name: &str) -> Result<PathBuf, String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || !name.ends_with(".zip") {
        return Err("Invalid backup name".to_string());
    }
    Ok(backups_dir_in(base).join(name))
}

pub fn delete_backup(name: &str) -> Result<(), String> {
    fs::remove_file(backup_path_in(&app_base()?, name)?).map_err(|e| format!("Failed to delete backup: {}", e))
}

/// Delete every project and all database data. A "pre-delete" backup is taken
/// first so it can be undone from the Backup page. Databases re-initialize empty
/// on their next start; the projects folder itself is kept (emptied).
pub fn delete_all_data() -> Result<String, String> {
    delete_all_in(&app_base()?, &installed_versions())
}

fn delete_all_in(base: &Path, versions: &HashMap<String, String>) -> Result<String, String> {
    let safety = create_in(base, Some("pre-delete"), versions)?;

    for rel in ["mysql/data", "pgsql/data"] {
        let dir = base.join(rel);
        if dir.exists() {
            crate::commands::remove_dir_all_with_retry(&dir)?;
        }
    }
    let projects = base.join("projects");
    if let Ok(entries) = fs::read_dir(&projects) {
        for entry in entries.flatten() {
            let path = entry.path();
            let res = if path.is_dir() { crate::commands::remove_dir_all_with_retry(&path) } else {
                fs::remove_file(&path).map_err(|e| format!("Failed to delete {}: {}", path.display(), e))
            };
            res.map_err(|e| format!("{} (everything is in backup {})", e, safety))?;
        }
    }
    Ok(safety)
}

/// Replace projects + database data with the backup's. Takes a "pre-restore" backup
/// of the current state first, so a restore can always be undone.
pub fn restore_backup(name: &str) -> Result<String, String> {
    restore_in(&app_base()?, name, &installed_versions())
}

/// `current`: versions installed now, checked against the backup's.
fn restore_in(base: &Path, name: &str, current: &HashMap<String, String>) -> Result<String, String> {
    let zip_path = backup_path_in(base, name)?;
    let dir = backups_dir_in(base);

    // Read manifest + which folders the backup holds, before touching anything.
    let (manifest, present) = {
        let file = File::open(&zip_path).map_err(|e| format!("Failed to open backup: {}", e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Not a valid backup: {}", e))?;
        let manifest: Manifest = {
            let entry = archive.by_name(MANIFEST).map_err(|_| "Not a CAMPP backup (manifest missing)".to_string())?;
            serde_json::from_reader(entry).map_err(|e| format!("Invalid backup manifest: {}", e))?
        };
        let present: Vec<&str> = DIRS.iter().copied()
            .filter(|d| archive.file_names().any(|n| n.starts_with(&format!("{}/", d))))
            .collect();
        (manifest, present)
    };
    check_compatible(
        &manifest.versions,
        current,
        present.contains(&"mysql/data"),
        present.contains(&"pgsql/data"),
        &manifest.os,
        std::env::consts::OS,
    )?;

    let safety = create_in(base, Some("pre-restore"), current)?;

    let tmp = dir.join(".restore-tmp");
    if tmp.exists() {
        fs::remove_dir_all(&tmp).map_err(|e| format!("Failed to clear temp folder: {}", e))?;
    }
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    RuntimeDownloader::new()?.extract_zip(&zip_path, &tmp)?;

    for rel in &present {
        let target = base.join(rel);
        if target.exists() {
            crate::commands::remove_dir_all_with_retry(&target)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::rename(tmp.join(rel), &target)
            .map_err(|e| format!("Failed to restore {} (your previous data is in {}): {}", rel, safety, e))?;
    }
    let _ = fs::remove_dir_all(&tmp);

    let pg_data = base.join("pgsql/data");
    if present.contains(&"pgsql/data") {
        // Re-apply the current PostgreSQL password on next start (MySQL does this every start).
        let _ = fs::remove_file(pg_data.join(".password_set"));
        // PostgreSQL refuses to start unless its data dir is private (0700).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&pg_data, fs::Permissions::from_mode(0o700));
        }
    }

    Ok(safety)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn zip_round_trip_keeps_files_and_empty_dirs() {
        let src = tempfile::TempDir::new().unwrap();
        let base = src.path();
        fs::create_dir_all(base.join("projects/site")).unwrap();
        fs::write(base.join("projects/site/index.php"), "<?php echo 1;").unwrap();
        fs::create_dir_all(base.join("pgsql/data/pg_notify")).unwrap(); // empty, must survive
        fs::write(base.join("pgsql/data/PG_VERSION"), "18").unwrap();

        let out = base.join("b.zip");
        write_zip(base, &out, 1, &HashMap::new()).unwrap();

        let dst = tempfile::TempDir::new().unwrap();
        RuntimeDownloader::new().unwrap().extract_zip(&out, dst.path()).unwrap();
        assert_eq!(fs::read_to_string(dst.path().join("projects/site/index.php")).unwrap(), "<?php echo 1;");
        assert!(dst.path().join("pgsql/data/pg_notify").is_dir());
        assert!(dst.path().join(MANIFEST).is_file());
        assert!(!dst.path().join("mysql").exists(), "absent folders are not invented");
    }

    #[cfg(unix)]
    #[test]
    fn zip_round_trip_keeps_unix_modes_and_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let src = tempfile::TempDir::new().unwrap();
        let base = src.path();
        fs::create_dir_all(base.join("projects/app/node_modules/.bin")).unwrap();
        fs::write(base.join("projects/app/run.sh"), "#!/bin/sh\necho hi\n").unwrap();
        fs::set_permissions(base.join("projects/app/run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        symlink("../tool/cli.js", base.join("projects/app/node_modules/.bin/tool")).unwrap();
        fs::create_dir_all(base.join("pgsql/data")).unwrap();
        fs::write(base.join("pgsql/data/postmaster.pid"), "123").unwrap();

        let out = base.join("b.zip");
        write_zip(base, &out, 1, &HashMap::new()).unwrap();
        let dst = tempfile::TempDir::new().unwrap();
        RuntimeDownloader::new().unwrap().extract_zip(&out, dst.path()).unwrap();

        let mode = fs::metadata(dst.path().join("projects/app/run.sh")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "executable bit kept");
        let link = dst.path().join("projects/app/node_modules/.bin/tool");
        assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from("../tool/cli.js"), "symlink kept");
        assert!(!dst.path().join("pgsql/data/postmaster.pid").exists(), "stale lock file skipped");
    }

    /// A fake CAMPP data folder with a project, MySQL data and PostgreSQL data.
    fn fake_install() -> tempfile::TempDir {
        let t = tempfile::TempDir::new().unwrap();
        let b = t.path();
        fs::create_dir_all(b.join("projects/site")).unwrap();
        fs::write(b.join("projects/site/index.php"), "v1").unwrap();
        fs::write(b.join("projects/index.php"), "welcome").unwrap();
        fs::create_dir_all(b.join("mysql/data/shop")).unwrap();
        fs::write(b.join("mysql/data/shop/orders.ibd"), "orders-v1").unwrap();
        fs::create_dir_all(b.join("pgsql/data/pg_notify")).unwrap();
        fs::write(b.join("pgsql/data/PG_VERSION"), "18").unwrap();
        fs::write(b.join("pgsql/data/.password_set"), "done").unwrap();
        t
    }

    fn read(b: &Path, rel: &str) -> String {
        fs::read_to_string(b.join(rel)).unwrap()
    }

    #[test]
    fn restore_brings_everything_back_and_saves_current_state_first() {
        let t = fake_install();
        let b = t.path();
        let v = m(&[("mysql", "8.4.11"), ("postgresql", "18.6.1")]);
        let name = create_in(b, None, &v).unwrap();

        // Change everything after the backup.
        fs::remove_dir_all(b.join("projects/site")).unwrap();
        fs::write(b.join("projects/new.php"), "new").unwrap();
        fs::write(b.join("mysql/data/shop/orders.ibd"), "orders-v2").unwrap();
        fs::write(b.join("pgsql/data/PG_VERSION"), "18-changed").unwrap();

        let safety = restore_in(b, &name, &v).unwrap();

        assert_eq!(read(b, "projects/site/index.php"), "v1");
        assert!(!b.join("projects/new.php").exists(), "files added after the backup are gone");
        assert_eq!(read(b, "mysql/data/shop/orders.ibd"), "orders-v1");
        assert_eq!(read(b, "pgsql/data/PG_VERSION"), "18");
        assert!(b.join("pgsql/data/pg_notify").is_dir());
        assert!(!b.join("pgsql/data/.password_set").exists(), "PG password re-applied on next start");
        assert!(!b.join("backups/.restore-tmp").exists(), "temp folder cleaned up");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(b.join("pgsql/data")).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "PostgreSQL needs a private data dir");
        }

        // The automatic backup holds the state from just before the restore, so it undoes it.
        assert!(safety.contains("pre-restore"));
        restore_in(b, &safety, &v).unwrap();
        assert_eq!(read(b, "projects/new.php"), "new");
        assert_eq!(read(b, "mysql/data/shop/orders.ibd"), "orders-v2");
    }

    #[test]
    fn incompatible_restore_changes_nothing() {
        let t = fake_install();
        let b = t.path();
        let name = create_in(b, None, &m(&[("mysql", "9.1.0")])).unwrap();
        fs::write(b.join("mysql/data/shop/orders.ibd"), "current").unwrap();

        assert!(restore_in(b, &name, &m(&[("mysql", "8.4.11")])).is_err());
        assert_eq!(read(b, "mysql/data/shop/orders.ibd"), "current", "data untouched");
        assert_eq!(list_in(&b.join("backups")).len(), 1, "no pre-restore backup taken");
    }

    #[test]
    fn restore_rejects_files_that_are_not_campp_backups() {
        let t = fake_install();
        let b = t.path();
        fs::create_dir_all(b.join("backups")).unwrap();
        fs::write(b.join("backups/garbage.zip"), "not a zip").unwrap();
        assert!(restore_in(b, "garbage.zip", &HashMap::new()).is_err());

        // A real zip without the CAMPP manifest.
        let mut zip = zip::ZipWriter::new(File::create(b.join("backups/other.zip")).unwrap());
        zip.start_file("projects/x.txt", zip::write::SimpleFileOptions::default()).unwrap();
        zip.finish().unwrap();
        let err = restore_in(b, "other.zip", &HashMap::new()).unwrap_err();
        assert!(err.contains("Not a CAMPP backup"), "{err}");
        assert_eq!(read(b, "projects/site/index.php"), "v1", "data untouched");
    }

    #[test]
    fn restore_keeps_folders_the_backup_does_not_have() {
        let t = fake_install();
        let b = t.path();
        fs::remove_dir_all(b.join("pgsql")).unwrap(); // PostgreSQL not installed at backup time
        let name = create_in(b, None, &HashMap::new()).unwrap();

        fs::create_dir_all(b.join("pgsql/data")).unwrap();
        fs::write(b.join("pgsql/data/PG_VERSION"), "added later").unwrap();
        restore_in(b, &name, &HashMap::new()).unwrap();
        assert_eq!(read(b, "pgsql/data/PG_VERSION"), "added later");
    }

    #[test]
    fn delete_all_empties_projects_and_databases_after_a_backup() {
        let t = fake_install();
        let b = t.path();
        let safety = delete_all_in(b, &HashMap::new()).unwrap();

        assert!(b.join("projects").is_dir(), "projects folder kept");
        assert_eq!(fs::read_dir(b.join("projects")).unwrap().count(), 0, "projects folder emptied");
        assert!(!b.join("mysql/data").exists());
        assert!(!b.join("pgsql/data").exists());
        assert!(b.join("backups").join(&safety).is_file(), "backups are not deleted");

        restore_in(b, &safety, &HashMap::new()).unwrap();
        assert_eq!(read(b, "projects/site/index.php"), "v1");
        assert_eq!(read(b, "mysql/data/shop/orders.ibd"), "orders-v1");
    }

    #[test]
    fn backups_in_the_same_second_get_distinct_names_and_are_listed() {
        let t = fake_install();
        let b = t.path();
        let a = create_in(b, None, &HashMap::new()).unwrap();
        let c = create_in(b, None, &HashMap::new()).unwrap();
        assert_ne!(a, c);
        let names: Vec<String> = list_in(&b.join("backups")).into_iter().map(|x| x.name).collect();
        assert!(names.contains(&a) && names.contains(&c), "{names:?}");
        assert!(!names.iter().any(|n| n.ends_with(".part")));

        fs::remove_file(backup_path_in(b, &a).unwrap()).unwrap();
        assert_eq!(list_in(&b.join("backups")).len(), 1);
    }

    #[test]
    fn backup_path_stays_inside_backups_folder() {
        assert!(backup_path_in(Path::new("base"), "campp-backup-1.zip").is_ok());
        for bad in ["../settings.zip", "..\\x.zip", "sub/x.zip", "C:\\x.zip", "notes.txt", "x.zip/.."] {
            assert!(backup_path_in(Path::new("base"), bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn compatibility_rules() {
        let cur = m(&[("mysql", "8.4.11"), ("postgresql", "18.6.1")]);
        let ok = |b: &HashMap<String, String>, my: bool, pg: bool| check_compatible(b, &cur, my, pg, "windows", "windows");
        // Same engine, older/equal MySQL, same PG major: ok
        assert!(ok(&m(&[("mysql", "8.0.46"), ("postgresql", "18.3.0")]), true, true).is_ok());
        // MySQL downgrade refused
        assert!(ok(&m(&[("mysql", "9.1.0")]), true, false).is_err());
        // Engine switch (MariaDB backup on a MySQL install) refused
        assert!(ok(&m(&[("mariadb", "12.3.3")]), true, false).is_err());
        // PG major change refused
        assert!(ok(&m(&[("postgresql", "17.11.0")]), false, true).is_err());
        // No data for an engine: its version doesn't matter
        assert!(ok(&m(&[("postgresql", "17.11.0")]), false, false).is_ok());

        let same = m(&[("mysql", "8.4.11")]);
        // Database data from another OS refused (lower_case_table_names / PG locales differ)
        assert!(check_compatible(&same, &cur, true, false, "windows", "macos").is_err());
        // Projects-only backup moves between OSes fine
        assert!(check_compatible(&same, &cur, false, false, "windows", "macos").is_ok());
        // Old backups without a recorded OS: not blocked
        assert!(check_compatible(&same, &cur, true, false, "", "macos").is_ok());
    }
}
