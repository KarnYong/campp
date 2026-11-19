// Screenshots for the campp-web user manual (light + dark, 2x), from the real React UI.
//
//   npx vite --port 5173 --strictPort          # in another terminal (campp/)
//   npm i --no-save playwright                 # once; uses the installed Edge/Chrome
//   node scripts/manual-screenshots.mjs <outDir> [http://localhost:5173]
//
// The Tauri backend is replaced by a mock of window.__TAURI_INTERNALS__.invoke that
// returns realistic data; package versions come from src-tauri/runtime-config.json, so
// shots always show the versions that ship. Writes <name>-<theme>.png into <outDir>.
import { chromium } from "playwright";
import { readFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = process.argv[2];
const baseUrl = process.argv[3] || "http://localhost:5173";
if (!outDir) {
  console.error("usage: node scripts/manual-screenshots.mjs <outDir> [baseUrl]");
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const appVersion = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
const cfg = JSON.parse(readFileSync(join(root, "src-tauri/runtime-config.json"), "utf8")).binaries;
const multi = (b) =>
  (b?.versions || []).map((v) => ({
    id: v.id, version: v.version, display_name: v.display_name,
    windowsX64: v.urls?.windowsX64 || "", windowsArm64: v.urls?.windowsArm64 || "",
    linuxX64: v.urls?.linuxX64 || "", linuxArm64: v.urls?.linuxArm64 || "",
    macOSX64: v.urls?.macOSX64 || "", macOSArm64: v.urls?.macOSArm64 || "",
    eol: !!v.eol, lts: !!v.lts, recommended: !!v.selected,
  }));
const single = (b) =>
  (b?.versions || []).map((v) => ({
    id: v.id, version: v.version, display_name: v.display_name, url: v.url,
    eol: !!v.eol, lts: !!v.lts, recommended: !!v.selected,
  }));
const packages = {
  php: multi(cfg.php), mysql: multi(cfg.mysql), mariadb: multi(cfg.mariadb),
  postgresql: multi(cfg.postgresql), ngrok: multi(cfg.ngrok),
  phpmyadmin: single(cfg.phpmyadmin), adminer: single(cfg.adminer), composer: single(cfg.composer),
};
const selected = (list) => (list.find((p) => p.recommended) || list[0]);
const ver = (key) => selected(packages[key])?.version;
const selection = {
  php: selected(packages.php).id, mysql: selected(packages.mysql).id, mariadb: selected(packages.mariadb).id,
  phpmyadmin: selected(packages.phpmyadmin).id, postgresql: selected(packages.postgresql).id,
  adminer: selected(packages.adminer).id, ngrok: selected(packages.ngrok).id,
  composer: selected(packages.composer).id, db_engine: "mysql",
};
const caddy = cfg.caddy.versions.find((v) => v.selected).version;
const installed = {
  caddy, php: ver("php"), mysql: ver("mysql"), phpmyadmin: ver("phpmyadmin"),
  postgresql: ver("postgresql"), adminer: ver("adminer"), composer: ver("composer"),
};
const base = "C:\\CAMPP";
const now = Math.floor(Date.now() / 1000);

function mockData(theme, firstRun, stopped = false) {
  const svc = (type, state, port) => ({ service_type: type, state: stopped ? "stopped" : state, port });
  return {
    check_runtime_installed: !firstRun,
    get_settings: {
      web_port: 8080, php_port: 9000, mysql_port: 3307, postgres_port: 5433,
      project_root: `${base}\\projects`, mysql_root_password: "campp-root", postgres_root_password: "campp-pg",
      listen_all_interfaces: false, ngrok_authtoken: "", theme, package_selection: selection,
    },
    get_package_selection: selection,
    get_available_packages_cmd: packages,
    get_installed_versions: installed,
    check_existing_components: firstRun ? {} : installed,
    get_caddy_version: caddy,
    get_all_statuses: {
      caddy: svc("caddy", "running", 8080),
      "php-fpm": svc("php-fpm", "running", 9000),
      mysql: svc("mysql", "running", 3307),
      postgresql: svc("postgresql", "stopped", 5433),
      ngrok: svc("ngrok", "stopped", 4040),
    },
    get_ngrok_public_url: null,
    get_install_dir: base,
    get_runtime_dir: `${base}\\runtime`,
    get_download_dir: `${base}\\downloads`,
    get_logs_dir: `${base}\\logs`,
    get_debug_info: {
      version: appVersion, exePath: `${base}\\CAMPP.exe`, resourceDir: base,
      runtimeDir: `${base}\\runtime`, configLoaded: true, os: "windows", arch: "x86_64",
    },
    check_ports: {
      web: { port: 8080, available: false }, php: { port: 9000, available: false },
      mysql: { port: 3307, available: false }, postgres: { port: 5433, available: true },
    },
    list_backups: [
      { name: `campp-backup-${now - 3600}.zip`, size: 48_234_112, created_unix: now - 3600 },
      { name: `campp-backup-${now - 86400 * 3}-pre-restore.zip`, size: 46_901_248, created_unix: now - 86400 * 3 },
    ],
    check_system_dependencies: { all_satisfied: true, dependencies: [], platform_notes: "" },
    "plugin:app|version": appVersion,
    "plugin:event|listen": 1,
  };
}

// Runs in the page before the app's scripts: stands in for the Tauri IPC bridge.
function installMock(data) {
  let cb = 0;
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback: (fn) => { const id = ++cb; window[`_${id}`] = fn; return id; },
    unregisterCallback: () => {},
    convertFileSrc: (p) => p,
    invoke: async (cmd) => (cmd in data ? structuredClone(data[cmd]) : null),
  };
}

const shots = [];
const browser = await chromium.launch({ channel: process.env.PW_CHANNEL || "msedge" });

async function capture(theme, firstRun, name, steps, { stopped = false } = {}) {
  const page = await browser.newPage({ viewport: { width: 1024, height: 620 }, deviceScaleFactor: 2 });
  await page.addInitScript(installMock, mockData(theme, firstRun, stopped));
  await page.emulateMedia({ colorScheme: theme });
  await page.goto(baseUrl);
  await page.waitForLoadState("networkidle");
  if (steps) await steps(page);
  await page.waitForTimeout(400);
  // Grow the viewport to fit the page's scrollable pane, so tall pages are captured whole.
  const extra = await page.evaluate(() =>
    Math.max(0, ...[...document.querySelectorAll("*")].map((el) => el.scrollHeight - el.clientHeight)),
  );
  if (extra > 0) await page.setViewportSize({ width: 1024, height: 620 + extra });
  await page.waitForTimeout(200);
  const file = join(outDir, `${name}-${theme}.png`);
  await page.screenshot({ path: file });
  shots.push(file);
  await page.close();
}

const nav = (label) => async (page) => {
  await page.getByRole("button", { name: label, exact: true }).click();
  await page.waitForTimeout(300);
};

for (const theme of ["light", "dark"]) {
  await capture(theme, false, "services");
  // Components can only install/uninstall with every service stopped
  await capture(theme, false, "components", nav("Components"), { stopped: true });
  await capture(theme, false, "reinstall-dialog", async (page) => {
    await nav("Components")(page);
    await page.getByRole("button", { name: "Reinstall" }).nth(1).click(); // PHP row
    await page.waitForTimeout(300);
  }, { stopped: true });
  await capture(theme, false, "settings", nav("Settings"), { stopped: true }); // editable state
  await capture(theme, false, "backup", nav("Backup"));
  await capture(theme, false, "debug", nav("Debug"));
  await capture(theme, true, "choose-components", async (page) => {
    await page.getByRole("button", { name: /get started/i }).click();
    await page.waitForTimeout(300);
  });
}

await browser.close();
console.log(shots.join("\n"));
