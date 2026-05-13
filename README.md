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
wget https://github.com/KarnYong/campp/releases/latest/download/CAMPP-0.3.1-amd64.deb

# Install
sudo dpkg -i CAMPP-0.3.1-amd64.deb

# If missing dependencies:
sudo apt-get install -f

# Run
/opt/campp/CAMPP
```

**Option B: AppImage**
```bash
# Download the latest AppImage
wget https://github.com/KarnYong/campp/releases/latest/download/CAMPP-0.3.1-amd64.AppImage

# Make executable
chmod +x CAMPP-0.3.1-amd64.AppImage

# Run
./CAMPP-0.3.1-amd64.AppImage
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
- **Quick Actions**: Open project folder, phpMyAdmin, or logs with one click
- **PHP Version Choice**: Select PHP 8.5.1 or PHP 7.4.33 (Windows) for legacy application support
- **Platform-Appropriate Database**: MySQL 8.4.0 LTS on Windows/macOS, MariaDB 12.3.1 on Linux

## Included Components

| Component | Version | Description |
|-----------|---------|-------------|
| Caddy | 2.8.4 | Modern web server with automatic HTTPS |
| PHP-FPM | 8.5.1 (7.4.33 available on Windows) | Fast and reliable PHP runtime |
| MySQL | 8.4.0 LTS | Enterprise-grade database (**Windows & macOS**) |
| MariaDB | 12.3.1 | Community-developed database fork (**Linux**) |
| phpMyAdmin | 5.2.2 | Web-based database administration interface |

### Runtime Sources

Runtime binaries are downloaded from the following sources:

| Component | Platform | Source |
|-----------|----------|--------|
| Caddy | All | [caddyserver/caddy releases](https://github.com/caddyserver/caddy/releases/) |
| PHP-FPM | Windows/macOS/Linux | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [PHP for Windows](https://downloads.php.net/~windows/releases/) and [static-php-cli](https://dl.static-php.dev/static-php-cli/bulk/)) |
| MySQL | Windows/macOS | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [MySQL Downloads](https://dev.mysql.com/downloads/mysql/)) |
| MariaDB | Linux | [MariaDB Archive](https://archive.mariadb.org/) |
| phpMyAdmin | All | [campp-runtime-binaries](https://github.com/KarnYong/campp-runtime-binaries) (built from [phpMyAdmin](https://www.phpmyadmin.net/)) |

> **Note**: The `campp-runtime-binaries` repository contains pre-compiled binaries packaged for CAMPP's specific requirements.

## Default Configuration

| Service | Port | Access |
|---------|------|--------|
| Web Server | 8080 | http://localhost:8080 |
| PHP-FPM | 9000 | Internal (FastCGI) |
| Database (MySQL/MariaDB) | 3307 | localhost:3307 |
| phpMyAdmin | 8080 | http://localhost:8080/phpmyadmin |

**Default Database Credentials**: `root` / (empty password)

**Port Customization**: You can change ports in Settings (⚙️). Running services will automatically restart when you save.

## Getting Started

1. **Install**: Run the downloaded installer
2. **First Run**: The app will download required binaries on first launch (~500MB)
3. **Start Services**: Click "Start All" or start individual services from the dashboard
4. **View Projects**: Access http://localhost:8080 to view your projects
5. **Manage Database**: Access http://localhost:8080/phpmyadmin for database management

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
└── DEVELOPMENT_PLAN.md    # Implementation roadmap
```

## Roadmap

See [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for the full implementation roadmap.

### Implemented Features

- ✅ Phase 1: Project Foundation - Tauri + React setup
- ✅ Phase 2: Runtime Download System - First-run binary download wizard
- ✅ Phase 3: Process Manager - Service start/stop/restart
- ✅ Phase 4: Configuration Generation - Dynamic config files
- ✅ Phase 5: MariaDB Initialization - Database setup and credential management
- ✅ Phase 6: Enhanced Dashboard UI - Service control interface
- ✅ Phase 7: Settings Panel - Port configuration and project folder selection
- ✅ Phase 8: Cross-platform Support - Windows, Linux, macOS installers

## System Requirements

- **Windows**: Windows 10/11 x64, WebView2 runtime (usually pre-installed)
- **Linux**: Ubuntu 22.04+ or similar distributions with webkit2gtk dependencies
- **macOS**: macOS 11+ (Big Sur or later), Apple Silicon or Intel

## Troubleshooting

### MySQL/MariaDB fails to start after unclean shutdown

If MySQL/MariaDB was not shut down properly (e.g., power failure, force quit), you may see an InnoDB error like:

```
[ERROR] [MY-012960] [InnoDB] Cannot create redo log files because data files are corrupt
```

CAMPP automatically handles this by starting MySQL with InnoDB recovery mode enabled (`--innodb-force-recovery=1`). This allows the database to start and recover automatically after unclean shutdowns.

If you still encounter issues, you can manually reset the database:

1. **Stop all services** in CAMPP
2. **Delete the MySQL data directory**:
   - Windows: Delete `C:\Users\<YourUsername>\.campp\mysql\data\`
   - Linux/macOS: Delete `~/.campp/mysql/data/`
3. **Restart CAMPP** - the database will be re-initialized automatically

**Warning**: Deleting the data directory will destroy all your databases. Export your data first using phpMyAdmin if you need to preserve it.

## License

MIT

## Contributing

Contributions are welcome! Please read [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for project architecture details.

**macOS testers needed!** If you have a Mac and want to help test CAMPP, please download the latest release and report any issues you encounter.
