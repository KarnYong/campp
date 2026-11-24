/**
 * The DB lines of a Laravel `.env` for CAMPP's MySQL/MariaDB (spec 006 US4,
 * contracts/env-block.md). `mysql` works for both engines; 127.0.0.1 (not localhost)
 * so PHP connects over TCP to the configured port on every OS.
 *
 * phpdotenv reads double-quoted values with `\\`, `\"` escapes and expands `${VAR}`,
 * so `\`, `"` and `$` are escaped. Erasable TS only: the Laravel e2e script imports
 * this file directly under Node's type stripping.
 */
export function buildEnvBlock({ port, password }: { port: number; password: string }): string {
  const quoted = password === "" ? "" : `"${password.replace(/[\\"$]/g, (c) => `\\${c}`)}"`;
  return [
    "DB_CONNECTION=mysql",
    "DB_HOST=127.0.0.1",
    `DB_PORT=${port}`,
    "DB_DATABASE=laravel",
    "DB_USERNAME=root",
    `DB_PASSWORD=${quoted}`,
  ].join("\n") + "\n";
}
