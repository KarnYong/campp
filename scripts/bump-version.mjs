#!/usr/bin/env node
// Bump the CAMPP version everywhere it is recorded, in one step:
//   npm run bump -- 0.4.0
// Edits only the version text in place (formatting and CRLF/LF survive), checks every
// file before writing any, and never runs git or creates tags.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const VERSION_RE = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

export function parseVersion(s) {
  if (typeof s !== "string" || !VERSION_RE.test(s)) {
    throw new Error(`invalid version "${s}" (expected MAJOR.MINOR.PATCH[-pre])`);
  }
  const [core, pre] = s.split(/-(.*)/s);
  return { nums: core.split(".").map(Number), pre: pre || null };
}

/** Numeric per part; a pre-release ranks below the same version without one. */
export function compareVersions(a, b) {
  const x = parseVersion(a);
  const y = parseVersion(b);
  for (let i = 0; i < 3; i++) {
    if (x.nums[i] !== y.nums[i]) return x.nums[i] < y.nums[i] ? -1 : 1;
  }
  if (x.pre === y.pre) return 0;
  if (x.pre === null) return 1;
  if (y.pre === null) return -1;
  return x.pre < y.pre ? -1 : 1;
}

const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Where the version lives in each file: every pattern must match exactly once. */
function patterns(file, oldV) {
  const v = esc(oldV);
  switch (file) {
    case "package.json":
    case "src-tauri/tauri.conf.json":
      return [new RegExp(`("version":\\s*")${v}(")`, "g")];
    case "package-lock.json":
      return [
        // top-level "version" (two-space indent)
        new RegExp(`(^ {2}"version": ")${v}(")`, "gm"),
        // packages[""] - the root package entry
        new RegExp(`("": \\{\\r?\\n\\s*"name": "[^"]*",\\r?\\n\\s*"version": ")${v}(")`, "g"),
      ];
    case "src-tauri/Cargo.toml":
      return [new RegExp(`(\\[package\\][^\\[]*?\\nversion = ")${v}(")`, "g")];
    case "src-tauri/Cargo.lock":
      // only the CAMPP crate, never a dependency that happens to share the version
      return [new RegExp(`(name = "CAMPP"\\r?\\nversion = ")${v}(")`, "g")];
    default:
      throw new Error(`no version pattern for ${file}`);
  }
}

export const FILES = [
  "package.json",
  "package-lock.json",
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
];

/** New file text with the version replaced; throws unless each pattern matches once. */
export function bumpText(file, text, oldV, newV) {
  let out = text;
  for (const re of patterns(file, oldV)) {
    const n = [...out.matchAll(re)].length;
    if (n !== 1) throw new Error(`${file}: expected 1 version match, found ${n}`);
    out = out.replace(re, `$1${newV}$2`);
  }
  return out;
}

function main(argv) {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const next = argv[0];
  if (!next) {
    console.error("usage: npm run bump -- <version>   e.g. npm run bump -- 0.4.0");
    return 2;
  }
  const current = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
  try {
    parseVersion(next);
    if (compareVersions(next, current) <= 0) {
      console.error(`${next} is not greater than current ${current}`);
      return 2;
    }
  } catch (e) {
    console.error(e.message);
    return 2;
  }
  // Compute every file first; write nothing unless all succeed.
  const updates = [];
  for (const f of FILES) {
    try {
      updates.push([f, bumpText(f, readFileSync(join(root, f), "utf8"), current, next)]);
    } catch (e) {
      console.error(e.message);
      return 1;
    }
  }
  for (const [f, text] of updates) {
    writeFileSync(join(root, f), text);
    console.log(`${f}: ${current} -> ${next}`);
  }
  // Same consistency rule as the release workflow's verify job
  const read = (f) => readFileSync(join(root, f), "utf8");
  const pkg = JSON.parse(read("package.json")).version;
  const tauri = JSON.parse(read("src-tauri/tauri.conf.json")).version;
  const cargo = /\[package\][^[]*?\nversion = "([^"]+)"/.exec(read("src-tauri/Cargo.toml"))?.[1];
  if (pkg !== next || tauri !== next || cargo !== next) {
    console.error(`versions inconsistent: package.json=${pkg} tauri.conf.json=${tauri} Cargo.toml=${cargo}`);
    return 1;
  }
  console.log(`versions consistent: ${next}`);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exit(main(process.argv.slice(2)));
}
