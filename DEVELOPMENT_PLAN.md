# CAMPP Desktop Application - Development Plan

## Context

CAMPP is a cross-platform desktop application (similar to XAMPP) that provides a complete local web development stack. The goal is to create a user-friendly control panel that bundles Caddy (web server), PHP-FPM (PHP runtime), MariaDB (database), and phpMyAdmin (database management UI) with no external dependencies required after installation.

**Target Users**: Developers who need a simple, portable local web stack without system-wide installations or administrator privileges.

**Key Differentiators**:
- No admin permissions required (uses non-default ports)
- Self-contained with bundled binaries
- Cross-platform (Windows/macOS/Linux)
- Modern Tauri + React UI

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    CAMPP Desktop App                        │
├─────────────────────────────────────────────────────────────┤
│  Frontend (React + TypeScript)                              │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
│  │ Dashboard   │ Service     │ Logs        │ Settings    │  │
│  │             │ Controls    │ Viewer      │             │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┘  │
│                         ↕ Tauri IPC                         │
├─────────────────────────────────────────────────────────────┤
│  Backend (Rust - Tauri)                                     │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
│  │ Process     │ Config      │ Port        │ Log         │  │
│  │ Manager     │ Generator   │ Detector    │ Collector   │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  Runtime Components                                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ Caddy    │ │ PHP-FPM  │ │ MariaDB  │ │ phpMyAdmin   │  │
│  │ :8080    │ │ :9000    │ │ :3307    │ │ /phpmyadmin/ │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
campp/
├── src-tauri/                      # Rust backend
│   ├── src/
│   │   ├── main.rs                 # Tauri entry point
│   │   ├── process/
│   │   │   ├── mod.rs
│   │   │   └── manager.rs         # Process spawn/control
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── generator.rs       # Config file generation
│   │   │   ├── ports.rs           # Port detection/allocation
│   │   │   └── settings.rs        # App settings persistence
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   └── mariadb.rs         # MariaDB initialization
│   │   ├── runtime/
│   │   │   ├── mod.rs
│   │   │   ├── downloader.rs      # Binary download system
│   │   │   └── locator.rs         # Runtime binary locator
│   │   └── commands.rs            # Tauri IPC commands
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs                   # Build script for resources
│   │
├── src/                           # React frontend
│   ├── App.tsx
│   ├── components/
│   │   ├── Dashboard.tsx          # Main dashboard
│   │   ├── ServiceCard.tsx        # Individual service control
│   │   ├── FirstRunWizard.tsx     # Download/setup wizard (MVP)
│   │   └── SettingsPanel.tsx      # Basic settings UI
│   ├── hooks/
│   │   ├── useServices.ts         # Service state management
│   │   └── useConfig.ts           # Config persistence
│   ├── types/
│   │   └── services.ts            # TypeScript interfaces
│   └── styles/
│   │   └── main.css
│   │
├── resources/                     # Bundled resources (lightweight)
│   └── phpmyadmin/                # Bundled phpMyAdmin only
│       └── (full phpMyAdmin distro)
│
├── templates/                     # Config templates
│   ├── Caddyfile.mustache
│   ├── php.ini.mustache
│   └── config.inc.php.mustache
│
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

---

## Implementation Phases

> **Strategy**: Simple MVP first, multi-platform from day one
> - MVP Target: Core functionality (start/stop services, basic dashboard)
> - v1.1 Target: Advanced features (multi-project, log viewer, advanced settings)
> - Distribution: Download-on-first-run (installer < 30MB)
> - PHP Version: 8.3 only (latest stable)

---

### Phase 1: Project Foundation (Week 1)

**Objective**: Set up the basic Tauri + React project structure

#### Tasks:
1. **Initialize Tauri project**
   - Create new Tauri app with React + TypeScript template
   - Configure Vite for development
   - Set up basic app window configuration

2. **Set up development environment**
   - Configure Rust toolchain for cross-platform builds
   - Set up Node.js dependencies
   - Configure ESLint, Prettier for consistent code style

3. **Create base UI layout**
   - Dashboard shell with simple header
   - Status bar component
   - Basic service card layout (3 cards: Caddy, PHP-FPM, MariaDB)

4. **Define data structures**
   - TypeScript interfaces for service states
   - Rust structs for process management
   - IPC command definitions

**Deliverables**: Working Tauri app with empty dashboard UI

---

### Phase 2: Runtime Download System (Week 2)

**Objective**: Implement first-run binary download system

