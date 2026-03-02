# CAMPP

A cross-platform local web development stack desktop application. CAMPP bundles Caddy (web server), PHP-FPM 8.3 (PHP runtime), MariaDB (database), and phpMyAdmin with no external dependencies after installation.

## Installation

Download the latest release from [GitHub Releases](https://github.com/KarnYong/campp/releases):

- **Windows**: Download `CAMPP_0.1.0_x64-setup.exe` (NSIS installer) or `CAMPP_0.1.0_x64_en-US.msi`

## Features

- **Zero Configuration**: Works out of the box with sensible defaults
- **No Admin Required**: Uses non-default ports to avoid conflicts
- **Self-Contained**: All binaries bundled, no separate installations needed
- **Modern UI**: Clean desktop interface built with Tauri + React + TypeScript
- **Service Management**: Start, stop, and restart services individually or all at once
- **Quick Actions**: Open project folder, phpMyAdmin, or logs with one click

## Included Components

| Component | Version | Description |
|-----------|---------|-------------|
| Caddy | 2.8.4 | Modern web server with automatic HTTPS |
| PHP-FPM | 8.3.x | Fast and reliable PHP runtime |
| MariaDB | 11.4.x | Drop-in replacement for MySQL |
| phpMyAdmin | 5.2.x | Web-based MySQL administration interface |

## Default Configuration

| Service | Port | Access |
|---------|------|--------|
| Web Server | 8080 | http://localhost:8080 |
| PHP-FPM | 9000 | Internal (FastCGI) |
| MariaDB | 3307 | localhost:3307 |
| phpMyAdmin | 8080 | http://localhost:8080/phpmyadmin |

**Default Database Credentials**: `root` / `root`

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

- Phase 1: Project Foundation - Tauri + React setup
- Phase 2: Runtime Download System - First-run binary download wizard
- Phase 3: Process Manager - Service start/stop/restart
- Phase 4: Configuration Generation - Dynamic config files

### Coming Soon

- Phase 5: MariaDB Initialization
- Phase 6: Enhanced Dashboard UI
- Phase 7: Settings Panel
- Phase 8: macOS/Linux Support

## System Requirements

- **Windows**: Windows 10/11 x64, WebView2 runtime (usually pre-installed)
- **macOS**: Coming soon
- **Linux**: Coming soon

## License

MIT

## Contributing

Contributions are welcome! Please read [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for project architecture details.
