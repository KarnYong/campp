use crate::runtime::locator::RuntimePaths;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn generate_caddyfile(path: &PathBuf, paths: &RuntimePaths, port: u16, php_port: u16, bind_address: &str) -> Result<(), String> {
    let projects_raw = paths.projects_dir
        .to_str()
        .ok_or("Invalid project path")?;

    if projects_raw.contains('"') || projects_raw.contains('\n') || projects_raw.contains('}') || projects_raw.contains('{') {
        return Err("Invalid project path: contains characters not allowed in Caddyfile".to_string());
    }
    let projects = projects_raw.replace('\\', "/");
    let log_file = paths.logs_dir.join("caddy-access.log")
        .to_str()
        .ok_or("Invalid log path")?
        .replace('\\', "/");

    // CAMPP never uses Caddy's admin API (it starts with `run --config`, stops by
    // killing the process). Turning it off frees localhost:2019, so two Caddys (an
    // installed CAMPP and a CAMPP_HOME test build, or the user's own Caddy) can run.
    let mut content = String::from("{\n    admin off\n}\n\n");
    // Always use :{port} as the site address so Caddy matches any Host header
    // (needed for ngrok tunnels which forward with the ngrok domain as Host).
    // Use `bind` directive to restrict listen address when not in LAN mode.
    let site_addr = format!(":{}", port);
    content.push_str(&format!("{} {{\n", site_addr));
    if bind_address != "0.0.0.0" {
        content.push_str(&format!("    bind {}\n", bind_address));
    }

    // Add phpMyAdmin route only if installed
    if paths.phpmyadmin.join("index.php").exists() {
        let phpmyadmin = paths.phpmyadmin
            .to_str()
            .ok_or("Invalid phpMyAdmin path")?
            .replace('\\', "/");
        content.push_str("    # Redirect /phpmyadmin to /phpmyadmin/\n");
        content.push_str("    redir /phpmyadmin /phpmyadmin/\n");
        content.push_str("\n");
        content.push_str("    # Handle phpMyAdmin requests - handle_path strips the /phpmyadmin prefix\n");
        content.push_str("    handle_path /phpmyadmin/* {\n");
        content.push_str(&format!("        root * \"{}\"\n", phpmyadmin));
        content.push_str(&format!("        php_fastcgi 127.0.0.1:{}\n", php_port));
        content.push_str("        file_server browse\n");
        content.push_str("    }\n");
        content.push_str("\n");
    }

    // Add Adminer route if directory exists
    let adminer_php = paths.adminer.join("adminer.php");
    if adminer_php.exists() {
        let adminer = paths.adminer
            .to_str()
            .ok_or("Invalid Adminer path")?
            .replace('\\', "/");
        content.push_str("    # Adminer - database management (supports MySQL and PostgreSQL)\n");
        content.push_str("    redir /adminer /adminer/\n");
        content.push_str("\n");
        content.push_str("    handle_path /adminer/* {\n");
        content.push_str(&format!("        root * \"{}\"\n", adminer));
        content.push_str(&format!("        php_fastcgi 127.0.0.1:{} {{\n", php_port));
        content.push_str("            index index.php\n");
        content.push_str("        }\n");
        content.push_str("        file_server browse\n");
        content.push_str("    }\n");
        content.push_str("\n");
    }
    content.push_str("    # Root directory for serving files (default project root)\n");
    content.push_str(&format!("    root * \"{}\"\n", projects));
    content.push_str("\n");
    content.push_str("    # A folder with index.html but no trailing slash: redirect (as php_fastcgi does\n");
    content.push_str("    # for index.php), so relative links in the page resolve inside the folder.\n");
    content.push_str("    @htmlDir {\n");
    content.push_str("        file {path}/index.html\n");
    content.push_str("        not path */\n");
    content.push_str("    }\n");
    content.push_str("    redir @htmlDir {path}/ 308\n");
    content.push_str("\n");
    content.push_str("    # Serve PHP files via FastCGI. A folder serves its index.php or index.html;\n");
    content.push_str("    # anything else is 404. php_fastcgi's default falls back to the root index.php,\n");
    content.push_str("    # which showed the CAMPP welcome page for every missing page or folder.\n");
    content.push_str(&format!("    php_fastcgi 127.0.0.1:{} {{\n", php_port));
    content.push_str("        index index.php\n");
    content.push_str("        try_files {path} {path}/index.php {path}/index.html =404\n");
    content.push_str("    }\n");
    content.push_str("\n");
    content.push_str("    # File server for project files\n");
    content.push_str("    file_server\n");
    content.push_str("\n");
    content.push_str("    # Visible 404 (an empty body is a blank page in Firefox/Safari)\n");
    content.push_str("    handle_errors 404 {\n");
    content.push_str("        respond \"404 Not Found\" 404\n");
    content.push_str("    }\n");
    content.push_str("\n");
    content.push_str("    # Logging\n");
    content.push_str("    log {\n");
    content.push_str(&format!("        output file \"{}\"\n", log_file));
    content.push_str("        format json\n");
    content.push_str("    }\n");
    content.push_str("\n");
    content.push_str("    # Encode responses\n");
    content.push_str("    encode gzip\n");
    content.push_str("\n");
    content.push_str("    # Security headers\n");
    content.push_str("    header {\n");
    content.push_str("        X-Content-Type-Options nosniff\n");
    content.push_str("        X-Frame-Options SAMEORIGIN\n");
    content.push_str("        Referrer-Policy no-referrer\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    // Spec 005: every project folder at http://<folder>.localhost:<port>. One block for
    // all folders: {labels.1} is the folder name (blog.localhost -> blog), resolved per
    // request, so a new folder works without regenerating or reloading anything.
    // Caddy picks this more specific site for *.localhost hosts; localhost, LAN IP and
    // ngrok hosts keep using the site above. http:// = no automatic HTTPS listeners.
    let bind = if bind_address != "0.0.0.0" { format!("    bind {bind_address}\n") } else { String::new() };
    content.push_str(&format!(
        r#"
http://*.localhost:{port} {{
{bind}
    @badname not expression `size({{labels.1}}) <= 63 && {{labels.1}}.matches('^[a-z0-9]([a-z0-9-]*[a-z0-9])?$')`
    @framework {{
        file {{
            root "{projects}/{{labels.1}}"
            try_files public/index.php
        }}
        not file {{
            root "{projects}/{{labels.1}}"
            try_files index.php
        }}
    }}
    @plain file {{
        root "{projects}"
        try_files {{labels.1}}/
    }}

    handle @badname {{
        respond "No site named {{labels.1}}. Project folders live in {projects}" 404
    }}
    # Laravel/Symfony layout: public/ is the root; Caddy's default php_fastcgi sends
    # every path that isn't a file to public/index.php (the front controller).
    handle @framework {{
        root * "{projects}/{{labels.1}}/public"
        php_fastcgi 127.0.0.1:{php_port}
        file_server
    }}
    handle @plain {{
        root * "{projects}/{{labels.1}}"
        php_fastcgi 127.0.0.1:{php_port} {{
            index index.php
            try_files {{path}} {{path}}/index.php {{path}}/index.html =404
        }}
        file_server
    }}
    handle {{
        respond "No site named {{labels.1}}. Project folders live in {projects}" 404
    }}

    log {{
        output file "{log_file}"
        format json
    }}
    encode gzip
    header {{
        X-Content-Type-Options nosniff
        X-Frame-Options SAMEORIGIN
        Referrer-Policy no-referrer
    }}
}}
"#
    ));

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create Caddyfile: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write Caddyfile: {}", e))?;

    Ok(())
}