#### Tasks:
1. **Binary source configuration**
   ```rust
   // src-tauri/src/runtime/downloader.rs
   pub struct RuntimeDownloader {
       base_url: String,  // CDN URL for binaries
       platform: Platform,
   }

   pub struct RuntimeManifest {
       caddy_url: String,
       php_url: String,
       mariadb_url: String,
       phpmyadmin_url: String,
       checksums: HashMap<String, String>,
   }

   impl RuntimeDownloader {
       pub async fn download_all(&self, progress_cb: ProgressCallback) -> Result<()>;
       pub async fn verify_checksums(&self) -> Result<bool>;
   }
   ```

2. **Platform-specific binary URLs**
   - Windows: `caddy-windows-amd64.zip`, `php-8.3-windows.zip`, `mariadb-windows.zip`
   - macOS (Intel): `caddy-darwin-amd64.tar.gz`, `php-8.3-darwin.tar.gz`
   - macOS (Apple Silicon): `caddy-darwin-arm64.tar.gz`, `php-8.3-darwin-arm64.tar.gz`
   - Linux: `caddy-linux-amd64.tar.gz`, `php-8.3-linux.tar.gz`, `mariadb-linux.tar.gz`

3. **Download progress UI**
   ```tsx
   function FirstRunWizard() {
       const [progress, setProgress] = useState({ step: 'downloading', percent: 0 });
       // Show progress bar, download speed, ETA
   }
   ```

4. **Extraction & installation**
   - Download to temp directory
   - Verify checksums
   - Extract to `~/.campp/runtime/`
   - Set executable permissions (Unix)

**Deliverables**: First-run wizard that downloads and installs runtime binaries

---

### Phase 3: Rust Backend - Process Manager (Week 2-3)

**Objective**: Implement core process spawning and control

#### Tasks:
1. **Runtime binary locator**
   ```rust
   // src-tauri/src/runtime/locator.rs
   pub fn locate_runtime_binaries() -> Result<RuntimePaths> {
       let runtime_dir = dirs::data_local_dir()
           .ok_or_else(|| anyhow!("Cannot find data directory"))?
           .join("campp")
           .join("runtime");

       Ok(RuntimePaths {
           caddy: detect_caddy_binary(&runtime_dir)?,
           php_fpm: detect_php_binary(&runtime_dir)?,
           mariadb: detect_mariadb_binary(&runtime_dir)?,
           phpmyadmin: runtime_dir.join("phpmyadmin"),
       })
   }
   ```

2. **Process Manager module**
   ```rust
   // src-tauri/src/process/manager.rs
   pub struct ProcessManager {
       services: HashMap<ServiceType, ServiceProcess>,
   }

   pub struct ServiceProcess {
       name: ServiceType,
       child: Option<Child>,
       state: ServiceState,
       port: u16,
   }

   impl ProcessManager {
       pub fn start(&mut self, service: ServiceType) -> Result<()>;
       pub fn stop(&mut self, service: ServiceType) -> Result<()>;
       pub fn restart(&mut self, service: ServiceType) -> Result<()>;
       pub fn status(&self, service: ServiceType) -> ServiceState;
   }
   ```

3. **IPC Commands**
   - `start_service(service_type)`
   - `stop_service(service_type)`
   - `restart_service(service_type)`
   - `get_service_status()`
   - `get_all_statuses()`

4. **Service definitions**
   ```rust
   pub enum ServiceType {
       Caddy,
       PhpFpm,
       MariaDB,
   }

   pub enum ServiceState {
       Stopped,
       Starting,
       Running,
       Stopping,
       Error(String),
   }
   ```

**Deliverables**: Working process manager that can start/stop services

---

### Phase 4: Configuration Generation (Week 3)

**Objective**: Generate configuration files for all services (simplified for MVP)

#### Tasks:
1. **App Data Directory Setup**
   ```rust
   // Create on first run:
   // - ~/.campp/config/     (generated configs)
   // - ~/.campp/mysql/data/  (MariaDB datadir)
   // - ~/.campp/logs/        (service logs)
   // - ~/.campp/projects/    (user projects)
   ```

2. **Port Detection**
   ```rust
   // src-tauri/src/config/ports.rs
   pub fn find_available_port(preferred: u16) -> u16 {
       // Check if port is in use
       // If conflict, try alternatives
   }
   ```

