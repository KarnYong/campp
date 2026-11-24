#!/usr/bin/env node
// Laravel end-to-end on CAMPP built from source (spec 006 US5, contracts/e2e-laravel.md).
//
// Builds a small "tasks" app with CAMPP's own Composer, PHP, MySQL/MariaDB and Caddy,
// running as an isolated test instance: CAMPP_HOME=<campp>/.e2e/home, ports
// 18080/19000/13307. It never reads, writes, stops or restarts an installed CAMPP:
// the installed settings file is only stat()ed (mtime+size), never opened.
//
//   npm run e2e:laravel -- [--keep] [--web 18080] [--php 19000] [--mysql 13307] [--bin <CAMPP binary>]
//
// Exit: 0 all pass · 1 a step failed · 2 isolation guard failed (nothing started).
// Needs Node 22.18+ (imports src/lib/envBlock.ts via native type stripping) and internet.

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildEnvBlock } from "../src/lib/envBlock.ts";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const HOME = path.join(ROOT, ".e2e", "home");
const WIN = process.platform === "win32";
// Special characters on purpose: proves the .env escaping end to end (research R7).
const PASSWORD = "e2e pa\"ss$#\\'w0rd";
const FR004 = ["intl", "bcmath", "mbstring", "openssl", "pdo_mysql", "curl", "fileinfo", "zip",
  "pdo_sqlite", "sqlite3", "tokenizer", "dom", "xml", "ctype", "gd", "sodium"];

// ---------- args ----------
const argv = process.argv.slice(2);
const flag = (name, dflt) => {
  const i = argv.indexOf(`--${name}`);
  return i >= 0 ? argv[i + 1] : dflt;
};
const KEEP = argv.includes("--keep");
const BIN = path.resolve(flag("bin", path.join(ROOT, "src-tauri", "target", "debug", WIN ? "CAMPP.exe" : "CAMPP")));
const wanted = { web: Number(flag("web", 18080)), php: Number(flag("php", 19000)), mysql: Number(flag("mysql", 13307)) };

// ---------- helpers ----------
const results = [];
let failed = false;
let needsFeature = false;
let runFailed = false;
const log = (s) => process.stdout.write(s + "\n");

async function step(name, fn) {
  if (failed) {
    results.push({ name, status: "SKIP" });
    log(`SKIP ${name}`);
    return;
  }
  const t = Date.now();
  try {
    await fn();
    results.push({ name, status: "PASS" });
    log(`PASS ${name} ${Date.now() - t}ms`);
  } catch (e) {
    // A missing prerequisite feature (spec 005) fails the run but doesn't stop the
    // independent steps after it.
    if (String(e?.message).startsWith("(needs spec 005)")) needsFeature = true;
    else failed = true;
    results.push({ name, status: "FAIL" });
    log(`FAIL ${name} ${Date.now() - t}ms\n  ${String(e?.message ?? e).split("\n").join("\n  ")}`);
  }
}

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

const tail = (s, n = 4096) => (s.length > n ? "…" + s.slice(-n) : s);

