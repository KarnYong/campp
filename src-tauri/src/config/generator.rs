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

// Server configuration (optimized for performance)
$i = 0;
$i++;
$cfg['Servers'][$i]['auth_type'] = 'config';
$cfg['Servers'][$i]['user'] = 'root';
$cfg['Servers'][$i]['password'] = '';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '{}';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = true;
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
