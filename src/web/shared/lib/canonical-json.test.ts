import { createHash } from 'node:crypto';
import { expect, test } from 'vitest';
import { canonicalJson } from './canonical-json.ts';
import { sha256 } from './sha256.ts';

test.each([
  ['', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'],
  ['abc', 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'],
  ['abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq', '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1'],
])('SHA-256 matches standard vector %s', (value, expected) => { expect(sha256(value)).toBe(expected); });
test('canonical JSON preserves scalar key ordering, numeric keys, arrays, exact floats and Unicode source', () => {
  const source = { '2': 'two', '10': 'ten', '\u{10000}': { '\uE000': '🌹', a: 0.000001 }, '\uE000': [-0, 1.0, 1e-7], ['__proto__']: 'literal' };
  const serialized = canonicalJson(source);
  expect(serialized).toBe('{"10":"ten","2":"two","__proto__":"literal","":[0,1,1e-7],"𐀀":{"a":0.000001,"":"🌹"}}');
  expect(sha256(serialized)).toBe(createHash('sha256').update(serialized).digest('hex'));
  expect(() => canonicalJson({ unsafe: 9007199254740992 })).toThrow('exactly');
});
