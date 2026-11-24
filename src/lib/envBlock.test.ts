import { describe, expect, it } from "vitest";
import { buildEnvBlock } from "./envBlock";

const head = "DB_CONNECTION=mysql\nDB_HOST=127.0.0.1\nDB_PORT=3307\nDB_DATABASE=laravel\nDB_USERNAME=root\n";

describe("buildEnvBlock (spec 006 contracts/env-block.md)", () => {
  it("writes the six lines in order with a trailing newline", () => {
    expect(buildEnvBlock({ port: 3307, password: "secret" })).toBe(`${head}DB_PASSWORD="secret"\n`);
  });

  it("uses the configured port", () => {
    expect(buildEnvBlock({ port: 13307, password: "" })).toContain("\nDB_PORT=13307\n");
  });

  it("leaves an empty password bare", () => {
    expect(buildEnvBlock({ port: 3307, password: "" })).toBe(`${head}DB_PASSWORD=\n`);
  });

  it.each([
    ["back\\slash", '"back\\\\slash"'],
    ['qu"ote', '"qu\\"ote"'],
    ["do$llar", '"do\\$llar"'],
    ["sp ace#hash'tick", "\"sp ace#hash'tick\""],
    ['e2e pa"ss$#\\\'w0rd', '"e2e pa\\"ss\\$#\\\\\'w0rd"'],
  ])("escapes %s", (password, quoted) => {
    expect(buildEnvBlock({ port: 3307, password })).toBe(`${head}DB_PASSWORD=${quoted}\n`);
  });
});
