import { describe, expect, it } from 'vitest';
import { changedTopLevelKeys, diffJson, diffLines, diffSequences, foldRows, jsonEqual, segmentLine, wordRanges } from './json-diff.ts';
import { tokenizeJson } from './json-tokens.ts';

/** Replays an edit script to make sure it really turns `before` into `after`. */
function apply(before: string[], after: string[], ops: string[]): string[] {
  const out: string[] = [];
  let i = 0;
  let j = 0;
  for (const op of ops) {
    if (op === 'equal') { expect(before[i]).toBe(after[j]); out.push(before[i]!); i += 1; j += 1; }
    else if (op === 'delete') i += 1;
    else { out.push(after[j]!); j += 1; }
  }
  expect(i).toBe(before.length);
  expect(j).toBe(after.length);
  return out;
}

describe('diffSequences', () => {
  it('handles empty and identical inputs', () => {
    expect(diffSequences([], [])).toEqual([]);
    expect(diffSequences(['a'], ['a'])).toEqual(['equal']);
    expect(diffSequences([], ['a', 'b'])).toEqual(['insert', 'insert']);
    expect(diffSequences(['a', 'b'], [])).toEqual(['delete', 'delete']);
  });

  it('finds a shortest script (classic Myers example)', () => {
    const before = 'ABCABBA'.split('');
    const after = 'CBABAC'.split('');
    const ops = diffSequences(before, after);
    expect(apply(before, after, ops)).toEqual(after);
    expect(ops.filter((op) => op !== 'equal')).toHaveLength(5);
  });

  it('produces valid scripts for random inputs', () => {
    let seed = 7;
    const random = (): number => { seed = (seed * 1664525 + 1013904223) % 4294967296; return seed / 4294967296; };
    for (let round = 0; round < 60; round += 1) {
      const a = Array.from({ length: Math.floor(random() * 30) }, () => String(Math.floor(random() * 5)));
      const b = Array.from({ length: Math.floor(random() * 30) }, () => String(Math.floor(random() * 5)));
      expect(apply(a, b, diffSequences(a, b))).toEqual(b);
    }
  });

  it('falls back to replacing the differing middle when the edit distance cap is hit', () => {
    const before = ['keep', ...Array.from({ length: 50 }, (_, index) => `old ${index}`), 'tail'];
    const after = ['keep', ...Array.from({ length: 50 }, (_, index) => `new ${index}`), 'tail'];
    const ops = diffSequences(before, after, 10);
    expect(apply(before, after, ops)).toEqual(after);
    expect(ops[0]).toBe('equal');
    expect(ops.at(-1)).toBe('equal');
    expect(ops.filter((op) => op === 'delete')).toHaveLength(50);
  });

  it('diffs 5000 lines with scattered edits and a completely different 5000 lines fast', () => {
    const before = Array.from({ length: 5000 }, (_, index) => `line ${index}`);
    const after = before.filter((_, index) => index % 50 !== 0).map((line, index) => (index % 70 === 0 ? `${line} edited` : line));
    const started = performance.now();
    expect(apply(before, after, diffSequences(before, after))).toEqual(after);
    const other = before.map((line) => `${line} other`);
    expect(apply(before, other, diffSequences(before, other))).toEqual(other);
    expect(performance.now() - started).toBeLessThan(3000);
  });
});