3. **Config Generator (using mustache or handlebars)**
   ```rust
   // src-tauri/src/config/generator.rs
   pub struct ConfigGenerator {
       caddy_port: u16,
       php_port: u16,
       mysql_port: u16,
       phpmyadmin_path: PathBuf,
       project_root: PathBuf,
   }

   impl ConfigGenerator {
       pub fn generate_caddyfile(&self) -> Result<String>;
       pub fn generate_php_ini(&self) -> Result<String>;
       pub fn generate_phpmyadmin_config(&self) -> Result<String>;
   }
   ```

4. **Generated Configs**:
   - **Caddyfile**: Routing, PHP-FPM proxy, phpMyAdmin location
   - **php.ini**: Extensions, error reporting, timezone
   - **config.inc.php**: phpMyAdmin MariaDB connection

**Deliverables**: Automatic config generation on first run

---

### Phase 5: MariaDB Initialization (Week 3-4)

**Objective**: Set up MariaDB with secure defaults

#### Tasks:
1. **Initial MariaDB Setup**
   ```rust
   // src-tauri/src/database/mariadb.rs
   pub fn initialize_mariadb(data_dir: &Path) -> Result<()> {
       // Run mysqld --initialize
       // Capture generated root password
       // Create phpmyadmin user
       // Save credentials to secure storage
   }
   ```

2. **Secure Credential Storage**
   - Use platform credential manager (Windows Credential Manager, Keychain, Secret Service)
   - Store: root password, phpmyadmin user password
   - Rotate passwords on first-run wizard

3. **Database Management Commands**
   - `create_database(name)`
   - `drop_database(name)`
   - `list_databases()`
   - `get_connection_info()`

**Deliverables**: Working MariaDB that starts with initialized data directory

---

### Phase 6: React UI - Dashboard (Week 4)

**Objective**: Build the simple control panel interface (MVP)

#### Tasks:
1. **Service Cards**
   ```tsx
   interface ServiceCardProps {
       service: ServiceType;
       status: ServiceState;
       port: number;
       onStart: () => void;
       onStop: () => void;
       onRestart: () => void;
   }
   ```

2. **Dashboard Layout**
   - Grid of service cards
   - Overall status indicator
   - Quick action buttons (Open Site, Open phpMyAdmin)

3. **State Management**
   ```typescript
   // src/hooks/useServices.ts (simplified for MVP)
   export function useServices() {
       const [services, setServices] = useState<ServiceMap>({});
       const [loading, setLoading] = useState(false);

       const refresh = useCallback(async () => {
           const statuses = await invoke<ServiceMap>('get_all_statuses');
           setServices(statuses);
       }, []);

       useEffect(() => {
           refresh();
           const interval = setInterval(refresh, 2000);
           return () => clearInterval(interval);
       }, [refresh]);

       const startService = async (service: ServiceType) => {
           setLoading(true);
           try {
               await invoke('start_service', { service });
               await refresh();
           } finally {
               setLoading(false);
           }
       };

       return { services, loading, startService, stopService, restartService };
   }
   ```

4. **Visual Feedback**
   - Status badges (Running/Stopped/Error)
   - Port conflict warnings
   - Simple spinners during operations