/// `campp-<FNV-1a of the php.ini path>`: stable for one install, different for each
/// install folder (FNV, not DefaultHasher, whose output may change between Rust versions).
fn opcache_cache_id(php_ini: &Path) -> String {
    let hash = php_ini
        .to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    format!("campp-{hash:016x}")
}

pub fn generate_php_ini(path: &PathBuf, paths: &RuntimePaths) -> Result<(), String> {
    let php_dir = paths.php_cgi.parent()
        .ok_or("Cannot determine PHP directory")?;

    let ext_dir = php_dir.join("ext");
    let ext_dir_str = ext_dir.to_string_lossy().replace('\\', "/");

    let error_log = paths.logs_dir.join("php-errors.log")
        .to_string_lossy()
        .replace('\\', "/");
    let session_path = paths.logs_dir.join("php-sessions")
        .to_string_lossy()
        .replace('\\', "/");

    let php_ini_content = format!(r#"; CAMPP PHP Configuration
; Basic PHP settings for development

[PHP]
; Error reporting - suppress deprecation warnings for phpMyAdmin compatibility
error_reporting = E_ALL & ~E_DEPRECATED & ~E_WARNING
display_errors = On
display_startup_errors = Off
log_errors = On
error_log = "{}"

; Maximum execution time
max_execution_time = 300
max_input_time = 300

; Memory limit
memory_limit = 256M

; POST data limit
post_max_size = 100M
upload_max_filesize = 100M
max_input_vars = 5000

; Date timezone
date.timezone = UTC

; Extensions - use absolute path for reliability
; Note: zlib and session are built-in to PHP 8.3 and cannot be loaded as extensions
extension_dir = "{}"
extension=curl
extension=mbstring
extension=mysqli
extension=openssl
extension=pdo
extension=pdo_mysql
extension=pdo_pgsql
extension=pgsql
extension=zip
extension=fileinfo
extension=pdo_sqlite
extension=sqlite3
extension=intl
extension=gd
extension=sodium

; Session settings - use absolute path for Windows compatibility
session.save_path = "{}"
session.cookie_httponly = 1
session.use_strict_mode = 1
session.use_cookies = 1
session.use_trans_sid = 0

; File uploads
upload_tmp_dir = "{}"

; CGI settings
cgi.force_redirect = 0
cgi.fix_pathinfo = 1

; Security settings
expose_php = Off

; OPcache settings — revalidate_freq=1 so edited PHP files show on next reload
zend_extension=opcache
opcache.enable=1
opcache.memory_consumption=256
opcache.interned_strings_buffer=16
opcache.max_accelerated_files=40000
opcache.revalidate_freq=1
opcache.fast_shutdown=1
opcache.enable_cli=0
opcache.validate_timestamps=1
opcache.save_comments=1
opcache.jit=tracing
opcache.jit_buffer_size=128M
; Windows: PHP processes of one version under one user share a single OPcache unless
; their cache_id differs. A second PHP (another CAMPP, a CAMPP_HOME test build, XAMPP)
; would otherwise attach to it and die with "Opcode handlers are unusable due to ASLR".
opcache.cache_id={}

; Realpath cache for better file path resolution (doubled)
realpath_cache_size=8192K
realpath_cache_ttl=300
"#, error_log, ext_dir_str, session_path, session_path, opcache_cache_id(path));

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create php.ini: {}", e))?;
    file.write_all(php_ini_content.as_bytes())
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok(())
}

pub fn generate_php_fpm_conf(path: &PathBuf, paths: &RuntimePaths, php_port: u16) -> Result<(), String> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "nobody".to_string());

    let fpm_conf_content = format!(
        r#"; CAMPP PHP-FPM Configuration
; Optimized for phpMyAdmin performance

[global]
error_log = {logs_dir}/php-fpm.log
log_level = warning

[www]
user = {user}
group = {user}
listen = 127.0.0.1:{php_port}
listen.owner = {user}
listen.group = {user}
listen.mode = 0660

; Process manager - static for better performance (no spawning delays)
pm = static
pm.max_children = 10

; Worker recycling to prevent memory leaks
pm.max_requests = 1000

; Request settings for phpMyAdmin
request_terminate_timeout = 300
php_admin_value[error_log] = {logs_dir}/php-fpm.log
php_admin_flag[log_errors] = on
php_value[session.save_path] = {logs_dir}/php-sessions

; Performance tuning
php_value[memory_limit] = 256M
"#,
        logs_dir = paths.logs_dir.display().to_string().replace('\\', "/"),
        user = user,
        php_port = php_port,
    );

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create php-fpm.conf: {}", e))?;
    file.write_all(fpm_conf_content.as_bytes())
        .map_err(|e| format!("Failed to write php-fpm.conf: {}", e))?;

    Ok(())
}

