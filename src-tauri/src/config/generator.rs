/// Configuration file generation for CAMPP services
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct ConfigGenerator {
    pub caddy_port: u16,
    pub php_port: u16,
    pub mysql_port: u16,
    pub phpmyadmin_path: String,
    pub project_root: String,
    pub config_dir: String,
    pub logs_dir: String,
}

impl ConfigGenerator {
    pub fn new(
        caddy_port: u16,
        php_port: u16,
        mysql_port: u16,
        phpmyadmin_path: String,
        project_root: String,
        config_dir: String,
        logs_dir: String,
    ) -> Self {
        Self {
            caddy_port,
            php_port,
            mysql_port,
            phpmyadmin_path,
            project_root,
            config_dir,
            logs_dir,
        }
    }

    /// Generate Caddyfile content
    pub fn generate_caddyfile(&self) -> String {
        let phpmyadmin = self.phpmyadmin_path.replace('\\', "/");
        let project_root = self.project_root.replace('\\', "/");
        let log_file = format!("{}/caddy-access.log", self.logs_dir.replace('\\', "/"));

        format!(
            r#":{caddy_port} {{
    # phpMyAdmin - must come before global directives
    # Redirect /phpmyadmin to /phpmyadmin/
    redir /phpmyadmin /phpmyadmin/

    # Handle phpMyAdmin requests - handle_path strips the /phpmyadmin prefix
    handle_path /phpmyadmin/* {{
        root * "{phpmyadmin}"
        php_fastcgi 127.0.0.1:{php_port}
        file_server browse
    }}

    # Root directory for serving files (default project root)
    root * "{project_root}"

    # Enable PHP for all other requests
    php_fastcgi 127.0.0.1:{php_port}

    # File server for project files
    file_server browse

    # Logging
    log {{
        output file "{log_file}"
        format json
    }}

    # Encode responses
    encode gzip

    # Security headers
    header {{
        X-Content-Type-Options nosniff
        X-Frame-Options SAMEORIGIN
        Referrer-Policy no-referrer
    }}
}}
"#,
            caddy_port = self.caddy_port,
            php_port = self.php_port,
            phpmyadmin = phpmyadmin,
            project_root = project_root,
            log_file = log_file
        )
    }

    /// Generate php.ini content
    pub fn generate_php_ini(&self) -> String {
        let error_log = format!("{}/php-errors.log", self.logs_dir.replace('\\', "/"));

        format!(
            r#"; CAMPP PHP Configuration
; Generated for PHP 8.3 with development settings

[PHP]
; Error reporting - suppress deprecation warnings for phpMyAdmin compatibility
error_reporting = E_ALL & ~E_DEPRECATED
display_errors = On
display_startup_errors = On
log_errors = On
error_log = "{error_log}"

; Maximum execution time
max_execution_time = 300

; Memory limit
memory_limit = 256M

; POST data limit
post_max_size = 100M
upload_max_filesize = 100M

; Date timezone
date.timezone = UTC

; Extensions
extension_dir = "ext"
extension=curl
extension=mbstring
extension=mysqli
extension=openssl
extension=pdo_mysql
extension=zlib

; Session settings
session.save_path = "/tmp"
session.cookie_httponly = 1
session.use_strict_mode = 1

; CGI settings
cgi.force_redirect = 0
cgi.fix_pathinfo = 1

; Security settings
expose_php = Off
"#,
            error_log = error_log
        )
    }

    /// Generate phpMyAdmin config.inc.php content
    pub fn generate_phpmyadmin_config(&self, blowfish_secret: &str) -> String {
        format!(
            r#"<?php
/**
 * CAMPP phpMyAdmin Configuration
 * Generated automatically - you can customize this file
 */

// Cookie encryption key (exactly 32 bytes)
$cfg['blowfish_secret'] = '{}';

// Server configuration
$i = 0;
$i++;
$cfg['Servers'][$i]['auth_type'] = 'config';
$cfg['Servers'][$i]['user'] = 'root';
$cfg['Servers'][$i]['password'] = '';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '{}';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = true;

// Upload directory
$cfg['UploadDir'] = '';
$cfg['SaveDir'] = '';

// Temp directory
$cfg['TempDir'] = './tmp/';

// Disable configuration storage warning (optional advanced features)
$cfg['PmaNoRelation_DisableWarning'] = true;

// Default language
$cfg['DefaultLang'] = 'en';

// Theme
$cfg['ThemeDefault'] = 'pmahomme';
"#,
            blowfish_secret, self.mysql_port
        )
    }

    /// Write all configuration files to disk
    pub fn write_configs(&self, config_dir: &Path) -> Result<(), String> {
        // Ensure config directory exists
        fs::create_dir_all(config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;

        // Write Caddyfile
        let caddyfile_path = config_dir.join("Caddyfile");
        let caddyfile_content = self.generate_caddyfile();
        let mut file = fs::File::create(&caddyfile_path)
            .map_err(|e| format!("Failed to create Caddyfile: {}", e))?;
        file.write_all(caddyfile_content.as_bytes())
            .map_err(|e| format!("Failed to write Caddyfile: {}", e))?;

        // Write php.ini
        let php_ini_path = config_dir.join("php.ini");
        let php_ini_content = self.generate_php_ini();
        let mut file = fs::File::create(&php_ini_path)
            .map_err(|e| format!("Failed to create php.ini: {}", e))?;
        file.write_all(php_ini_content.as_bytes())
            .map_err(|e| format!("Failed to write php.ini: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_generator_new() {
        let gen = ConfigGenerator::new(
            8080,
            9000,
            3307,
            "/path/to/phpmyadmin".to_string(),
            "/path/to/projects".to_string(),
            "/path/to/config".to_string(),
            "/path/to/logs".to_string(),
        );

        assert_eq!(gen.caddy_port, 8080);
        assert_eq!(gen.php_port, 9000);
        assert_eq!(gen.mysql_port, 3307);
    }

    #[test]
    fn test_generate_caddyfile() {
        let gen = ConfigGenerator::new(
            8080,
            9000,
            3307,
            "/path/to/phpmyadmin".to_string(),
            "/path/to/projects".to_string(),
            "/path/to/config".to_string(),
            "/path/to/logs".to_string(),
        );

        let content = gen.generate_caddyfile();

        assert!(content.contains(":8080"));
        assert!(content.contains("127.0.0.1:9000"));
        assert!(content.contains("/phpmyadmin"));
        assert!(content.contains("php_fastcgi"));
    }

    #[test]
    fn test_generate_php_ini() {
        let gen = ConfigGenerator::new(
            8080,
            9000,
            3307,
            "/path/to/phpmyadmin".to_string(),
            "/path/to/projects".to_string(),
            "/path/to/config".to_string(),
            "/path/to/logs".to_string(),
        );

        let content = gen.generate_php_ini();

        assert!(content.contains("E_ALL & ~E_DEPRECATED"));
        assert!(content.contains("extension=mysqli"));
        assert!(content.contains("memory_limit = 256M"));
    }

    #[test]
    fn test_generate_phpmyadmin_config() {
        let gen = ConfigGenerator::new(
            8080,
            9000,
            3307,
            "/path/to/phpmyadmin".to_string(),
            "/path/to/projects".to_string(),
            "/path/to/config".to_string(),
            "/path/to/logs".to_string(),
        );

        let content = gen.generate_phpmyadmin_config("test_secret_key_32_bytes_long!!");

        assert!(content.contains("test_secret_key_32_bytes_long!!"));
        assert!(content.contains("3307"));
        assert!(content.contains("$cfg['Servers'][$i]['host'] = '127.0.0.1'"));
    }
}
