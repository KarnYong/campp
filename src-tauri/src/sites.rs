//! Project folders and their `<name>.localhost` addresses, for the Sites page
//! (spec 005 US2, data-model.md). The address itself is served by the
//! `*.localhost` block in config/generator.rs; this only explains it.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteStatus {
    Ok,
    /// Not a hostname label: spaces, dots, underscores, non-Latin letters, > 63 chars.
    InvalidName,
    /// Linux only: folder names are case-sensitive, addresses are lowercase.
    Uppercase,
    /// Two folders that differ only in capitals (case-sensitive file systems).
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    /// `public/index.php` and no top-level `index.php` (Laravel, Symfony...): served from public/.
    Framework,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Site {
    pub name: String,
    pub host: Option<String>,
    pub url: Option<String>,
    pub kind: SiteKind,
    pub status: SiteStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SitesResponse {
    pub projects_dir: String,
    pub web_port: u16,
    pub sites: Vec<Site>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub has_public_index: bool,
    pub has_index: bool,
}

/// Same rule as the Caddyfile's `@badname` matcher: a lowercase hostname label.
fn is_label(name: &str) -> bool {
    let ok_char = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    !name.is_empty()
        && name.len() <= 63
        && name.chars().all(ok_char)
        && !name.starts_with('-')
        && !name.ends_with('-')
}

pub fn classify(entries: &[Entry], case_sensitive_fs: bool, web_port: u16) -> Vec<Site> {
    let mut sites: Vec<Site> = entries
        .iter()
        .map(|e| {
            let lower = e.name.to_lowercase();
            let status = if !is_label(&lower) {
                SiteStatus::InvalidName
            } else if entries.iter().filter(|o| o.name.to_lowercase() == lower).count() > 1 {
                SiteStatus::Conflict
            } else if case_sensitive_fs && e.name.chars().any(|c| c.is_ascii_uppercase()) {
                SiteStatus::Uppercase
            } else {
                SiteStatus::Ok
            };
            let host = (status == SiteStatus::Ok).then(|| format!("{lower}.localhost"));
            Site {
                name: e.name.clone(),
                url: host.as_ref().map(|h| format!("http://{h}:{web_port}")),
                host,
                kind: if e.has_public_index && !e.has_index { SiteKind::Framework } else { SiteKind::Plain },
                status,
            }
        })
        .collect();
    sites.sort_by_key(|s| s.name.to_lowercase());
    sites
}

/// Scan the projects folder one level deep. Files and hidden (`.`) entries are skipped;
/// symlinks/junctions to folders count as folders. A missing folder = no sites.
pub fn list_sites() -> Result<SitesResponse, String> {
    let settings = crate::config::AppSettings::load();
    let dir = std::path::PathBuf::from(&settings.project_root);
    let mut entries = Vec::new();
    if dir.is_dir() {
        let read = std::fs::read_dir(&dir)
            .map_err(|e| format!("Cannot read projects folder {}: {}", dir.display(), e))?;
        for item in read.flatten() {
            let name = item.file_name().to_string_lossy().into_owned();
            let path = item.path();
            if name.starts_with('.') || !path.is_dir() {
                continue;
            }
            entries.push(Entry {
                has_public_index: path.join("public").join("index.php").is_file(),
                has_index: path.join("index.php").is_file(),
                name,
            });
        }
    }
    Ok(SitesResponse {
        projects_dir: dir.to_string_lossy().into_owned(),
        web_port: settings.web_port,
        sites: classify(&entries, cfg!(target_os = "linux"), settings.web_port),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> Entry {
        Entry { name: name.into(), has_public_index: false, has_index: true }
    }

    fn one(e: Entry, case_sensitive: bool) -> Site {
        classify(&[e], case_sensitive, 8080).remove(0)
    }

    #[test]
    fn plain_folder_gets_an_address() {
        let s = one(entry("blog"), false);
        assert_eq!(s.status, SiteStatus::Ok);
        assert_eq!(s.kind, SiteKind::Plain);
        assert_eq!(s.host.as_deref(), Some("blog.localhost"));
        assert_eq!(s.url.as_deref(), Some("http://blog.localhost:8080"));
    }

    #[test]
    fn framework_layout_is_detected() {
        let shop = Entry { name: "shop".into(), has_public_index: true, has_index: false };
        assert_eq!(one(shop, false).kind, SiteKind::Framework);
        let both = Entry { name: "both".into(), has_public_index: true, has_index: true };
        assert_eq!(one(both, false).kind, SiteKind::Plain);
    }

    #[test]
    fn names_that_are_not_hostname_labels_get_no_address() {
        let long = "a".repeat(64);
        for name in ["My Notes", "a.b", "under_score", "ไทย", "-x", "x-", long.as_str()] {
            let s = one(entry(name), false);
            assert_eq!(s.status, SiteStatus::InvalidName, "{name}");
            assert_eq!((s.host, s.url), (None, None), "{name}");
        }
        assert_eq!(one(entry(&"a".repeat(63)), false).status, SiteStatus::Ok);
        assert_eq!(one(entry("my-app-2"), false).status, SiteStatus::Ok);
    }

    #[test]
    fn capitals_conflict_or_need_lowercase_only_on_case_sensitive_file_systems() {
        let both = classify(&[entry("Blog"), entry("blog")], true, 8080);
        assert!(both.iter().all(|s| s.status == SiteStatus::Conflict), "{both:?}");
        assert_eq!(one(entry("Blog"), true).status, SiteStatus::Uppercase);
        let win = one(entry("Blog"), false);
        assert_eq!(win.status, SiteStatus::Ok);
        assert_eq!(win.host.as_deref(), Some("blog.localhost"));
    }

    #[test]
    fn sorted_by_name_ignoring_case() {
        let names: Vec<String> = classify(&[entry("zeta"), entry("Alpha"), entry("beta")], false, 8080)
            .into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Alpha", "beta", "zeta"]);
    }
}
