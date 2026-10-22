// ponytail: length + a short list of the passwords people actually type here. Swap for a
// real strength estimator (e.g. zxcvbn) if this ever needs to be more than a nudge.
const COMMON = new Set([
  "root", "admin", "password", "passw0rd", "mysql", "postgres", "postgresql", "campp",
  "12345678", "123456789", "1234567890", "qwerty123", "11111111", "00000000",
]);

/** True for passwords too easy to guess for a database reachable from other devices. */
export function isWeakPassword(pw: string): boolean {
  return pw.length < 8 || COMMON.has(pw.toLowerCase()) || /^(.)\1*$/.test(pw);
}