/** Run a CAMPP launcher (`php`/`composer` in <bin>) or any command; throws on non-zero. */
let LAUNCHERS = "";
function run(cmd, args, { cwd, env = {}, allowFail = false, timeout = 600_000 } = {}) {
  const launcher = ["php", "composer"].includes(cmd);
  let file = launcher ? path.join(LAUNCHERS, WIN ? `${cmd}.cmd` : cmd) : cmd;
  let shell = false;
  if (WIN && launcher) {
    // .cmd files need a shell; quote anything with spaces or cmd metacharacters.
    shell = true;
    const q = (a) => (/[\s"&|<>^%()]/.test(a) ? `"${a.replace(/"/g, '""')}"` : a);
    file = q(file);
    args = args.map(q);
  }
  const r = spawnSync(file, args, {
    cwd,
    shell,
    encoding: "utf8",
    timeout,
    env: { ...process.env, ...COMPOSER_ENV, ...env },
  });
  const out = `${r.stdout ?? ""}${r.stderr ?? ""}`;
  if (!allowFail && r.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} → exit ${r.status ?? r.error}\n${tail(out, 2000)}`);
  }
  return { status: r.status, out };
}

/** Free = we can listen on 127.0.0.1:port right now. */
const isFree = (port) =>
  new Promise((resolve) => {
    const s = net.createServer();
    s.once("error", () => resolve(false));
    s.listen(port, "127.0.0.1", () => s.close(() => resolve(true)));
  });

const forbidden = (p) => p === 2019 || (p >= 8000 && p <= 8100) || (p >= 9000 && p <= 9100) || (p >= 3300 && p <= 3400);

/** HTTP request to 127.0.0.1 with an optional Host header (Node can't rely on *.localhost DNS). */
function request(port, urlPath, { method = "GET", host, headers = {}, body } = {}) {
  return new Promise((resolve, reject) => {
    const req = http.request(
      { host: "127.0.0.1", port, path: urlPath, method, headers: { ...(host ? { Host: host } : {}), ...headers }, timeout: 15_000 },
      (res) => {
        let data = "";
        res.setEncoding("utf8");
        res.on("data", (c) => (data += c));
        res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body: data }));
      },
    );
    req.on("timeout", () => req.destroy(new Error(`timeout ${method} ${urlPath}`)));
    req.on("error", reject);
    if (body) req.write(body);
    req.end();
  });
}

const reachable = async (port) => {
  try {
    return (await request(port, "/")).status < 500;
  } catch {
    return false;
  }
};

function installedSettingsPath() {
  if (WIN) return path.join(process.env.LOCALAPPDATA ?? "", "campp", "config", "settings.json");
  if (process.platform === "darwin") return path.join(os.homedir(), "Library", "Application Support", "campp", "config", "settings.json");
  const data = process.env.XDG_DATA_HOME || path.join(os.homedir(), ".local", "share");
  return path.join(data, "campp", "config", "settings.json");
}
const statOnly = (f) => {
  try {
    const s = fs.statSync(f); // metadata only: the file is never opened
    return { mtimeMs: s.mtimeMs, size: s.size };
  } catch {
    return null;
  }
};

const write = (file, text) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
};

// Composer cache/config inside the test home: nothing outside .e2e is written.
const COMPOSER_ENV = { COMPOSER_HOME: path.join(HOME, ".composer"), COMPOSER_CACHE_DIR: path.join(HOME, ".composer", "cache"), COMPOSER_NO_INTERACTION: "1" };

// ---------- state ----------
const ports = { ...wanted };
let serve = null;
let serveOut = "";
let snapshot = null;
let installedWebAtStart = false;
const installedWebMisses = [];
let poll = null;
const APP = path.join(HOME, "projects", "tasks");

