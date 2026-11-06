import { describe, expect, test } from 'bun:test';
import { validateVersions } from './check-version.ts';

describe('release version validation', () => {
  test('rejects the historical v0.5.0 tag with 0.3.3 manifests', () => {
    expect(() => validateVersions('0.3.3', '0.3.3', 'v0.5.0', '0.3.3')).toThrow();
  });
  test('accepts matching prerelease versions', () => {
    expect(validateVersions('1.0.0-beta.1', '1.0.0-beta.1', 'v1.0.0-beta.1', '1.0.0-beta.1')).toBe('1.0.0-beta.1');
  });
  test('rejects malformed tags and mismatched binary', () => {
    for (const tag of ['1.0.0', 'v01.0.0', 'v1.0.0-beta..1', 'v1.0.0-01', 'v1.0.0/evil']) {
      expect(() => validateVersions('1.0.0', '1.0.0', tag)).toThrow();
    }
    expect(() => validateVersions('1.0.0', '1.0.0', 'v1.0.0', '0.9.0')).toThrow();
  });
});
