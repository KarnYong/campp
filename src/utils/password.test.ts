import { describe, it, expect } from 'vitest';
import { isWeakPassword } from './password';

describe('isWeakPassword', () => {
  it('flags short, common and repeated passwords', () => {
    for (const pw of ['', 'abc', '1234567', 'password', 'PASSWORD', 'Root', '12345678', 'aaaaaaaaaa', 'campp']) {
      expect(isWeakPassword(pw), pw).toBe(true);
    }
  });

  it('accepts longer, uncommon passwords', () => {
    for (const pw of ['pgsecret', 'Long-and-unique-77', 'correct horse battery']) {
      expect(isWeakPassword(pw), pw).toBe(false);
    }
  });
});