5. **Quick Actions**
   - "Open in Browser" button (http://localhost:8080)
   - "Open phpMyAdmin" button
   - "Open Project Folder" button

**Deliverables**: Interactive service control UI (MVP)

---

### Phase 7: Basic Settings (Week 5)

**Objective**: Essential configuration UI (MVP only)

#### Tasks:
1. **Simple Settings Panel**
   - Port configuration (with conflict detection)
   - Project folder selection
   - "Show in menu bar/tray" toggle

2. **Persistent Settings**
   ```rust
   // src-tauri/src/config/settings.rs (simplified)
   #[derive(Serialize, Deserialize, Default)]
   pub struct AppSettings {
       web_port: u16,
       mysql_port: u16,
       project_root: PathBuf,
   }

   pub fn load_settings() -> AppSettings { /* ... */ }
   pub fn save_settings(settings: &AppSettings) -> Result<()> { /* ... */ }
   ```

3. **Port Conflict Detection**
   - Visual indicators when ports are in use
   - Auto-suggest alternatives
   - "Test Configuration" button

**Deliverables**: Basic settings interface

---

## v1.1 Features (Post-MVP)

> These features will be implemented after the MVP is released

### Log Viewer
- Real-time log streaming in the UI
- Rust file watching backend
- React log viewer component with filtering

### Multi-Project Management
- Multiple project support
- Virtual host configuration
- Project creation wizard
- Quick project switching

### Advanced Settings
- Service startup order/delay
- Auto-start on boot
- PHP configuration editor
- MariaDB advanced settings

### Security Enhancements
- Platform credential store integration
- Password rotation UI
- Remote DB access toggle

---

## MVP Release: Phase 8 (Week 6-7)

**Objective**: Cross-platform installers and release

**Objective**: Production-ready security defaults

#### Tasks:
1. **MariaDB Security**
   - Bind to 127.0.0.1 only (configurable)
   - Strong random password generation
   - Dedicated phpmyadmin user (not root)
   - Disable remote access by default

2. **Web Server Security**
   - Hide server headers
   - Rate limiting for phpMyAdmin
   - CSRF/XSS protection headers
   - Secure phpMyAdmin with cookie-based auth

3. **Credential Management**
   - Platform credential store integration
   - Password rotation UI
   - Export/import credentials (encrypted)

**Deliverables**: Security audit passed

#### Tasks:
1. **Lightweight Installers** (~30MB each)
   - No bundled binaries (downloaded on first run)
   - Just the Tauri app + phpMyAdmin bundle

2. **Installer Configuration**
   - **Windows**: NSIS installer with:
     - Desktop shortcut
     - Start menu entry
     - Clean uninstall
   - **macOS**: DMG with:
     - Drag-to-install
     - Code signing (for distribution)
   - **Linux**: AppImage (universal)

3. **Cross-Platform Build Pipeline**
   - GitHub Actions for automated builds
   - Parallel builds for all 3 platforms
   - Automatic release asset generation

4. **First-Run Wizard** (from Phase 2)
   - Welcome screen
   - Download progress with ETA
   - Port configuration (optional)
   - Test all services on completion

**Deliverables**: Installers for Windows/macOS/Linux

---

## Final Testing & Documentation (Week 7-8)

**Objective**: Quality assurance and user documentation

#### Tasks:
1. **Testing**
   - Unit tests for Rust modules
   - Integration tests for IPC commands
   - E2E tests with Playwright
   - Manual testing on all platforms

2. **Documentation**
   - User guide (getting started)
   - Developer documentation
   - Troubleshooting guide
   - API documentation for plugins

3. **Performance**
   - Startup time optimization
   - Memory usage profiling
   - Idle CPU usage

**Deliverables**: Production-ready release

---

## Critical Files & Components

### Rust Backend Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/runtime/downloader.rs` | Binary download & extraction |
| `src-tauri/src/runtime/locator.rs` | Runtime binary path resolution |
| `src-tauri/src/process/manager.rs` | Core process spawn/control logic |
| `src-tauri/src/config/generator.rs` | Config file generation |
| `src-tauri/src/database/mariadb.rs` | MariaDB initialization |
| `src-tauri/src/commands.rs` | Tauri IPC command handlers |
| `src-tauri/build.rs` | Resource bundling script |

### React Frontend Key Files

| File | Purpose |
|------|---------|
| `src/components/Dashboard.tsx` | Main dashboard view |
| `src/components/ServiceCard.tsx` | Individual service display |
| `src/components/FirstRunWizard.tsx` | Download/setup wizard |
| `src/hooks/useServices.ts` | Service state management |
| `src/types/services.ts` | TypeScript type definitions |

### Configuration Templates

| File | Purpose |
|------|---------|
| `templates/Caddyfile.mustache` | Caddy web server config |
| `templates/php.ini.mustache` | PHP 8.3 configuration |
| `templates/config.inc.php.mustache` | phpMyAdmin config |

---

## Runtime Binary Sources

### Official Sources (recommended)

| Component | Windows | macOS (x64) | macOS (ARM64) | Linux |
|-----------|---------|-------------|---------------|-------|
| **Caddy** | caddy-server.com | caddy-server.com | caddy-server.com | caddy-server.com |
| **PHP 8.3** | windows.php.net | homebrew.php.net | homebrew.php.net | php.net |
| **MariaDB** | mariadb.org | mariadb.org | mariadb.org | mariadb.org |

### CDN Structure
```
https://cdn.campp.dev/v1.0.0/
├── manifests/
│   ├── windows-x64.json      # Checksums + URLs
│   ├── darwin-x64.json
│   ├── darwin-arm64.json
│   └── linux-x64.json
├── binaries/
│   ├── windows-x64/
│   │   ├── caddy.zip
│   │   ├── php-8.3.zip
│   │   └── mariadb.zip
│   └── ...
└── phpmyadmin/
    └── phpmyadmin-5.2.1.zip  # Bundled with installer
```

### Fallback Strategy
1. **Primary CDN**: Fastly/CloudFront
2. **Mirror 1**: GitHub Releases
3. **Mirror 2**: SourceForge (legacy)

### Offline Installation (Enterprise)
- Provide "full bundle" installer (~200MB) with all binaries included
- Enable via environment variable or command-line flag

---

## Default Configuration

### Ports (avoid conflicts)
```yaml
Web Server:    8080  # HTTP
PHP-FPM:       9000  # Internal only
MariaDB:       3307  # MySQL-compatible
phpMyAdmin:    8080/phpmyadmin  # Behind web server
```

### Paths (per OS)
```rust
Windows: C:\Users\<user>\.campp\
macOS:   ~/.campp/
Linux:   ~/.campp/
```

### Directory Structure
```
~/.campp/
├── config/
│   ├── Caddyfile
│   ├── php.ini
│   └── phpmyadmin.config.inc.php
├── mysql/
│   └── data/           # MariaDB datadir
├── logs/
│   ├── caddy.log
│   ├── php-fpm.log
│   └── mariadb.log
├── projects/
│   └── localhost/      # Default project
└── runtime/            # Extracted binaries
```

---

## Dependencies

### Rust (Cargo.toml)
```toml
[dependencies]
tauri = "2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
mustache = "0.9"
sysinfo = "0.30"
dirs = "5"
rand = "0.8"
```

### Node.js (package.json)
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "react": "^18",
    "react-dom": "^18"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^18",
    "typescript": "^5",
    "vite": "^5"
  }
}
```

---

## Success Criteria

### MVP (v1.0)
- [ ] All services start/stop/restart reliably
- [ ] No admin permissions required on any platform
- [ ] Port conflicts detected and resolved
- [ ] First-run download wizard completes successfully
- [ ] phpMyAdmin accessible and functional
- [ ] Basic dashboard shows correct service status
- [ ] Installers work on Windows/macOS/Linux
- [ ] Startup time < 5 seconds
- [ ] Memory usage < 200MB idle
- [ ] Installer size < 30MB (binaries downloaded separately)

### v1.1 (Post-MVP)
- [ ] Real-time logs visible in UI
- [ ] Multiple projects can be managed
- [ ] Advanced settings (service startup order, custom PHP config)
- [ ] Security audit passed

---

## Verification Plan

### MVP End-to-End Testing
1. Install CAMPP on clean system (installer < 30MB)
2. Launch application - first-run wizard appears
3. Complete download wizard (binaries download and extract)
4. All services start automatically
5. Open http://localhost:8080 → PHP info page
6. Open http://localhost:8080/phpmyadmin → login works
7. Stop/start individual services - UI updates correctly
8. Change ports in settings - restart and verify new ports work
9. Uninstall - confirm clean removal

### Platform-Specific Testing
- **Windows**: Test on Windows 10/11, no UAC prompt required
- **macOS**: Test on Intel + Apple Silicon, code signing valid
- **Linux**: Test on Ubuntu/Debian/Fedora, AppImage runs

### v1.1 Testing (Post-MVP)
- Multi-project creation and switching
- Log viewer real-time updates
- Advanced settings persistence

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Download failures | Retry logic, resume support, mirror fallback |
| No internet access | Document offline bundle option for enterprise |
| Antivirus flagging | Code signing, whitelist submissions |
| Port conflicts | Smart detection, fallback ports |
| MariaDB init failures | Robust error handling, retry logic |
| Cross-platform path issues | Use `dirs` crate, extensive testing |
| phpMyAdmin updates | Version pinning, update mechanism |
| CDN downtime | Mirror URLs, local caching |
| Memory leaks | Profile testing, proper cleanup |

---

## Next Steps

Once plan is approved:

1. Initialize Tauri project with `npm create tauri-app@latest`
2. Set up project structure according to Phase 1
3. Implement Runtime Download System (Phase 2) - core differentiator
4. Build Process Manager (Phase 3)
5. Progress sequentially through MVP phases

---

## Development Timeline

- **Week 1**: Project Foundation
- **Week 2**: Runtime Download System
- **Week 2-3**: Process Manager
- **Week 3**: Configuration Generation
- **Week 3-4**: MariaDB Initialization
- **Week 4**: Dashboard UI
- **Week 5**: Basic Settings
- **Week 6-7**: MVP Release & Testing
- **Week 7-8**: Final Testing & Documentation

**Total MVP Timeline**: ~7-8 weeks to first release
4. Progress sequentially through each phase
