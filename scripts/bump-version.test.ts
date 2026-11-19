import { describe, it, expect } from "vitest";
// @ts-expect-error - plain ESM script without type declarations
import { parseVersion, compareVersions, bumpText } from "./bump-version.mjs";

describe("bump-version", () => {
  it("validates versions", () => {
    expect(() => parseVersion("0.4.0")).not.toThrow();
    expect(() => parseVersion("0.4.0-beta.1")).not.toThrow();
    expect(() => parseVersion("banana")).toThrow(/invalid version/);
    expect(() => parseVersion("1.2")).toThrow(/invalid version/);
  });

  it("orders versions numerically, pre-release below release", () => {
    expect(compareVersions("0.4.0", "0.3.14")).toBe(1);
    expect(compareVersions("0.3.10", "0.3.9")).toBe(1);
    expect(compareVersions("0.4.0-beta.1", "0.4.0")).toBe(-1);
    expect(compareVersions("0.3.14", "0.3.14")).toBe(0);
  });

  it("changes only the version line and keeps CRLF", () => {
    const crlf = '{\r\n  "name": "tauri-app",\r\n  "version": "0.3.14",\r\n  "private": true\r\n}\r\n';
    const out = bumpText("package.json", crlf, "0.3.14", "0.4.0");
    expect(out).toBe(crlf.replace("0.3.14", "0.4.0"));
    expect(out.split("\r\n").length).toBe(crlf.split("\r\n").length);
  });

  it("bumps both package-lock entries and nothing else", () => {
    const lock = [
      "{",
      '  "name": "tauri-app",',
      '  "version": "0.3.14",',
      '  "packages": {',
      '    "": {',
      '      "name": "tauri-app",',
      '      "version": "0.3.14",',
      "    },",
      '    "node_modules/x": {',
      '      "version": "0.3.14"',
      "    }",
      "  }",
      "}",
    ].join("\n");
    const out = bumpText("package-lock.json", lock, "0.3.14", "0.4.0");
    expect(out.match(/0\.4\.0/g)).toHaveLength(2);
    expect(out).toContain('"node_modules/x": {\n      "version": "0.3.14"'); // dependency untouched
  });

  it("Cargo.lock: only the CAMPP crate, not a dependency with the same version", () => {
    const lock = '[[package]]\r\nname = "CAMPP"\r\nversion = "0.3.14"\r\n\r\n[[package]]\r\nname = "errno"\r\nversion = "0.3.14"\r\n';
    const out = bumpText("src-tauri/Cargo.lock", lock, "0.3.14", "0.4.0");
    expect(out).toContain('name = "CAMPP"\r\nversion = "0.4.0"');
    expect(out).toContain('name = "errno"\r\nversion = "0.3.14"');
  });

  it("Cargo.toml: the [package] version, not a dependency's", () => {
    const toml = '[package]\nname = "CAMPP"\nversion = "0.3.14"\n\n[dependencies.x]\nversion = "0.3.14"\n';
    const out = bumpText("src-tauri/Cargo.toml", toml, "0.3.14", "0.4.0");
    expect(out).toBe('[package]\nname = "CAMPP"\nversion = "0.4.0"\n\n[dependencies.x]\nversion = "0.3.14"\n');
  });

  it("refuses zero or ambiguous matches", () => {
    expect(() => bumpText("src-tauri/tauri.conf.json", '{"version": "9.9.9"}', "0.3.14", "0.4.0")).toThrow(/found 0/);
    expect(() =>
      bumpText("src-tauri/tauri.conf.json", '{"version": "0.3.14", "x": {"version": "0.3.14"}}', "0.3.14", "0.4.0"),
    ).toThrow(/found 2/);
  });
});