// ---------- the Laravel "tasks" app (written from templates) ----------
const files = {
  "database/migrations/2026_01_01_000000_create_tasks_table.php": `<?php

use Illuminate\\Database\\Migrations\\Migration;
use Illuminate\\Database\\Schema\\Blueprint;
use Illuminate\\Support\\Facades\\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('tasks', function (Blueprint $table) {
            $table->id();
            $table->string('title', 100);
            $table->boolean('done')->default(false);
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('tasks');
    }
};
`,
  "app/Models/Task.php": `<?php

namespace App\\Models;

use Illuminate\\Database\\Eloquent\\Model;

class Task extends Model
{
    protected $fillable = ['title', 'done'];

    protected $casts = ['done' => 'boolean'];
}
`,
  "app/Http/Controllers/TaskController.php": `<?php

namespace App\\Http\\Controllers;

use App\\Models\\Task;
use Illuminate\\Http\\Request;

class TaskController extends Controller
{
    public function index()
    {
        return view('tasks.index', ['tasks' => Task::orderBy('id')->get()]);
    }

    public function create()
    {
        return view('tasks.create');
    }

    public function store(Request $request)
    {
        $data = $request->validate(['title' => 'required|string|max:100']);
        Task::create($data);

        return redirect()->route('tasks.index');
    }

    public function show(Task $task)
    {
        return view('tasks.show', ['task' => $task]);
    }

    public function update(Task $task)
    {
        $task->update(['done' => true]);

        return redirect()->route('tasks.show', $task);
    }

    public function destroy(Task $task)
    {
        $task->delete();

        return redirect()->route('tasks.index');
    }
}
`,
  "resources/views/tasks/index.blade.php": `<!doctype html>
<title>Tasks</title>
<h1>Tasks</h1>
<p>Total: {{ \\Illuminate\\Support\\Number::format($tasks->count()) }}</p>
<ul>
@foreach ($tasks as $task)
    <li><a href="{{ route('tasks.show', $task) }}">{{ $task->title }}</a>@if ($task->done) (done)@endif</li>
@endforeach
</ul>
<a href="{{ route('tasks.create') }}">New task</a>
`,
  "resources/views/tasks/create.blade.php": `<!doctype html>
<title>New task</title>
<form method="POST" action="{{ route('tasks.store') }}">
    @csrf
    <input name="title" value="{{ old('title') }}">
    @foreach ($errors->all() as $error)
        <p class="error">{{ $error }}</p>
    @endforeach
    <button>Add</button>
</form>
`,
  "resources/views/tasks/show.blade.php": `<!doctype html>
<title>{{ $task->title }}</title>
<h1>{{ $task->title }}</h1>
<p>Status: {{ $task->done ? 'done' : 'open' }}</p>
<form method="POST" action="{{ route('tasks.update', $task) }}">@csrf @method('PATCH')<button>Done</button></form>
<form method="POST" action="{{ route('tasks.destroy', $task) }}">@csrf @method('DELETE')<button>Delete</button></form>
`,
  "database/seeders/TaskSeeder.php": `<?php

namespace Database\\Seeders;

use App\\Models\\Task;
use Illuminate\\Database\\Seeder;

class TaskSeeder extends Seeder
{
    public function run(): void
    {
        foreach (['Seed task one', 'Seed task two', 'Seed task three'] as $title) {
            Task::create(['title' => $title]);
        }
    }
}
`,
  "database/seeders/DatabaseSeeder.php": `<?php

namespace Database\\Seeders;

use Illuminate\\Database\\Seeder;

class DatabaseSeeder extends Seeder
{
    public function run(): void
    {
        $this->call(TaskSeeder::class);
    }
}
`,
  "tests/Feature/TaskTest.php": `<?php

namespace Tests\\Feature;

use Illuminate\\Foundation\\Testing\\RefreshDatabase;
use Tests\\TestCase;

class TaskTest extends TestCase
{
    use RefreshDatabase;

    public function test_a_task_can_be_created_and_listed(): void
    {
        $this->post('/tasks', ['title' => 'Written by a test'])->assertRedirect('/tasks');
        $this->get('/tasks')->assertOk()->assertSee('Written by a test');
    }

    public function test_title_is_required(): void
    {
        $this->post('/tasks', ['title' => ''])->assertSessionHasErrors('title');
    }
}
`,
};

// PDO helper run with CAMPP's PHP (password via env var, so no shell quoting).
const DB_PHP = `<?php
$pdo = new PDO('mysql:host=127.0.0.1;port=' . getenv('E2E_PORT'), 'root', getenv('E2E_PW'));
$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
if ($argv[1] === 'create') {
    foreach (['tasks', 'tasks_test'] as $db) { $pdo->exec("CREATE DATABASE IF NOT EXISTS \`$db\`"); }
    echo "ok";
} elseif ($argv[1] === 'count') {
    echo $pdo->query('SELECT COUNT(*) FROM tasks.tasks')->fetchColumn();
}
`;
const dbPhp = (mode) =>
  run("php", [path.join(HOME, "e2e-db.php"), mode], { cwd: HOME, env: { E2E_PORT: String(ports.mysql), E2E_PW: PASSWORD } }).out.trim();

