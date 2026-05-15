use crate::runtime::locator::RuntimePaths;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub fn generate_caddyfile(path: &PathBuf, paths: &RuntimePaths, port: u16, php_port: u16) -> Result<(), String> {
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
    let phpmyadmin = paths.phpmyadmin
        .to_str()
        .ok_or("Invalid phpMyAdmin path")?
        .replace('\\', "/");

    let mut content = String::new();
    content.push_str(&format!("http://localhost:{} {{\n", port));
    content.push_str("    # phpMyAdmin - must come before global directives\n");
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
    content.push_str("    # Enable PHP for all other requests\n");
    content.push_str(&format!("    php_fastcgi 127.0.0.1:{}\n", php_port));
    content.push_str("\n");
    content.push_str("    # File server for project files\n");
    content.push_str("    file_server browse\n");
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

    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create Caddyfile: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write Caddyfile: {}", e))?;

    Ok(())
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

; OPcache settings optimized for phpMyAdmin performance
zend_extension=opcache
opcache.enable=1
opcache.memory_consumption=256
opcache.interned_strings_buffer=16
opcache.max_accelerated_files=40000
opcache.revalidate_freq=60
opcache.fast_shutdown=1
opcache.enable_cli=0
opcache.validate_timestamps=1
opcache.save_comments=1
opcache.jit=tracing
opcache.jit_buffer_size=128M

; Realpath cache for better file path resolution (doubled)
realpath_cache_size=8192K
realpath_cache_ttl=300
"#, error_log, ext_dir_str, session_path, session_path);

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

pub fn generate_phpmyadmin_config(paths: &RuntimePaths, mysql_port: u16, mysql_root_password: &str) -> Result<(), String> {
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

    let allow_no_password = if mysql_root_password.is_empty() { "true" } else { "false" };
    let escaped_password = mysql_root_password.replace('\'', "\\'");

    let config_content = format!(r#"<?php
/**
 * CAMPP phpMyAdmin Configuration
 * Generated automatically - you can customize this file
 */

// Cookie encryption key (exactly 32 bytes)
$cfg['blowfish_secret'] = '{}';

// Server configuration (optimized for performance)
$i = 0;
$i++;
$cfg['Servers'][$i]['auth_type'] = 'config';
$cfg['Servers'][$i]['user'] = 'root';
$cfg['Servers'][$i]['password'] = '{}';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '{}';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = {};
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
"#, blowfish_secret, escaped_password, mysql_port, allow_no_password, upload_dir_str, upload_dir_str, tmp_dir_str);

    let mut file = File::create(&config_path)
        .map_err(|e| format!("Failed to create phpMyAdmin config: {}", e))?;
    file.write_all(config_content.as_bytes())
        .map_err(|e| format!("Failed to write phpMyAdmin config: {}", e))?;

    Ok(())
}

/// Generate PostgreSQL configuration file
pub fn generate_postgresql_conf(data_dir: &PathBuf, port: u16) -> Result<(), String> {
    let path = data_dir.join("postgresql.conf");

    // Platform-specific shared memory type
    #[cfg(target_os = "windows")]
    let shared_memory_type = "windows";
    #[cfg(target_os = "macos")]
    let shared_memory_type = "posix";
    #[cfg(target_os = "linux")]
    let shared_memory_type = "posix";

    let content = format!(
        r#"# CAMPP PostgreSQL Configuration
# Generated automatically

# Connection settings
listen_addresses = '127.0.0.1'
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
        port, shared_memory_type
    );

    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create postgresql.conf: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write postgresql.conf: {}", e))?;

    Ok(())
}

/// Generate PostgreSQL client authentication config
pub fn generate_pg_hba_conf(data_dir: &PathBuf, auth_method: &str) -> Result<(), String> {
    let path = data_dir.join("pg_hba.conf");

    let content = format!(r#"# CAMPP PostgreSQL Client Authentication
# TYPE  DATABASE  USER  ADDRESS       METHOD
local   all       all                 {auth_method}
host    all       all   127.0.0.1/32  {auth_method}
host    all       all   ::1/128        {auth_method}
"#, auth_method = auth_method);

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
    mysql_root_password: &str,
    postgres_port: u16,
    postgres_root_password: &str,
) -> Result<(), String> {
    let adminer_dir = &paths.adminer;
    let index_path = adminer_dir.join("index.php");

    let escaped_mysql_pw = mysql_root_password.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_pg_pw = postgres_root_password.replace('\\', "\\\\").replace('\'', "\\'");

    let content = format!(r##"<?php
// CAMPP Adminer Launcher - Pre-configured database connections
$mysql_host = '127.0.0.1:{mysql_port}';
$mysql_user = 'root';
$mysql_pass = '{escaped_mysql_pw}';
$pg_host = '127.0.0.1:{postgres_port}';
$pg_user = 'root';
$pg_pass = '{escaped_pg_pw}';

// Mode 1: Auto-login from launcher click (only when 'auto' is the sole GET param)
if (isset($_GET['auto']) && count($_GET) === 1 && $_SERVER['REQUEST_METHOD'] === 'GET') {{
    session_start();
    $token = rand(1, 1000000);
    $_SESSION['token'] = $token;

    if ($_GET['auto'] === 'mysql') {{
        $driver = 'server';
        $server = $mysql_host;
        $user = $mysql_user;
        $pass = $mysql_pass;
    }} else {{
        $driver = 'pgsql';
        $server = $pg_host;
        $user = $pg_user;
        $pass = $pg_pass;
    }}

    $_POST = array(
        'token' => $token,
        'auth' => array(
            'driver' => $driver,
            'server' => $server,
            'username' => $user,
            'password' => $pass,
            'db' => '',
        ),
    );
    $_SERVER['REQUEST_METHOD'] = 'POST';
    include __DIR__ . '/adminer.php';
    exit;
}}

// Mode 2: Pass through to Adminer for all other requests
// (Adminer navigation has params like pgsql=, server=, table=, etc.)
// (Adminer form submissions are POST requests)
if (!empty($_GET) || $_SERVER['REQUEST_METHOD'] === 'POST') {{
    include __DIR__ . '/adminer.php';
    exit;
}}

// Mode 3: Clean /adminer/ visit - show launcher page
?>
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CAMPP - Database Manager</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: #1a1a2e; color: #e0e0e0;
    display: flex; align-items: center; justify-content: center;
    min-height: 100vh;
}}
.container {{ text-align: center; max-width: 600px; width: 100%; padding: 20px; }}
h1 {{ font-size: 1.8rem; margin-bottom: 0.5rem; color: #fff; }}
.subtitle {{ color: #888; margin-bottom: 2rem; font-size: 0.95rem; }}
.cards {{ display: flex; gap: 20px; justify-content: center; flex-wrap: wrap; }}
.card {{
    background: #16213e; border-radius: 12px; padding: 30px 24px;
    width: 260px; text-decoration: none; color: inherit;
    border: 1px solid #0f3460; cursor: pointer;
    transition: transform 0.15s, border-color 0.15s;
    display: flex; flex-direction: column; align-items: center; gap: 12px;
}}
.card:hover {{ transform: translateY(-3px); border-color: #533483; }}
.card.mysql:hover {{ border-color: #00758f; }}
.card.pgsql:hover {{ border-color: #336791; }}
.card .icon {{ font-size: 2.5rem; }}
.card .name {{ font-size: 1.1rem; font-weight: 600; }}
.card .detail {{ font-size: 0.82rem; color: #888; }}
.card .connect-btn {{
    margin-top: 8px; padding: 8px 20px; border: none; border-radius: 6px;
    font-size: 0.9rem; cursor: pointer; font-weight: 500;
    transition: opacity 0.15s;
}}
.card .connect-btn:hover {{ opacity: 0.85; }}
.card.mysql .connect-btn {{ background: #00758f; color: #fff; }}
.card.pgsql .connect-btn {{ background: #336791; color: #fff; }}
.manual {{ margin-top: 2rem; font-size: 0.85rem; }}
.manual a {{ color: #7f8fa6; text-decoration: none; }}
.manual a:hover {{ text-decoration: underline; }}
</style>
</head>
<body>
<div class="container">
    <h1>CAMPP Database Manager</h1>
    <p class="subtitle">Select a database to manage</p>
    <div class="cards">
        <a href="?auto=mysql" class="card mysql">
            <div class="icon">&#128200;</div>
            <div class="name">MySQL</div>
            <div class="detail">127.0.0.1:{mysql_port}</div>
            <div class="detail">User: root</div>
            <button class="connect-btn" type="button">Connect</button>
        </a>
        <a href="?auto=pgsql" class="card pgsql">
            <div class="icon">&#128202;</div>
            <div class="name">PostgreSQL</div>
            <div class="detail">127.0.0.1:{postgres_port}</div>
            <div class="detail">User: root</div>
            <button class="connect-btn" type="button">Connect</button>
        </a>
    </div>
    <p class="manual">Or use <a href="adminer.php">manual login</a></p>
</div>
</body>
</html>"##,
        mysql_port = mysql_port,
        postgres_port = postgres_port,
        escaped_mysql_pw = escaped_mysql_pw,
        escaped_pg_pw = escaped_pg_pw,
    );

    let mut file = File::create(&index_path)
        .map_err(|e| format!("Failed to create Adminer index.php: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write Adminer index.php: {}", e))?;

    Ok(())
}
