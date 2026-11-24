import { describe, expect, it } from 'vitest';
import { isGlobalKey, sortedGlobalEntries } from '../../../../../../src/web/entities/runtime-global/lib/global-key.ts';

describe('isGlobalKey', () => {
  it('mirrors the host identifier rule', () => {
    for (const key of ['commandPort', '_port', 'a', 'A1._-b']) expect(isGlobalKey(key)).toBe(true);
    for (const key of ['', '9lives', 'has space', 'semi;colon', 'slash/a', '{{ x }}', 'k'.repeat(65)]) {
      expect(isGlobalKey(key)).toBe(false);
    }
  });
});

describe('sortedGlobalEntries', () => {
  it('orders entries by key', () => {
    expect(sortedGlobalEntries({ b: '2', a: '1' })).toEqual([
      ['a', '1'],
      ['b', '2'],
    ]);
  });
});