// ---------- steps ----------
async function guard() {
  const home = process.env.CAMPP_HOME ? path.resolve(process.env.CAMPP_HOME) : HOME;
  assert(home === HOME, `CAMPP_HOME must be ${HOME} (got ${home})`);
  assert(fs.existsSync(BIN), `CAMPP binary not found: ${BIN} (run cargo build in src-tauri first)`);
  for (const key of ["web", "php", "mysql"]) {
    let p = ports[key];
    assert(!forbidden(p), `test ${key} port ${p} is too close to an everyday CAMPP's ports`);
    const taken = () => Object.entries(ports).some(([k, v]) => k !== key && v === p);
    let shifted = 0;
    while (forbidden(p) || taken() || !(await isFree(p))) {
      assert(++shifted <= 100, `no free ${key} port near ${ports[key]}`);
      p++;
    }
    if (p !== ports[key]) log(`  ${key} port ${ports[key]} not free, using ${p}`);
    ports[key] = p;
  }
  snapshot = statOnly(installedSettingsPath());
  installedWebAtStart = await reachable(8080);
  log(`  test ports web ${ports.web} php ${ports.php} mysql ${ports.mysql}; installed CAMPP on 8080: ${installedWebAtStart ? "up" : "not running"}`);
}

async function prep() {
  write(
    path.join(HOME, "config", "settings.json"),
    JSON.stringify(
      { web_port: ports.web, php_port: ports.php, mysql_port: ports.mysql, postgres_port: 15433,
        project_root: path.join(HOME, "projects"), mysql_root_password: PASSWORD },
      null, 2,
    ),
  );
  const r = spawnSync(BIN, ["--prepare-runtime", "--include", "composer"], {
    cwd: path.join(ROOT, "src-tauri"), encoding: "utf8", timeout: 1_200_000, env: { ...process.env, CAMPP_HOME: HOME },
  });
  assert(r.status === 0, `--prepare-runtime exit ${r.status}\n${tail(`${r.stdout}${r.stderr}`, 2000)}`);
}

async function startServe() {
  serve = spawn(BIN, ["--serve"], {
    cwd: path.join(ROOT, "src-tauri"), env: { ...process.env, CAMPP_HOME: HOME }, stdio: ["pipe", "pipe", "pipe"],
  });
  serve.stderr.on("data", () => {}); // tracing noise
  const ready = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`no CAMPP_READY within 120s\n${tail(serveOut, 2000)}`)), 120_000);
    serve.stdout.setEncoding("utf8");
    serve.stdout.on("data", (chunk) => {
      serveOut += chunk;
      const line = serveOut.split(/\r?\n/).find((l) => l.startsWith("CAMPP_READY ") || l.startsWith("CAMPP_ERROR "));
      if (line) {
        clearTimeout(timer);
        line.startsWith("CAMPP_READY ") ? resolve(JSON.parse(line.slice(12))) : reject(new Error(line));
      }
    });
    serve.on("exit", (code) => reject(new Error(`--serve exited ${code}\n${tail(serveOut, 2000)}`)));
  });
  LAUNCHERS = ready.bin;
  assert(ready.web === ports.web && ready.mysql === ports.mysql, `READY ports ${JSON.stringify(ready)} differ from settings`);
  if (installedWebAtStart) {
    poll = setInterval(async () => {
      if (!(await reachable(8080))) installedWebMisses.push(new Date().toISOString());
    }, 10_000);
  }
}

async function step1() {
  fs.rmSync(APP, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(APP), { recursive: true });
  const r = run("composer", ["create-project", "laravel/laravel", "tasks", "--no-interaction", "--prefer-dist"], {
    cwd: path.dirname(APP), allowFail: true, timeout: 900_000,
  });
  if (r.status !== 0) {
    const network = /could not resolve host|curl error|failed to open stream|connection timed out|network/i.test(r.out);
    throw new Error(`${network ? "(network needed) " : ""}composer create-project exit ${r.status}\n${tail(r.out, 2000)}`);
  }
  const v = run("php", ["artisan", "--version"], { cwd: APP }).out;
  assert(/Laravel Framework/.test(v), `artisan --version: ${v}`);
  log(`  ${v.trim()}`);
}