pub fn generate_phpmyadmin_config(paths: &RuntimePaths, mysql_port: u16) -> Result<(), String> {
    let config_path = paths.phpmyadmin.join("config.inc.php");

    let tmp_dir = paths.phpmyadmin.join("tmp");
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create phpMyAdmin tmp directory: {}", e))?;

    let upload_dir = paths.logs_dir.join("phpmyadmin_uploads");
    fs::create_dir_all(&upload_dir)
        .map_err(|e| format!("Failed to create phpMyAdmin upload directory: {}", e))?;

    let blowfish_secret: String = (0..32)
        .map(|_| {
            const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let idx = rand::random::<usize>() % CHARSET.len();
            CHARSET[idx] as char
        })
        .collect();

    let tmp_dir_str = tmp_dir.to_string_lossy().replace('\\', "/");
    let upload_dir_str = upload_dir.to_string_lossy().replace('\\', "/");

    let config_content = format!(r#"<?php
/**
 * CAMPP phpMyAdmin Configuration
 * Generated automatically - you can customize this file
 */

// Cookie encryption key (exactly 32 bytes)
$cfg['blowfish_secret'] = '{}';

// Caddy serves phpMyAdmin under /phpmyadmin/ but strips that prefix (handle_path),
// so phpMyAdmin would think it lives at the site root and redirect there after
// login. Built per request so it's right for localhost, a LAN IP and ngrok (https).
$cfg['PmaAbsoluteUri'] = ((($_SERVER['HTTP_X_FORWARDED_PROTO'] ?? '') === 'https' || !empty($_SERVER['HTTPS'])) ? 'https' : 'http')
    . '://' . ($_SERVER['HTTP_HOST'] ?? '127.0.0.1') . '/phpmyadmin/';

// Server configuration (optimized for performance)
// Login screen, no stored password: phpMyAdmin is reachable from other devices
// in LAN mode and through ngrok, so it must never log anyone in automatically.
$i = 0;
$i++;
$cfg['Servers'][$i]['auth_type'] = 'cookie';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '{}';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = false;
$cfg['Servers'][$i]['hide_db'] = '^(information_schema|mysql|performance_schema)$';

// Performance optimizations
$cfg['Servers'][$i]['persistent_connections'] = true;
$cfg['Servers'][$i]['connect_type'] = 'tcp';
$cfg['Servers'][$i]['DisableIS'] = true;
$cfg['Servers'][$i]['MaxTableUiprefs'] = 100;

// Configuration storage is disabled by default
// To enable: create phpmyadmin database, pma user, and import sql/create_tables.sql
// $cfg['Servers'][$i]['pmadb'] = 'phpmyadmin';
// $cfg['Servers'][$i]['controluser'] = 'pma';
// $cfg['Servers'][$i]['controlpass'] = '';

// Upload and save directories
$cfg['UploadDir'] = '{}';
$cfg['SaveDir'] = '{}';

// Temp directory (absolute path for reliability)
$cfg['TempDir'] = '{}';

// Disable database statistics warning and server info
$cfg['ShowServerInfo'] = false;
$cfg['ShowPhpInfo'] = false;
$cfg['ShowChangelogUrl'] = false;

// Default language
$cfg['DefaultLang'] = 'en';

// Theme
$cfg['ThemeDefault'] = 'pmahomme';

// Performance settings
$cfg['MemoryLimit'] = '256M';
$cfg['LoginCookieValidity'] = 1440;
$cfg['ExecTimeLimit'] = 300;

// Navigation and query optimizations
$cfg['NavigationTreeEnableGrouping'] = true;
$cfg['NavigationTreeDisplayItemFilterMinimum'] = 30;
$cfg['FirstDayOfCalendar'] = 1;

// Execution time (for large database operations)
$cfg['ExecTimeLimit'] = 0;
"#, blowfish_secret, mysql_port, upload_dir_str, upload_dir_str, tmp_dir_str);

    let mut file = File::create(&config_path)
        .map_err(|e| format!("Failed to create phpMyAdmin config: {}", e))?;
    file.write_all(config_content.as_bytes())
        .map_err(|e| format!("Failed to write phpMyAdmin config: {}", e))?;

    Ok(())
}

/// Generate PostgreSQL configuration file
pub fn generate_postgresql_conf(data_dir: &PathBuf, port: u16, listen_all_interfaces: bool) -> Result<(), String> {
    let path = data_dir.join("postgresql.conf");

    // Platform-specific shared memory type
    #[cfg(target_os = "windows")]
    let shared_memory_type = "windows";
    #[cfg(target_os = "macos")]
    let shared_memory_type = "posix";
    #[cfg(target_os = "linux")]
    let shared_memory_type = "posix";

    let pg_listen = if listen_all_interfaces { "*" } else { "127.0.0.1" };
    let content = format!(
        r#"# CAMPP PostgreSQL Configuration
# Generated automatically

# Connection settings
listen_addresses = '{}'
port = {}
max_connections = 100

# Memory settings
shared_buffers = 128MB
dynamic_shared_memory_type = {}

# Logging
logging_collector = on
log_directory = 'log'
log_filename = 'postgresql-%Y-%m-%d.log'
log_statement = 'none'
log_min_duration_statement = -1

# Locale
datestyle = 'iso, mdy'
timezone = 'UTC'
lc_messages = 'C'
lc_monetary = 'C'
lc_numeric = 'C'
lc_time = 'C'
default_text_search_config = 'pg_catalog.english'
"#,
        pg_listen, port, shared_memory_type
    );

    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create postgresql.conf: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write postgresql.conf: {}", e))?;

    Ok(())
}

/// Generate PostgreSQL client authentication config
pub fn generate_pg_hba_conf(data_dir: &PathBuf, auth_method: &str, listen_all_interfaces: bool) -> Result<(), String> {
    let path = data_dir.join("pg_hba.conf");

    let mut content = format!(r#"# CAMPP PostgreSQL Client Authentication
# TYPE  DATABASE  USER  ADDRESS       METHOD
local   all       all                 {auth_method}
host    all       all   127.0.0.1/32  {auth_method}
host    all       all   ::1/128        {auth_method}
"#, auth_method = auth_method);

    // Other devices always need the password, even while `auth_method` is "trust" for
    // the first-start password setup (that only needs this computer). If no password is
    // set yet, scram fails and remote logins are simply refused.
    if listen_all_interfaces {
        content.push_str("host    all       all   0.0.0.0/0     scram-sha-256\n");
        content.push_str("host    all       all   ::/0          scram-sha-256\n");
    }

    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create pg_hba.conf: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write pg_hba.conf: {}", e))?;

    Ok(())
}

/// Generate Adminer launcher page with pre-configured MySQL and PostgreSQL servers
pub fn generate_adminer_config(
    paths: &RuntimePaths,
    mysql_port: u16,
    postgres_port: u16,
) -> Result<(), String> {
    let adminer_dir = &paths.adminer;
    let index_path = adminer_dir.join("index.php");

    let content = format!(r##"<?php
// CAMPP Adminer Launcher. The buttons open Adminer's login form with server and
// user filled in; the password is always typed. No password is stored here, since
// Adminer is reachable from other devices in LAN mode and through ngrok.

// Any Adminer request (navigation params like server=, pgsql=, table=, or a form POST)
// goes straight to Adminer.
if (!empty($_GET) || $_SERVER['REQUEST_METHOD'] === 'POST') {{
    include __DIR__ . '/adminer.php';
    exit;
}}

// Plain /adminer/ visit: show the launcher page
?>
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CAMPP - Database Manager</title>
<link rel="stylesheet" href="campp-launcher.css">
</head>
<body>
  <main class="card">
    <div class="hero">
      <img src="data:image/png;base64,{logo}" alt="CAMPP">
      <h1>Database <span>Manager</span></h1>
      <p class="muted">Adminer for your local databases. Log in as root with your database password.</p>
    </div>

    <section>
      <h2>Connect</h2>
      <ul>
        <li><a href="?server=127.0.0.1:{mysql_port}&amp;username=root">
          <span class="db"><strong>MySQL</strong><span class="mono muted">127.0.0.1:{mysql_port} · root</span></span>
          <span class="pill">Log in</span>
        </a></li>
        <li><a href="?pgsql=127.0.0.1:{postgres_port}&amp;username=root">
          <span class="db"><strong>PostgreSQL</strong><span class="mono muted">127.0.0.1:{postgres_port} · root</span></span>
          <span class="pill">Log in</span>
        </a></li>
      </ul>
    </section>

    <footer class="muted">Other server or user? Use <a href="adminer.php">manual login</a></footer>
  </main>
</body>
</html>"##,
        mysql_port = mysql_port,
        postgres_port = postgres_port,
        logo = LAUNCHER_LOGO_B64.trim(),
    );

    let mut file = File::create(&index_path)
        .map_err(|e| format!("Failed to create Adminer index.php: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write Adminer index.php: {}", e))?;

    // Launcher styles only — Adminer itself only loads adminer.css / adminer-dark.css.
    fs::write(adminer_dir.join("campp-launcher.css"), LAUNCHER_CSS)
        .map_err(|e| format!("Failed to write Adminer launcher CSS: {}", e))?;

    Ok(())
}

const LAUNCHER_CSS: &str = include_str!("adminer_launcher.css");
const LAUNCHER_LOGO_B64: &str = include_str!("../process/welcome_logo.b64");

/// Generate ngrok configuration file
pub fn generate_ngrok_config(
    path: &PathBuf,
    authtoken: &str,
    caddy_port: u16,
) -> Result<(), String> {
    let content = format!(
        r#"version: "2"
authtoken: {}
web_addr: "127.0.0.1:4040"
tunnels:
  caddy:
    proto: http
    addr: {}
"#,
        authtoken, caddy_port,
    );

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create ngrok config: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write ngrok config: {}", e))?;

    Ok(())
}

/// Extensions Composer/Laravel need that php.ini files written before CAMPP enabled
/// them lack: zip (Composer unpacks dists), fileinfo, pdo_sqlite + sqlite3 (Laravel's
/// default DB), intl (Number helpers, `artisan db:show`), gd (images), sodium. On
/// static Linux/macOS builds these are compiled in, so the lines are inert.
const EXTRA_EXTENSIONS: [&str; 7] = ["zip", "fileinfo", "pdo_sqlite", "sqlite3", "intl", "gd", "sodium"];

/// The extension a php.ini line names, active or commented out: `extension=intl`,
/// `; extension = "php_intl.dll"` → `intl`. A commented line counts as present, so a
/// user's opt-out is never re-enabled (spec 006 FR-005).
fn ini_extension_name(line: &str) -> Option<String> {
    let line = line.trim().trim_start_matches(';').trim_start();
    let (key, value) = line.split_once('=')?;
    if key.trim() != "extension" {
        return None;
    }
    let name = value.trim().trim_matches('"').to_ascii_lowercase();
    let name = name.strip_prefix("php_").unwrap_or(&name);
    let name = name.strip_suffix(".dll").or_else(|| name.strip_suffix(".so")).unwrap_or(name);
    Some(name.to_string())
}

/// Add any missing EXTRA_EXTENSIONS lines to an existing php.ini, right after
/// `extension=pgsql` (else at the end). Idempotent; keeps line endings; touches nothing else.
pub fn ensure_php_ini_extensions(path: &Path) -> Result<(), String> {
    let Ok(text) = fs::read_to_string(path) else { return Ok(()) };
    let updated = add_missing_extensions(&text);
    if updated != text {
        fs::write(path, updated).map_err(|e| format!("Failed to update php.ini: {}", e))?;
    }
    Ok(())
}

fn add_missing_extensions(text: &str) -> String {
    let present: Vec<String> = text.lines().filter_map(ini_extension_name).collect();
    let has = |ext: &str| present.iter().any(|p| p == ext);
    let missing: Vec<&str> = EXTRA_EXTENSIONS.iter().copied().filter(|e| !has(e)).collect();
    if missing.is_empty() {
        return text.to_string();
    }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let block: String = missing.iter().map(|e| format!("extension={e}{nl}")).collect();
    match text.find("extension=pgsql") {
        Some(pos) => {
            let line_end = text[pos..].find('\n').map(|i| pos + i + 1).unwrap_or(text.len());
            let sep = if line_end == text.len() && !text.ends_with('\n') { nl } else { "" };
            format!("{}{}{}{}", &text[..line_end], sep, block, &text[line_end..])
        }
        None => {
            let sep = if text.is_empty() || text.ends_with('\n') { "" } else { nl };
            format!("{text}{sep}{block}")
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn php_ini_upgrade_adds_missing_extensions_once() {
        let old = "[PHP]\r\nextension=openssl\r\nextension=pgsql\r\n\r\n; Session\r\nsession.save_path = \"x\"\r\n";
        let new = super::add_missing_extensions(old);
        for e in super::EXTRA_EXTENSIONS {
            assert_eq!(new.matches(&format!("extension={e}\r\n")).count(), 1, "{e}");
        }
        assert!(new.contains("extension=pgsql\r\nextension=zip\r\n"), "inserted after pgsql");
        assert!(new.ends_with("session.save_path = \"x\"\r\n"), "rest untouched");
        assert!(!new.replace("\r\n", "").contains('\n'), "keeps CRLF");
        assert_eq!(super::add_missing_extensions(&new), new, "idempotent");
        // Already present (fresh ini) -> unchanged; no pgsql line -> appended
        let all: String = super::EXTRA_EXTENSIONS.iter().map(|e| format!("extension={e}\n")).collect();
        assert_eq!(super::add_missing_extensions(&all), all);
        assert_eq!(super::add_missing_extensions("[PHP]"), format!("[PHP]\n{all}"));
    }

    #[test]
    fn laravel_extensions_are_added_to_old_ini_files() {
        // Spec 006 FR-004: intl (Number helpers, db:show), gd, sodium.
        for e in ["intl", "gd", "sodium"] {
            assert!(super::EXTRA_EXTENSIONS.contains(&e), "{e}");
        }
        let old = "extension=openssl\nextension=pgsql\nextension=zip\n";
        let new = super::add_missing_extensions(old);
        for e in ["intl", "gd", "sodium"] {
            assert_eq!(new.matches(&format!("extension={e}\n")).count(), 1, "{e}");
        }
        assert_eq!(new.matches("extension=zip\n").count(), 1);
    }

    #[test]
    fn commented_out_extension_is_the_users_choice() {
        // FR-005: a line the user commented out must not come back enabled.
        for line in [";extension=intl", "; extension = intl", ";extension=php_intl.dll", "  ;extension=\"intl\""] {
            let ini = format!("extension=pgsql\n{line}\n");
            let new = super::add_missing_extensions(&ini);
            assert!(!new.lines().any(|l| l.trim() == "extension=intl"), "{line}: {new}");
            assert!(new.contains(line), "user line kept: {line}");
        }
        // Other similar names don't count as intl.
        assert!(super::add_missing_extensions(";extension=intlx\n").contains("\nextension=intl\n"));
    }

    #[test]
    fn fresh_php_ini_enables_laravel_extensions() {
        use crate::runtime::locator::RuntimePaths;
        let dir = tempfile::TempDir::new().unwrap();
        let p = |n: &str| dir.path().join(n);
        let paths = RuntimePaths {
            caddy: p("caddy"), php_cgi: p("php-cgi"), php_ini: p("php.ini"), php_ext_dir: p("ext"),
            mysql: p("mysqld"), phpmyadmin: p("phpmyadmin"), pgsql_dir: p("pgsql"), adminer: p("adminer"),
            mysql_data_dir: p("data"), pgsql_data_dir: p("pgdata"), logs_dir: p("logs"),
            config_dir: p("config"), projects_dir: p("projects"), ngrok: p("ngrok"),
        };
        super::generate_php_ini(&paths.php_ini, &paths).unwrap();
        let ini = std::fs::read_to_string(&paths.php_ini).unwrap();
        for e in super::EXTRA_EXTENSIONS {
            assert_eq!(ini.lines().filter(|l| l.trim() == format!("extension={e}")).count(), 1, "{e}");
        }
        // The upgrade pass changes nothing on a fresh file.
        assert_eq!(super::add_missing_extensions(&ini), ini);
        // Own OPcache per install, so it can run beside another CAMPP's PHP (Windows).
        let id = super::opcache_cache_id(&paths.php_ini);
        assert!(ini.contains(&format!("\nopcache.cache_id={id}\n")), "{ini}");
    }

    #[test]
    fn opcache_cache_id_differs_per_install() {
        use std::path::Path;
        let a = super::opcache_cache_id(Path::new("C:/CAMPP/runtime/php.ini"));
        let b = super::opcache_cache_id(Path::new("C:/src/campp/.e2e/home/runtime/php.ini"));
        assert_ne!(a, b);
        assert_eq!(a, super::opcache_cache_id(Path::new("C:/CAMPP/runtime/php.ini")));
        assert!(a.starts_with("campp-") && a.len() == 22, "{a}");
    }

    #[test]
    fn caddyfile_gives_404_instead_of_falling_back_to_the_welcome_page() {
        use crate::runtime::locator::RuntimePaths;
        let dir = tempfile::TempDir::new().unwrap();
        let p = |n: &str| dir.path().join(n);
        let paths = RuntimePaths {
            caddy: p("caddy"), php_cgi: p("php-cgi"), php_ini: p("php.ini"), php_ext_dir: p("ext"),
            mysql: p("mysqld"), phpmyadmin: p("phpmyadmin"), pgsql_dir: p("pgsql"), adminer: p("adminer"),
            mysql_data_dir: p("data"), pgsql_data_dir: p("pgdata"), logs_dir: p("logs"),
            config_dir: p("config"), projects_dir: p("projects"), ngrok: p("ngrok"),
        };
        let file = dir.path().join("Caddyfile");
        super::generate_caddyfile(&file, &paths, 8080, 9000, "127.0.0.1").unwrap();
        let cf = std::fs::read_to_string(&file).unwrap();

        // Missing page or folder without index -> 404, never the root index.php.
        assert!(cf.contains("try_files {path} {path}/index.php {path}/index.html =404"), "{cf}");
        // No directory listings of project folders.
        assert!(!cf.contains("file_server browse"), "{cf}");
        // index.html folders redirect to the trailing slash like index.php folders.
        assert!(cf.contains("redir @htmlDir {path}/ 308"), "{cf}");
        assert!(cf.contains("respond \"404 Not Found\" 404"), "{cf}");
        // Admin API off (unused): a second Caddy (installed CAMPP + test build) would
        // otherwise fail on the shared admin port 2019.
        assert!(cf.starts_with("{\n    admin off\n}\n\n"), "{cf}");
    }

    fn caddyfile_for(bind: &str) -> (String, String) {
        use crate::runtime::locator::RuntimePaths;
        let dir = tempfile::TempDir::new().unwrap();
        let p = |n: &str| dir.path().join(n);
        let paths = RuntimePaths {
            caddy: p("caddy"), php_cgi: p("php-cgi"), php_ini: p("php.ini"), php_ext_dir: p("ext"),
            mysql: p("mysqld"), phpmyadmin: p("phpmyadmin"), pgsql_dir: p("pgsql"), adminer: p("adminer"),
            mysql_data_dir: p("data"), pgsql_data_dir: p("pgdata"), logs_dir: p("logs"),
            config_dir: p("config"), projects_dir: p("projects"), ngrok: p("ngrok"),
        };
        let file = dir.path().join("Caddyfile");
        super::generate_caddyfile(&file, &paths, 18080, 19000, bind).unwrap();
        let projects = paths.projects_dir.to_str().unwrap().replace('\\', "/");
        (std::fs::read_to_string(&file).unwrap(), projects)
    }

    #[test]
    fn caddyfile_serves_each_project_at_its_own_localhost_name() {
        // Spec 005 US1, contracts/caddyfile-sites.md
        let (cf, p) = caddyfile_for("127.0.0.1");
        assert!(cf.starts_with("{\n    admin off\n}\n\n"), "{cf}");
        let main = cf.find(":18080 {").expect("main site");
        let sites = cf.find("http://*.localhost:18080 {").expect("*.localhost site");
        assert!(main < sites, "main site stays first");
        let (main_block, site_block) = cf.split_at(sites);

        // The shared site is unchanged.
        assert!(main_block.contains("try_files {path} {path}/index.php {path}/index.html =404"));
        assert!(main_block.contains("respond \"404 Not Found\" 404"));

        // Folder name from the host, validated before it touches the file system.
        assert!(site_block.contains(
            "@badname not expression `size({labels.1}) <= 63 && {labels.1}.matches('^[a-z0-9]([a-z0-9-]*[a-z0-9])?$')`"
        ), "{site_block}");
        // Laravel/Symfony layout: public/index.php and no top-level index.php.
        assert!(site_block.contains(&format!(
            "    @framework {{\n        file {{\n            root \"{p}/{{labels.1}}\"\n            try_files public/index.php\n        }}\n        not file {{\n            root \"{p}/{{labels.1}}\"\n            try_files index.php\n        }}\n    }}\n"
        )), "{site_block}");
        assert!(site_block.contains(&format!(
            "    @plain file {{\n        root \"{p}\"\n        try_files {{labels.1}}/\n    }}\n"
        )), "{site_block}");
        // Framework: public/ root and Caddy's default front-controller fallback (no =404).
        assert!(site_block.contains(&format!(
            "    handle @framework {{\n        root * \"{p}/{{labels.1}}/public\"\n        php_fastcgi 127.0.0.1:19000\n        file_server\n    }}\n"
        )), "{site_block}");
        assert!(site_block.contains(&format!("    handle @plain {{\n        root * \"{p}/{{labels.1}}\"\n")), "{site_block}");
        assert!(site_block.contains("try_files {path} {path}/index.php {path}/index.html =404"));
        let no_site = format!("respond \"No site named {{labels.1}}. Project folders live in {p}\" 404");
        assert_eq!(site_block.matches(&no_site).count(), 2, "{site_block}");
        assert!(site_block.contains("bind 127.0.0.1"));
    }

    #[test]
    fn lan_mode_binds_both_sites_to_all_interfaces() {
        let (cf, _) = caddyfile_for("0.0.0.0");
        assert!(cf.contains("http://*.localhost:18080 {"));
        assert!(!cf.contains("bind "), "{cf}");
    }

    #[test]
    fn pg_hba_never_trusts_other_devices() {
        let dir = tempfile::TempDir::new().unwrap();
        let data = dir.path().to_path_buf();

        // First start in LAN mode: local "trust" to set the password, network still needs it.
        super::generate_pg_hba_conf(&data, "trust", true).unwrap();
        let hba = std::fs::read_to_string(data.join("pg_hba.conf")).unwrap();
        assert!(hba.contains("127.0.0.1/32  trust"));
        assert!(hba.contains("0.0.0.0/0     scram-sha-256"));
        assert!(hba.contains("::/0          scram-sha-256"));
        let remote_lines = hba.lines().filter(|l| l.contains("0.0.0.0/0") || l.contains("::/0"));
        assert!(remote_lines.clone().count() == 2 && remote_lines.into_iter().all(|l| !l.contains("trust")));

        // Local only: no network lines at all.
        super::generate_pg_hba_conf(&data, "scram-sha-256", false).unwrap();
        let hba = std::fs::read_to_string(data.join("pg_hba.conf")).unwrap();
        assert!(!hba.contains("0.0.0.0/0") && !hba.contains("::/0"));
    }

    #[test]
    fn phpmyadmin_requires_login_and_stores_no_password() {
        use crate::runtime::locator::RuntimePaths;
        let dir = tempfile::TempDir::new().unwrap();
        let p = |n: &str| dir.path().join(n);
        let paths = RuntimePaths {
            caddy: p("caddy"), php_cgi: p("php-cgi"), php_ini: p("php.ini"), php_ext_dir: p("ext"),
            mysql: p("mysqld"), phpmyadmin: p("phpmyadmin"), pgsql_dir: p("pgsql"), adminer: p("adminer"),
            mysql_data_dir: p("data"), pgsql_data_dir: p("pgdata"), logs_dir: p("logs"),
            config_dir: p("config"), projects_dir: p("projects"), ngrok: p("ngrok"),
        };
        std::fs::create_dir_all(&paths.phpmyadmin).unwrap();

        super::generate_phpmyadmin_config(&paths, 3307).unwrap();

        let cfg = std::fs::read_to_string(paths.phpmyadmin.join("config.inc.php")).unwrap();
        assert!(cfg.contains("['auth_type'] = 'cookie'"));
        assert!(!cfg.contains("['password']"), "no stored password");
        assert!(cfg.contains("['port'] = '3307'"));
        assert!(cfg.contains("'/phpmyadmin/'"), "PmaAbsoluteUri keeps the /phpmyadmin/ prefix");
    }

    #[test]
    fn adminer_launcher_uses_campp_theme_without_touching_adminer_css() {
        use crate::runtime::locator::RuntimePaths;
        let dir = tempfile::TempDir::new().unwrap();
        let p = |n: &str| dir.path().join(n);
        let paths = RuntimePaths {
            caddy: p("caddy"), php_cgi: p("php-cgi"), php_ini: p("php.ini"), php_ext_dir: p("ext"),
            mysql: p("mysqld"), phpmyadmin: p("phpmyadmin"), pgsql_dir: p("pgsql"), adminer: p("adminer"),
            mysql_data_dir: p("data"), pgsql_data_dir: p("pgdata"), logs_dir: p("logs"),
            config_dir: p("config"), projects_dir: p("projects"), ngrok: p("ngrok"),
        };
        std::fs::create_dir_all(&paths.adminer).unwrap();

        super::generate_adminer_config(&paths, 3307, 5433).unwrap();

        let index = std::fs::read_to_string(paths.adminer.join("index.php")).unwrap();
        assert!(index.contains(r#"href="campp-launcher.css""#));
        assert!(index.contains("data:image/png;base64,") && !index.contains("{logo}"));
        assert!(index.contains("?server=127.0.0.1:3307&amp;username=root"));
        assert!(index.contains("?pgsql=127.0.0.1:5433&amp;username=root"));
        assert!(!index.contains("$_POST"), "no auto-login");
        assert!(!index.contains("auto="), "auto-login removed");
        assert!(paths.adminer.join("campp-launcher.css").exists());
        // Adminer's own stylesheet hooks stay untouched.
        assert!(!paths.adminer.join("adminer.css").exists());
        assert!(!paths.adminer.join("adminer-dark.css").exists());
    }
}
