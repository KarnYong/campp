<?php
// CAMPP welcome page. Safe to edit or replace — CAMPP only creates this file when it is missing.
$projects = array_filter(scandir(__DIR__), function ($d) {
    return $d[0] !== '.' && is_dir(__DIR__ . DIRECTORY_SEPARATOR . $d);
});
?>
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CAMPP</title>
<style>
  /* Colours and shapes follow the campp-web site theme (emerald/teal brand, slate neutrals). */
  :root {
    --bg: #f5faf8; --card: #ffffff; --muted-bg: #f6f8f7; --hover: #ecfdf5;
    --text: #0f172a; --text2: #334155; --muted: #64748b;
    --border: #e2e8f0; --accent: #059669; --accent-text: #047857;
    --hero: linear-gradient(135deg, #a7f3d0, #ccfbf1 50%, #d1fae5);
    --btn: linear-gradient(90deg, #059669, #0d9488); --btn-hover: linear-gradient(90deg, #047857, #0f766e);
    --pill-bg: #ffffff;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #0a0f1a; --card: #101828; --muted-bg: #0e1524; --hover: #172136;
      --text: #f8fafc; --text2: #e2e8f0; --muted: #94a3b8;
      --border: #1e293b; --accent: #34d399; --accent-text: #34d399;
      --hero: linear-gradient(135deg, #0a0f1a, #0e1524 50%, #0a0f1a);
      --pill-bg: #131c2e;
    }
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px 16px;
    background: var(--bg); color: var(--text);
    font: 15px/1.6 "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }
  .card {
    width: 100%; max-width: 600px; background: var(--card); border: 1px solid var(--border);
    border-radius: 24px; overflow: hidden; box-shadow: 0 20px 50px -20px rgba(5, 150, 105, 0.25);
  }
  .hero { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px 24px 32px; text-align: center; background: var(--hero); border-bottom: 1px solid var(--border); }
  .hero img { width: 150px; height: auto; }
  @media (prefers-reduced-motion: no-preference) {
    .hero img { animation: float 4s ease-in-out infinite; }
    @keyframes float { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-8px); } }
  }
  h1 { font-size: 30px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.2; }
  h1 span { background: linear-gradient(90deg, #059669, #0d9488); -webkit-background-clip: text; background-clip: text; color: transparent; }
  @media (prefers-color-scheme: dark) { h1 span { background-image: linear-gradient(90deg, #34d399, #2dd4bf); } }
  .status {
    display: inline-flex; align-items: center; gap: 8px; padding: 4px 12px; border-radius: 999px;
    background: var(--pill-bg); border: 1px solid var(--border); font-size: 13px; font-weight: 600; color: var(--accent-text);
  }
  .status::before { content: ""; width: 8px; height: 8px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.25); }
  .muted { color: var(--muted); }
  .mono { font-family: "Geist Mono", "Cascadia Code", Consolas, ui-monospace, monospace; font-size: 13px; }
  section { padding: 22px 28px; }
  section + section { border-top: 1px solid var(--border); }
  h2 { font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); margin-bottom: 12px; }
  .links { display: flex; flex-wrap: wrap; gap: 10px; }
  a.btn {
    display: inline-flex; align-items: center; justify-content: center; padding: 9px 20px; border-radius: 999px;
    border: 1px solid var(--border); background: var(--card); color: var(--text2); text-decoration: none;
    font-size: 14px; font-weight: 600; transition: transform 0.15s, background-color 0.15s, box-shadow 0.15s;
  }
  a.btn:hover { background: var(--hover); transform: translateY(-1px); }
  a.btn.primary {
    border: 0; color: #ffffff; background: var(--btn); box-shadow: 0 10px 20px -8px rgba(5, 150, 105, 0.45);
  }
  a.btn.primary:hover { background: var(--btn-hover); box-shadow: 0 12px 24px -8px rgba(5, 150, 105, 0.6); }
  ul { list-style: none; border: 1px solid var(--border); border-radius: 16px; overflow: hidden; }
  li + li { border-top: 1px solid var(--border); }
  li a { display: flex; justify-content: space-between; align-items: center; padding: 11px 16px; color: var(--text2); text-decoration: none; font-weight: 500; }
  li a:hover { background: var(--hover); color: var(--accent-text); }
  a:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  footer { padding: 14px 28px; background: var(--muted-bg); border-top: 1px solid var(--border); font-size: 12.5px; }
</style>
</head>
<body>
  <main class="card">
    <div class="hero">
      <img src="data:image/png;base64,{{LOGO}}" alt="CAMPP">
      <h1>Welcome to <span>CAMPP</span></h1>
      <span class="status">Web server is running</span>
      <p class="muted">PHP <span class="mono"><?= htmlspecialchars(PHP_VERSION) ?></span> · <?= htmlspecialchars($_SERVER['SERVER_SOFTWARE'] ?? 'Caddy') ?></p>
    </div>

    <section>
      <h2>Tools</h2>
      <div class="links">
        <a class="btn primary" href="info.php">PHP info</a>
        <a class="btn" href="/phpmyadmin/">phpMyAdmin</a>
        <a class="btn" href="/adminer/">Adminer</a>
      </div>
    </section>

    <section>
      <h2>Your projects</h2>
      <?php if ($projects): ?>
        <ul>
          <?php foreach ($projects as $p): ?>
            <li><a href="<?= rawurlencode($p) ?>/"><span><?= htmlspecialchars($p) ?></span><span class="muted">→</span></a></li>
          <?php endforeach; ?>
        </ul>
      <?php else: ?>
        <p class="muted">No projects yet. Create a folder in <span class="mono"><?= htmlspecialchars(__DIR__) ?></span> to get started.</p>
      <?php endif; ?>
    </section>

    <footer class="muted">Serving <span class="mono"><?= htmlspecialchars(__DIR__) ?></span></footer>
  </main>
</body>
</html>