describe('diffJson', () => {
  it('reports identical documents without changed rows', () => {
    const diff = diffJson({ a: 1, b: [1, 2] }, { a: 1, b: [1, 2] });
    expect(diff.identical).toBe(true);
    expect(diff.stats).toEqual({ added: 0, removed: 0, changed: 0, unchanged: 7 });
    expect(diff.rows.every((row) => row.kind === 'unchanged' && row.oldNumber === row.newNumber)).toBe(true);
  });

  it('treats a missing side as an empty document', () => {
    const created = diffJson(undefined, { a: 1 });
    expect(created.rows.map((row) => row.kind)).toEqual(['added', 'added', 'added']);
    expect(created.rows.map((row) => [row.oldNumber, row.newNumber])).toEqual([[null, 1], [null, 2], [null, 3]]);
    const removed = diffJson({ a: 1 }, undefined);
    expect(removed.stats.removed).toBe(3);
    expect(diffJson(undefined, undefined)).toMatchObject({ rows: [], identical: true });
  });

  it('pairs a modified value as one changed row with both line numbers and word ranges', () => {
    const diff = diffJson({ name: 'Old name', n: 1 }, { name: 'New name', n: 1 });
    const changed = diff.rows.filter((row) => row.kind === 'changed');
    expect(changed).toHaveLength(1);
    const row = changed[0]!;
    expect([row.oldNumber, row.newNumber]).toEqual([2, 2]);
    expect(row.oldText).toBe('  "name": "Old name",');
    expect(row.newText).toBe('  "name": "New name",');
    expect(row.oldRanges.map(([s, e]) => row.oldText!.slice(s, e))).toEqual(['Old']);
    expect(row.newRanges.map(([s, e]) => row.newText!.slice(s, e))).toEqual(['New']);
    expect(diff.stats).toEqual({ added: 0, removed: 0, changed: 1, unchanged: 3 });
  });

  it('reports added and removed lines with the right numbering', () => {
    const diff = diffJson({ a: 1, b: 2 }, { a: 1, c: 3, d: 4 });
    const kinds = diff.rows.map((row) => row.kind);
    expect(kinds).toEqual(['unchanged', 'unchanged', 'changed', 'added', 'unchanged']);
    expect(diff.rows[3]).toMatchObject({ oldNumber: null, newNumber: 4, newText: '  "d": 4' });
  });

  it('follows nested changes through arrays and objects', () => {
    const before = { rules: [{ id: 1, on: true }, { id: 2, on: true }], meta: { v: 1 } };
    const after = { rules: [{ id: 1, on: false }, { id: 2, on: true }, { id: 3, on: true }], meta: { v: 1 } };
    const diff = diffJson(before, after);
    expect(diff.rows.filter((row) => row.kind === 'changed').map((row) => row.newText)).toContain('      "on": false');
    expect(diff.stats.added).toBeGreaterThanOrEqual(4);
    const oldNumbers = diff.rows.map((row) => row.oldNumber).filter((n): n is number => n !== null);
    expect(oldNumbers).toEqual([...oldNumbers].sort((a, b) => a - b));
  });

  it('shows reordered keys as changes unless keys are sorted', () => {
    const a = { x: 1, y: 2 };
    const b = { y: 2, x: 1 };
    expect(diffJson(a, b).identical).toBe(false);
    expect(diffJson(a, b, { sortKeys: true }).identical).toBe(true);
  });

  it('does not mark words for lines that share too little', () => {
    const row = diffJson({ alpha: 'one' }, { omega: 'two' }).rows.find((item) => item.kind === 'changed')!;
    expect(row.oldRanges).toEqual([]);
    expect(row.newRanges).toEqual([]);
  });

  it('copes with huge documents', () => {
    const make = (offset: number) => ({ items: Array.from({ length: 1250 }, (_, index) => ({ id: index, name: `item ${index + (index % 97 === 0 ? offset : 0)}`, on: index % 2 === 0 })) });
    const started = performance.now();
    const diff = diffJson(make(0), make(1), { maxLines: 20_000 });
    expect(diff.rows.length).toBeGreaterThan(6000);
    expect(diff.stats.changed).toBe(13);
    expect(diff.truncated).toBe(false);
    expect(performance.now() - started).toBeLessThan(3000);
    const capped = diffJson(make(0), make(1));
    expect(capped.truncated).toBe(true);
  });
});

