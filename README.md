# CAMPP

A cross-platform local web development stack desktop application. CAMPP bundles Caddy (web server), PHP-FPM (PHP runtime), MySQL/MariaDB and PostgreSQL (databases), plus phpMyAdmin and Adminer, with no admin rights and no external dependencies after installation.

## Installation

Download the latest release from [GitHub Releases](https://github.com/KarnYong/campp/releases/latest).

### Windows

| File | What it is |
|------|------------|
| `CAMPP-<version>-x64.exe` | NSIS installer (recommended) |
| `CAMPP-<version>-x64.msi` | MSI installer |
| `CAMPP-<version>-x64-portable.zip` | Portable, no install, downloads runtimes on first run |
| `CAMPP-<version>-x64-portable-offline.zip` | Portable with runtimes bundled, works fully offline |

1. Run the installer. No admin permissions are required.
2. If SmartScreen warns, click **More info → Run anyway**.
3. Launch CAMPP from the Start Menu or desktop shortcut.

For the portable zips, extract anywhere you can write to and run `CAMPP.exe`.

### Linux (Ubuntu/Debian/Fedora)

**DEB package (recommended)**
```bash
sudo apt install ./CAMPP-<version>-amd64.deb
```

**RPM package**
```bash
sudo dnf install ./CAMPP-<version>-amd64.rpm
```

**AppImage**
```bash
chmod +x CAMPP-<version>-amd64.AppImage
./CAMPP-<version>-amd64.AppImage
```

**Portable tarball**
```bash
tar -xzf CAMPP-<version>-amd64-portable.tar.gz
./CAMPP/CAMPP
```

**To uninstall (DEB):**
```bash
sudo apt remove campp
rm -rf ~/.local/share/campp
```

### macOS

1. Download `CAMPP-<version>-universal.dmg` (Apple Silicon and Intel)
   - Or `CAMPP-<version>-arm64.dmg` (Apple Silicon) / `CAMPP-<version>-x64.dmg` (Intel)
   - Or `CAMPP-<version>-universal-portable.zip` (no DMG)
2. Open the DMG and drag CAMPP to Applications.
3. Launch CAMPP. macOS builds are signed and notarized. If Gatekeeper still complains, right-click the app and choose **Open**.

## Features

- **Zero configuration**: works out of the box with sensible defaults
- **No admin required**: uses non-default ports to avoid conflicts with system services
- **Self-contained**: every binary lives in CAMPP's own folder, with no separate installations
- **Version choice**: choose PHP, database, and tool versions in the setup wizard
- **Optional components**: install only what you need (MySQL/MariaDB, PostgreSQL, phpMyAdmin, Adminer, ngrok), and add, reinstall or remove components later on the Components page
- **Service management**: start, stop, and restart services individually, or all at once
- **Port configuration**: change each service's port in Settings
- **LAN mode**: make services reachable from other devices on your network
- **Public tunnels (ngrok)**: share your local site at a public URL, handy for webhooks and demos (**Windows only**)
- **Database root passwords**: set during setup for MySQL/MariaDB and PostgreSQL. You can change them later in **Settings → Database passwords**.
- **Backup & restore**: projects and all database data in one zip, restorable from the Backup page
- **Adminer launcher**: opens the login for your local MySQL or PostgreSQL with server and user filled in
- **Quick open**: website, phpMyAdmin, Adminer, and projects folder are one click away
- **Themes**: System, Light, and Dark
- **Debug page**: paths, port checks, copyable diagnostics, and a full reset

## Included Components

The default (recommended) version is listed first. Other versions can be picked during setup or reinstall.

| Component | Versions | Platforms |
|-----------|----------|-----------|
| Caddy | **2.11.7**, 2.11.4 | All |
| PHP-FPM | **8.5**, 8.4, 8.3, 8.2 | All |
| PHP (legacy) | 7.4.33 | Windows only |
| MySQL | **8.4.11 LTS**, 8.0.46 | Windows, macOS |
| MariaDB | **12.3.3 LTS**, 11.8.9, 11.4.13, 10.11.19 | Linux (x64) |
| PostgreSQL | **18.6.1**, 17.11, 16.15, 15.19 | All (optional) |
| phpMyAdmin | **5.2.3** | All (optional) |
| Adminer | **5.4.2**, 6.1.2 | All (optional) |
| ngrok | **3.19.2** | Windows (optional) |

On Linux and macOS, the PHP 8.5 package is a static build from static-php-cli.

### Runtime Sources

| Component | Source |
|-----------|--------|
| Caddy | [caddyserver/caddy releases](https://github.com/caddyserver/caddy/releases/) |
| PHP | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [PHP for Windows](https://downloads.php.net/~windows/releases/) and [static-php-cli](https://dl.static-php.dev/static-php-cli/bulk/)) |
| MySQL | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (from [MySQL Downloads](https://dev.mysql.com/downloads/mysql/)) |
| MariaDB | [MariaDB Archive](https://archive.mariadb.org/) |
| PostgreSQL | [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) |
| phpMyAdmin | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (from [phpMyAdmin](https://www.phpmyadmin.net/)) |
| Adminer | [vrana/adminer](https://github.com/vrana/adminer) |
| ngrok | [ngrok.com](https://ngrok.com/) |

Versions and download URLs are defined in `src-tauri/runtime-config.json`.

## Default Configuration

| Service | Port | Access |
|---------|------|--------|
| Web server (Caddy) | 8080 | http://localhost:8080 |
| PHP-FPM | 9000 | Internal (FastCGI) |
| MySQL / MariaDB | 3307 | 127.0.0.1:3307 |
| PostgreSQL | 5433 | 127.0.0.1:5433 |
| phpMyAdmin | 8080 | http://localhost:8080/phpmyadmin |
| Adminer | 8080 | http://localhost:8080/adminer/ |

**Database credentials**: user `root`, with the password you set in the setup wizard. The same applies to MySQL/MariaDB and PostgreSQL.

**Changing ports**: stop all services first, then edit ports in **Settings**. Changes take effect the next time services start.

## Getting Started

1. **Install** CAMPP from the release that matches your platform.
2. **First run**: the setup wizard lets you choose components and versions and set database root passwords. It then downloads the runtimes (several hundred MB). The offline portable zip skips the download.
3. **Start services**: click **Start all** on the dashboard, or start services one at a time.
4. **Build sites**: put your files in the projects folder (**Quick open → Projects folder**), then browse http://localhost:8080.
5. **Manage databases**: use phpMyAdmin or Adminer from **Quick open**.

The full user manual (`src-tauri/MANUAL.html`) ships with the app. Open it with **User manual** in the sidebar.

## Development

```bash
npm install          # Install dependencies
npm run tauri dev    # Run the app in development mode
npm run tauri build  # Build installers
npm test             # Frontend tests (vitest)
```

### Rust Backend (src-tauri/)

```bash
cd src-tauri
cargo build    # Build Rust backend
cargo test     # Run Rust tests
cargo clippy   # Lint Rust code
cargo clean    # Remove build output (target/ grows to several GB)
```

### Releasing

Bump the version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` (CI checks that they match), then push a `v*` tag. GitHub Actions builds the Windows, macOS, and Linux installers and portable packages. See `github-workflow-guide.md`.

## Project Structure

```
campp/
├── src/                    # React + TypeScript frontend
│   ├── components/         # UI components (Dashboard, Settings, Debug, wizard)
│   ├── assets/logos/       # Component logos
│   ├── hooks/              # Custom React hooks
│   └── types/              # TypeScript definitions
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── process/        # Process management + welcome page
│   │   ├── config/         # Config generation (Caddyfile, php.ini, my.ini, Adminer)
│   │   ├── runtime/        # Package list, download & locate binaries
│   │   ├── database/       # MySQL/MariaDB & PostgreSQL init
│   │   └── commands.rs     # Tauri IPC commands
│   ├── runtime-config.json # Runtime versions, URLs, checksums
│   └── MANUAL.html         # User manual (bundled resource)
└── .github/workflows/      # CI/CD (build & release)
```

## System Requirements

- **Windows**: Windows 10/11 (x64, or ARM64 via x64 emulation), with the WebView2 runtime (usually pre-installed)
- **Linux**: Ubuntu 22.04+ or a similar distribution with webkit2gtk
- **macOS**: macOS 13 (Ventura) or later, on Apple Silicon or Intel

## License

MIT

## Contributing

Contributions are welcome!
