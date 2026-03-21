# CAMPP

A cross-platform local web development stack desktop application. CAMPP bundles Caddy (web server), PHP-FPM 8.3 (PHP runtime), MariaDB (database), and phpMyAdmin with no external dependencies after installation.

## Installation

Download the latest release from [GitHub Releases](https://github.com/KarnYong/campp/releases):

- **Windows**: Download `CAMPP_x64-setup.exe` (NSIS installer) or `CAMPP_x64_en-US.msi`
- **Linux**: Download `CAMPP_amd64.AppImage` and make it executable: `chmod +x CAMPP_amd64.AppImage`
- **macOS**: Download `CAMPP_aarch64.dmg` (Apple Silicon) or `CAMPP_x64.dmg` (Intel)

> **Note**: macOS builds are available but need more testing volunteers. If you encounter any issues on macOS, please report them!

## Features

- **Zero Configuration**: Works out of the box with sensible defaults
- **No Admin Required**: Uses non-default ports to avoid conflicts
- **Self-Contained**: All binaries bundled, no separate installations needed
- **Modern UI**: Clean desktop interface built with Tauri + React + TypeScript
- **Service Management**: Start, stop, and restart services individually or all at once
- **Port Configuration**: Customize ports for each service via Settings panel
- **Quick Actions**: Open project folder, phpMyAdmin, or logs with one click

## Included Components

| Component | Version | Description |
|-----------|---------|-------------|
| Caddy | 2.8.4 | Modern web server with automatic HTTPS |
| PHP-FPM | 8.5.1 | Fast and reliable PHP runtime |
| MySQL | 8.4.0 LTS | Enterprise-grade database (MariaDB on Linux) |
| phpMyAdmin | 5.2.2 | Web-based MySQL administration interface |

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
| MariaDB | 3307 | localhost:3307 |
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

## License

MIT

## Contributing

Contributions are welcome! Please read [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for project architecture details.

**macOS testers needed!** If you have a Mac and want to help test CAMPP, please download the latest release and report any issues you encounter.