describe('wordRanges and segmentLine', () => {
  it('marks only the differing words', () => {
    const { oldRanges, newRanges } = wordRanges('  "level": "high",', '  "level": "low",');
    expect(oldRanges).toEqual([[12, 16]]);
    expect(newRanges).toEqual([[12, 15]]);
  });

  it('splits syntax tokens at changed boundaries', () => {
    const [line] = tokenizeJson('abcdef').lines;
    const segments = segmentLine(line!, [[2, 4]]);
    expect(segments.map((segment) => [segment.text, segment.changed])).toEqual([['"a', false], ['bc', true], ['def"', false]]);
    expect(segments.every((segment) => segment.kind === 'string')).toBe(true);
    expect(segmentLine(line!, [])).toEqual([{ kind: 'string', text: '"abcdef"', changed: false }]);
    const spanning = segmentLine([{ kind: 'key', text: '"ab"' }, { kind: 'punct', text: ': ' }, { kind: 'number', text: '12' }], [[3, 7]]);
    expect(spanning.map((segment) => [segment.kind, segment.text, segment.changed])).toEqual([
      ['key', '"ab', false], ['key', '"', true], ['punct', ': ', true], ['number', '1', true], ['number', '2', false],
    ]);
  });
});

describe('foldRows', () => {
  const rowsOf = (before: unknown, after: unknown) => diffLines(tokenizeJson(before).lines, tokenizeJson(after).lines).rows;
  const big = (flag: boolean) => ({ list: Array.from({ length: 40 }, (_, index) => index), flag });

  it('keeps context around changes and folds long unchanged runs', () => {
    const rows = rowsOf(big(false), big(true));
    const items = foldRows(rows);
    const folds = items.filter((item) => item.type === 'fold');
    expect(folds).toHaveLength(1);
    expect(folds[0]).toMatchObject({ type: 'fold', start: 0, count: 40 });
    expect(items.filter((item) => item.type === 'row').length).toBe(rows.length - 40);
  });

  it('folds from both sides of a change in the middle and respects context', () => {
    const before = { head: Array.from({ length: 30 }, (_, i) => i), mid: 1, tail: Array.from({ length: 30 }, (_, i) => i) };
    const rows = rowsOf(before, { ...before, mid: 2 });
    const items = foldRows(rows, { context: 2 });
    expect(items.filter((item) => item.type === 'fold')).toHaveLength(2);
    const changedAt = rows.findIndex((row) => row.kind === 'changed');
    const visible = items.filter((item) => item.type === 'row').map((item) => (item as { index: number }).index);
    expect(visible).toContain(changedAt - 2);
    expect(visible).not.toContain(changedAt - 3);
    expect(visible).toContain(changedAt + 2);
    expect(visible).not.toContain(changedAt + 3);
  });

  it('keeps short runs visible and folds a fully identical document into one marker', () => {
    const rows = rowsOf({ a: 1, b: 2 }, { a: 1, b: 3 });
    expect(foldRows(rows).every((item) => item.type === 'row')).toBe(true);
    const same = rowsOf(big(true), big(true));
    expect(foldRows(same)).toEqual([{ type: 'fold', start: 0, count: same.length }]);
  });
});

describe('jsonEqual and changedTopLevelKeys', () => {
  it('compares structurally ignoring key order', () => {
    expect(jsonEqual({ a: 1, b: { c: [1, { d: 2 }] } }, { b: { c: [1, { d: 2 }] }, a: 1 })).toBe(true);
    expect(jsonEqual([1, 2], [2, 1])).toBe(false);
    expect(jsonEqual({ a: 1 }, { a: 1, b: undefined })).toBe(true);
    expect(jsonEqual({ a: 1 }, { a: '1' })).toBe(false);
    expect(jsonEqual(null, undefined)).toBe(false);
    expect(jsonEqual([], {})).toBe(false);
  });

  it('lists which top-level keys were added, removed, changed or kept', () => {
    const result = changedTopLevelKeys({ id: 'a', name: 'x', config: { n: 1 }, old: 1 }, { config: { n: 2 }, id: 'a', name: 'x', fresh: true });
    expect(result).toEqual({ added: ['fresh'], removed: ['old'], changed: ['config'], unchanged: ['id', 'name'], rootChanged: false });
  });

  it('reports a changed root when either side is not an object', () => {
    expect(changedTopLevelKeys([1], [1, 2]).rootChanged).toBe(true);
    expect(changedTopLevelKeys(undefined, { a: 1 }).rootChanged).toBe(true);
    expect(changedTopLevelKeys('a', 'a').rootChanged).toBe(false);
  });
});
