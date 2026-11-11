import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { PLUGIN_ID_PATTERN, assertPluginId, isValidPluginId } from './plugin-id.ts';

type Corpus = { valid: Array<{ label: string; id: string }>; invalid: Array<{ label: string; id: string }> };

const sdkRoot = resolve(import.meta.dir, '..', '..', 'packages', 'plugin-sdk');
// The same file is read by the Rust host test (`validation.rs`).
const corpus = JSON.parse(readFileSync(resolve(sdkRoot, 'plugin-id-corpus.json'), 'utf8')) as Corpus;
const schema = JSON.parse(readFileSync(resolve(sdkRoot, 'plugin.schema.json'), 'utf8')) as {
  properties: { id: { pattern: string } };
};
const schemaPattern = new RegExp(schema.properties.id.pattern);

describe('plugin id contract', () => {
  test('the corpus covers the boundary lengths', () => {
    const lengths = new Set([...corpus.valid, ...corpus.invalid].map(({ id }) => id.length));
    for (const length of [1, 2, 64, 65, 128, 129]) expect(lengths.has(length)).toBe(true);
  });

  test('the schema pattern is the shared validator pattern', () => {
    expect(schemaPattern.source).toBe(PLUGIN_ID_PATTERN.source);
  });

  for (const { label, id } of corpus.valid) {
    test(`accepts ${label}`, () => {
      expect(isValidPluginId(id)).toBe(true);
      expect(schemaPattern.test(id)).toBe(true);
      expect(assertPluginId(id, 'plugin.json')).toBe(id);
    });
  }

  for (const { label, id } of corpus.invalid) {
    test(`rejects ${label}`, () => {
      expect(isValidPluginId(id)).toBe(false);
      expect(schemaPattern.test(id)).toBe(false);
      expect(() => assertPluginId(id, '/repo/plugins/x/plugin.json')).toThrow(/\/repo\/plugins\/x\/plugin\.json/);
    });
  }

  test('non-string ids are rejected and never normalized', () => {
    for (const id of [undefined, null, 7, {}, ['ab']]) expect(isValidPluginId(id)).toBe(false);
    expect(() => assertPluginId(' ab ', 'plugin.json')).toThrow(/invalid plugin id/);
  });
});
