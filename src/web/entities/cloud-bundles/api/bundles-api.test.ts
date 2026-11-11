import { describe, expect, it } from 'vitest';
import { InvalidBundleResponse, parseAsset, parsePreflight, toBase64 } from './bundles-api.ts';

describe('bundle response parsing', () => {
  it('keeps the preflight items and refuses a report that does not use client apply', () => {
    const report = parsePreflight({
      packageVersion: 1,
      items: [{ kind: 'plugin', id: 'acme.gift', version: '1.2.0', revision: null, required: true, status: 'ready', permissions: 'reviewRequired' }],
      blocking: [],
      canApply: true,
      applyMode: 'clientJournal',
    });
    expect(report.canApply).toBe(true);
    expect(report.items[0]).toMatchObject({ kind: 'plugin', version: '1.2.0', permissions: 'reviewRequired' });
    expect(() => parsePreflight({ items: [], canApply: true, applyMode: 'server' })).toThrow(InvalidBundleResponse);
  });

  it('requires an asset identifier that is a UUID', () => {
    expect(parseAsset({ id: '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c', mime: 'image/png', width: 2, height: 2, size: 80 }).width).toBe(2);
    expect(() => parseAsset({ id: 'x', mime: 'image/png', width: 1, height: 1, size: 1 })).toThrow(InvalidBundleResponse);
  });

  it('encodes large byte arrays without overflowing the argument limit', () => {
    const bytes = new Uint8Array(300_000).fill(65);
    expect(atob(toBase64(bytes))).toHaveLength(300_000);
    expect(toBase64(new Uint8Array([0x89, 0x50]))).toBe(btoa('\u0089P'));
  });
});
