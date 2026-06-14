# CAMPP

A cross-platform local web development stack desktop application. CAMPP bundles Caddy (web server), PHP-FPM (PHP runtime), MySQL/MariaDB (database), and phpMyAdmin with no external dependencies after installation.

## Installation

Download the latest release from [GitHub Releases](https://github.com/KarnYong/campp/releases).

### Windows

1. Download `CAMPP-<version>-x64.exe` (NSIS installer) or `CAMPP-<version>-x64.msi`
2. Run the installer — no admin permissions required
3. Launch CAMPP from the Start Menu or desktop shortcut

### Linux (Ubuntu/Debian)

**Option A: DEB package (recommended)**
```bash
# Download the latest .deb
wget https://github.com/KarnYong/campp/releases/latest/download/CAMPP-0.3.6-amd64.deb

# Install
sudo dpkg -i CAMPP-0.3.6-amd64.deb

# If missing dependencies:
sudo apt-get install -f

# Run
/opt/campp/CAMPP
```

**Option B: AppImage**
```bash
# Download the latest AppImage
wget https://github.com/KarnYong/campp/releases/latest/download/CAMPP-0.3.6-amd64.AppImage

# Make executable
chmod +x CAMPP-0.3.6-amd64.AppImage

# Run
./CAMPP-0.3.6-amd64.AppImage
```

**To uninstall:**
```bash
sudo dpkg -r CAMPP
rm -rf ~/.local/share/campp
```

### macOS

1. Download `CAMPP-<version>-universal.dmg` (works on both Apple Silicon and Intel)
   - Or `CAMPP-<version>-arm64.dmg` (Apple Silicon) / `CAMPP-<version>-x64.dmg` (Intel)
2. Open the DMG and drag CAMPP to Applications
3. On first launch, right-click the app and select Open (required for unsigned apps)

> **Note**: macOS builds are signed and notarized. If you encounter gatekeeper issues, right-click the app and select Open.

## Features

- **Zero Configuration**: Works out of the box with sensible defaults
- **No Admin Required**: Uses non-default ports to avoid conflicts
- **Self-Contained**: All binaries bundled, no separate installations needed
- **Modern UI**: Clean desktop interface built with Tauri + React + TypeScript
- **Service Management**: Start, stop, and restart services individually or all at once
- **Port Configuration**: Customize ports for each service via Settings panel
- **LAN Mode**: Expose services to other devices on your local network
- **Public Tunnels (ngrok)**: Expose your local server to the internet via a public URL — ideal for testing webhooks or sharing work in progress
- **Editable php.ini**: Manually customize PHP configuration (e.g. add extensions)
- **PostgreSQL Support**: Built-in PostgreSQL alongside MySQL/MariaDB
- **Database Password Management**: Set root passwords for MySQL and PostgreSQL from Settings
- **Optional Components**: Choose which components to install (phpMyAdmin, Adminer, MySQL, PostgreSQL)
- **Quick Actions**: Open project folder, phpMyAdmin, or logs with one click
- **PHP Version Choice**: Select PHP 8.5.1 or PHP 7.4.33 (Windows) for legacy application support
- **Platform-Appropriate Database**: MySQL 8.4.0 LTS on Windows/macOS, MariaDB 12.3.1 on Linux

## Included Components

| Component | Version | Description |
|-----------|---------|-------------|
| Caddy | 2.11.3 | Modern web server with automatic HTTPS |
| PHP-FPM | 8.5.1 (7.4.33 available on Windows) | Fast and reliable PHP runtime |
| MySQL | 8.4.0 LTS | Enterprise-grade database (**Windows & macOS**) |
| MariaDB | 12.3.1 | Community-developed database fork (**Linux**) |
| PostgreSQL | 18.3.0 | Advanced open-source database (**optional**) |
| phpMyAdmin | 5.2.2 | Web-based database administration interface |
| Adminer | 5.4.2 | Lightweight database manager (**optional**) |
| ngrok | 3.19.2 | Secure public tunneling tool (**optional**) |

### Runtime Sources

Runtime binaries are downloaded from the following sources:

| Component | Platform | Source |
|-----------|----------|--------|
| Caddy | All | [caddyserver/caddy releases](https://github.com/caddyserver/caddy/releases/) |
| PHP-FPM | Windows/macOS/Linux | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [PHP for Windows](https://downloads.php.net/~windows/releases/) and [static-php-cli](https://dl.static-php.dev/static-php-cli/bulk/)) |
| MySQL | Windows/macOS | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [MySQL Downloads](https://dev.mysql.com/downloads/mysql/)) |
| MariaDB | Linux | [MariaDB Archive](https://archive.mariadb.org/) |
| PostgreSQL | All | [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) |
| phpMyAdmin | All | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [phpMyAdmin](https://www.phpmyadmin.net/)) |
| Adminer | All | [vrana/adminer](https://github.com/vrana/adminer) |
| ngrok | All | [ngrok.com](https://ngrok.com/) |

> **Note**: The `campp-runtime-binaries` repository contains pre-compiled binaries packaged for CAMPP's specific requirements.

## Default Configuration

| Service | Port | Access |
|---------|------|--------|
| Web Server | 8080 | http://localhost:8080 |
| PHP-FPM | 9000 | Internal (FastCGI) |
| Database (MySQL/MariaDB) | 3307 | localhost:3307 |
| PostgreSQL | 5433 | localhost:5433 |
| phpMyAdmin | 8080 | http://localhost:8080/phpmyadmin |
| Adminer | 8080 | http://localhost:8080/adminer/ |

**Default Database Credentials**: `root` / (empty password) — for both MySQL/MariaDB and PostgreSQL

**Port Customization**: You can change ports in Settings (⚙️). Running services will automatically restart when you save.

## Getting Started

1. **Install**: Run the downloaded installer
2. **First Run**: The app will download required binaries on first launch (~500MB)
3. **Start Services**: Click "Start All" or start individual services from the dashboard
4. **View Projects**: Access http://localhost:8080 to view your projects
5. **Manage Database**: Access http://localhost:8080/phpmyadmin or http://localhost:8080/adminer/ for database management

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run tauri dev

# Build for production
npm run tauri build
```

### Rust Backend (src-tauri/)

```bash
cd src-tauri
cargo build    # Build Rust backend
cargo test     # Run Rust tests
cargo clippy   # Lint Rust code
```

## Project Structure

```
campp/
├── src/                    # React + TypeScript frontend
│   ├── components/        # UI components
│   ├── hooks/             # Custom React hooks
│   └── types/             # TypeScript definitions
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── process/       # Process management
│   │   ├── config/        # Configuration generation
│   │   ├── runtime/       # Binary download system
│   │   └── database/      # MariaDB integration
│   └── templates/         # Service config templates
└── .github/workflows/     # CI/CD (build & release)
```

## System Requirements

- **Windows**: Windows 10/11 x64, WebView2 runtime (usually pre-installed)
- **Linux**: Ubuntu 22.04+ or similar distributions with webkit2gtk dependencies
- **macOS**: macOS 13+ (Ventura or later), Apple Silicon or Intel

## License

MIT

## Contributing

Contributions are welcome!
