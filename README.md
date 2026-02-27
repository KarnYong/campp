# CAMPP

A cross-platform local web development stack desktop application. CAMPP bundles Caddy (web server), PHP-FPM 8.5 (PHP runtime), MariaDB (database), and phpMyAdmin with no external dependencies after installation.

## Features

- **Zero Configuration**: Works out of the box with sensible defaults
- **No Admin Required**: Uses non-default ports to avoid conflicts
- **Cross-Platform**: Windows, macOS, and Linux support
- **Self-Contained**: All binaries bundled, no separate installations needed
- Modern desktop UI built with Tauri + React + TypeScript

## Included Components

| Component | Version | Description |
|-----------|---------|-------------|
| Caddy | 2.8.4 | Modern web server with automatic HTTPS |
| PHP-FPM | 8.5.1 | Fast and reliable PHP runtime |
| MariaDB | 12.2.2 | Drop-in replacement for MySQL |
| phpMyAdmin | 5.2.2 | Web-based MySQL administration interface |

## Default Configuration

| Service | Port |
|----------|------|
| Web Server | 8080 |
| PHP-FPM | 9000 |
| MariaDB | 3307 |

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run tauri dev

# Build for production
npm run build

# Run tests
npm run test
```

### Rust Backend (src-tauri/)

```bash
# Build Rust backend
cd src-tauri
cargo build

# Run Rust tests
cargo test

# Check Rust code
cargo clippy
```

## Project Structure

```
campp/
├── src/                    # React + TypeScript frontend
│   ├── components/        # UI components
│   │   ├── Dashboard.tsx
│   │   ├── ServiceCard.tsx
│   │   ├── FirstRunWizard.tsx
│   │   └── StatusBar.tsx
│   ├── types/              # TypeScript type definitions
│   └── App.tsx             # Main app component
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands.rs     # Tauri IPC commands
│   │   ├── process/        # Process management
│   │   ├── config/         # Configuration generation
│   │   ├── runtime/        # Binary download system
│   │   └── database/       # MariaDB integration
│   └── Cargo.toml          # Rust dependencies
└── DEVELOPMENT_PLAN.md     # Implementation roadmap
```

## Getting Started

1. **First Run**: The app will download required binaries on first launch (~500MB)
2. **Start Services**: Use the dashboard to start/stop/restart each service
3. **Open Browser**: Access `http://localhost:8080` to view your projects
4. **phpMyAdmin**: Access `http://localhost:8080/phpmyadmin` for database management

## Roadmap

See [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for the full implementation roadmap.

### Implemented Features

- ✅ Phase 1: Project Foundation - Dashboard UI with service cards
- ✅ Phase 2: Runtime Download System - First-run binary installation wizard

### In Progress

- 🔄 Phase 3: Process Manager - Service start/stop/restart functionality
- 🔄 Phase 4-8: Configuration, MariaDB initialization, Settings, etc.

## License

MIT

## Contributing

Contributions are welcome! Please read [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) for information about the project architecture and implementation phases.