async function step2() {
  const envFile = path.join(APP, ".env");
  const text = fs.readFileSync(envFile, "utf8");
  const eol = text.includes("\r\n") ? "\r\n" : "\n"; // keep the file's line endings (Constitution III)
  const block = buildEnvBlock({ port: ports.mysql, password: PASSWORD }).replace("DB_DATABASE=laravel", "DB_DATABASE=tasks");
  const lines = text.split(/\r?\n/);
  const first = lines.findIndex((l) => /^#?\s*DB_\w+=/.test(l));
  const kept = lines.filter((l) => !/^#?\s*DB_\w+=/.test(l));
  kept.splice(first < 0 ? kept.length : first, 0, ...block.trimEnd().split("\n"));
  fs.writeFileSync(envFile, kept.join(eol));
  write(path.join(HOME, "e2e-db.php"), DB_PHP);
  assert(dbPhp("create") === "ok", "could not create databases tasks/tasks_test");
  // Connecting with the copied block proves the password escaping (spec US4 scenario 2).
  run("php", ["artisan", "config:clear"], { cwd: APP });
  run("php", ["artisan", "migrate", "--force"], { cwd: APP });
}

async function step3() {
  for (const f of ["database/migrations/2026_01_01_000000_create_tasks_table.php", "app/Models/Task.php"]) write(path.join(APP, f), files[f]);
  run("php", ["artisan", "migrate", "--force"], { cwd: APP });
}

async function step4() {
  for (const f of ["app/Http/Controllers/TaskController.php", "resources/views/tasks/index.blade.php",
    "resources/views/tasks/create.blade.php", "resources/views/tasks/show.blade.php"]) write(path.join(APP, f), files[f]);
  const routes = path.join(APP, "routes", "web.php");
  let web = fs.readFileSync(routes, "utf8");
  if (!web.includes("TaskController")) {
    web = web.replace("<?php", "<?php\n\nuse App\\Http\\Controllers\\TaskController;");
    web += "\nRoute::resource('tasks', TaskController::class)->except(['edit']);\n";
    fs.writeFileSync(routes, web);
  }
  const list = JSON.parse(run("php", ["artisan", "route:list", "--path=tasks", "--json"], { cwd: APP }).out);
  assert(list.length === 6, `expected 6 task routes, got ${list.length}`);
}

async function step5() {
  for (const f of ["database/seeders/TaskSeeder.php", "database/seeders/DatabaseSeeder.php"]) write(path.join(APP, f), files[f]);
  run("php", ["artisan", "migrate:fresh", "--seed", "--force"], { cwd: APP });
  const n = dbPhp("count");
  assert(n === "3", `expected 3 seeded tasks, got ${n}`);
}

async function step6() {
  run("php", ["artisan", "db:show"], { cwd: APP }); // needs intl (Number helpers)
  const mods = run("php", ["-m"], { cwd: APP }).out.toLowerCase().split(/\r?\n/).map((s) => s.trim());
  // pdo_sqlite is checked by capability: the Linux/macOS static PHP gets its PDO
  // SQLite driver from Swoole (--enable-swoole-sqlite), so `php -m` has no
  // pdo_sqlite line although PDO::getAvailableDrivers() includes sqlite.
  write(path.join(HOME, "e2e-drivers.php"), "<?php echo implode(',', PDO::getAvailableDrivers());");
  const drivers = run("php", [path.join(HOME, "e2e-drivers.php")], { cwd: HOME }).out.trim().split(",");
  const missing = FR004.filter((e) => (e === "pdo_sqlite" ? !drivers.includes("sqlite") : !mods.includes(e)));
  assert(missing.length === 0, `PHP extensions missing: ${missing.join(", ")} (PDO drivers: ${drivers.join(", ")})`);
}

async function step7() {
  write(path.join(APP, "tests/Feature/TaskTest.php"), files["tests/Feature/TaskTest.php"]);
  const xmlFile = path.join(APP, "phpunit.xml");
  let xml = fs.readFileSync(xmlFile, "utf8");
  xml = xml.replace(/^\s*(<!--\s*)?<env name="DB_(CONNECTION|DATABASE)"[^>]*\/>(\s*-->)?\s*\r?\n/gm, "");
  xml = xml.replace("</php>", '    <env name="DB_DATABASE" value="tasks_test"/>\n    </php>');
  fs.writeFileSync(xmlFile, xml);
  run("php", ["artisan", "config:clear"], { cwd: APP });
  run("php", ["artisan", "test"], { cwd: APP });
}

async function step8() {
  const host = "tasks.localhost";
  const jar = new Map();
  const cookie = () => [...jar].map(([k, v]) => `${k}=${v}`).join("; ");
  const go = async (p, opts = {}) => {
    const res = await request(ports.web, p, { host, ...opts, headers: { Cookie: cookie(), ...(opts.headers ?? {}) } });
    for (const c of [res.headers["set-cookie"] ?? []].flat()) {
      const [kv] = c.split(";");
      const i = kv.indexOf("=");
      jar.set(kv.slice(0, i), kv.slice(i + 1));
    }
    return res;
  };
  const form = (fields) => ({
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams(fields).toString(),
  });
  const token = (html) => html.match(/name="_token" value="([^"]+)"/)?.[1];

  const list = await go("/tasks");
  if (list.status === 404 && !/laravel/i.test(list.body)) {
    throw new Error(`(needs spec 005) http://${host}:${ports.web}/tasks is not served by the Laravel app: per-project sites aren't built yet`);
  }
  assert(list.status === 200 && list.body.includes("Seed task three"), `GET /tasks → ${list.status}`);

  const createPage = await go("/tasks/create");
  const t = token(createPage.body);
  assert(t, "no CSRF token on /tasks/create");
  const created = await go("/tasks", form({ _token: t, title: "Added over HTTP" }));
  assert(created.status === 302, `POST /tasks → ${created.status}`);
  assert((await go("/tasks")).body.includes("Added over HTTP"), "new task not listed");

  const bad = await go("/tasks", form({ _token: token((await go("/tasks/create")).body), title: "" }));
  assert(bad.status === 302, `empty POST → ${bad.status}`);
  assert(/title field is required/i.test((await go("/tasks/create")).body), "validation error not shown");

  const id = (await go("/tasks")).body.match(/href="[^"]*\/tasks\/(\d+)">Added over HTTP/)?.[1];
  assert(id, "can't find the new task's link");
  const show = await go(`/tasks/${id}`);
  assert(show.status === 200, `GET /tasks/${id} → ${show.status}`);
  const done = await go(`/tasks/${id}`, form({ _token: token(show.body), _method: "PATCH" }));
  assert(done.status === 302 && (await go(`/tasks/${id}`)).body.includes("Status: done"), "PATCH did not mark done");
  const del = await go(`/tasks/${id}`, form({ _token: token((await go(`/tasks/${id}`)).body), _method: "DELETE" }));
  assert(del.status === 302 && (await go(`/tasks/${id}`)).status === 404, "DELETE did not remove the task");

  const missing = await go("/nope");
  assert(missing.status === 404 && missing.body !== "404 Not Found", "unknown route should be Laravel's 404 page");
  assert((await go("/favicon.ico")).status === 200, "static file from public/ not served");
  assert((await go("/.env")).status === 404, "/.env must not be served");
}

async function step9() {
  let port = 18100;
  while (forbidden(port) || !(await isFree(port))) port++;
  const cmd = WIN ? path.join(LAUNCHERS, "php.cmd") : path.join(LAUNCHERS, "php");
  const child = spawn(cmd, ["artisan", "serve", `--port=${port}`], { cwd: APP, shell: WIN, stdio: "ignore" });
  try {
    let status = 0;
    for (let i = 0; i < 40 && status !== 200; i++) {
      await new Promise((r) => setTimeout(r, 500));
      status = await request(port, "/tasks").then((r) => r.status, () => 0);
    }
    assert(status === 200, `php artisan serve --port=${port}: GET /tasks → ${status}`);
  } finally {
    killTree(child.pid);
  }
}

function killTree(pid) {
  if (!pid) return;
  if (WIN) spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore" });
  else {
    try {
      process.kill(pid, "SIGTERM");
    } catch {}
  }
}

async function teardown() {
  if (poll) clearInterval(poll);
  if (serve && serve.exitCode === null) {
    serve.stdin.end(); // --serve stops everything it started when stdin closes
    const stopped = await new Promise((resolve) => {
      const timer = setTimeout(() => resolve(false), 30_000);
      serve.on("exit", () => {
        clearTimeout(timer);
        resolve(true);
      });
    });
    if (!stopped) {
      log("  --serve did not stop in 30s; killing only its process tree");
      killTree(serve.pid);
    }
  }
  assert(!serve || serveOut.includes("CAMPP_STOPPED") || serve.exitCode !== 0, "no CAMPP_STOPPED");
  for (let i = 0; i < 20; i++) {
    const busy = [];
    for (const p of Object.values(ports)) if (!(await isFree(p))) busy.push(p);
    if (busy.length === 0) break;
    assert(i < 19, `test ports still busy after teardown: ${busy.join(", ")}`);
    await new Promise((r) => setTimeout(r, 500));
  }
  if (!runFailed && !KEEP) {
    fs.rmSync(APP, { recursive: true, force: true });
    for (const d of ["mysql", "mariadb"]) fs.rmSync(path.join(HOME, d), { recursive: true, force: true });
  }
}

async function untouched() {
  const now = statOnly(installedSettingsPath());
  assert(JSON.stringify(now) === JSON.stringify(snapshot), `installed CAMPP settings changed: ${JSON.stringify(snapshot)} → ${JSON.stringify(now)}`);
  if (installedWebAtStart) {
    assert(installedWebMisses.length === 0, `installed CAMPP stopped answering at ${installedWebMisses.join(", ")}`);
    assert(await reachable(8080), "installed CAMPP no longer answers on 8080");
  }
}

// ---------- main ----------
log(`CAMPP Laravel e2e · CAMPP_HOME=${HOME}`);
const t0 = Date.now();
try {
  await guard();
} catch (e) {
  log(`FAIL guard\n  ${e.message}`);
  process.exit(2);
}
log("PASS guard");
try {
  await step("prep", prep);
  await step("serve", startServe);
  await step("1 create project", step1);
  await step("2 .env + database", step2);
  await step("3 migration + model", step3);
  await step("4 routes, controller, views", step4);
  await step("5 seeder", step5);
  await step("6 intl, db:show, extensions", step6);
  await step("7 php artisan test", step7);
  await step("8 HTTP through CAMPP's web server", step8);
  await step("9 php artisan serve", step9);
} finally {
  // Teardown and the isolation check always run, whatever failed before.
  const stepsFailed = failed;
  runFailed = stepsFailed || needsFeature; // keep the app + data of a failed run for debugging
  failed = false;
  await step("teardown", teardown);
  const teardownFailed = failed;
  failed = false;
  await step("untouched", untouched);
  failed = failed || teardownFailed || stepsFailed || needsFeature;
  if (failed) {
    const logs = path.join(HOME, "logs");
    for (const f of fs.existsSync(logs) ? fs.readdirSync(logs) : []) {
      const full = path.join(logs, f);
      if (fs.statSync(full).isFile()) log(`--- ${f}\n${tail(fs.readFileSync(full, "utf8"))}`);
    }
  }
  log(`${failed ? "FAILED" : "ALL PASSED"} in ${Math.round((Date.now() - t0) / 1000)}s`);
  process.exit(failed ? 1 : 0);
}
