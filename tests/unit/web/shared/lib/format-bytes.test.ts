import { expect, it } from 'vitest';
import { formatBytes } from '../../../../../src/web/shared/lib/format-bytes.ts';

it('formats sizes with the right unit and clamps negatives', () => {
  expect(formatBytes(0, 'en')).toBe('0 B');
  expect(formatBytes(1536, 'en')).toBe('1.5 KB');
  expect(formatBytes(16 * 1024 * 1024, 'en')).toBe('16.0 MB');
  expect(formatBytes(-5, 'en')).toBe('0 B');
});
