import { describe, expect, it } from 'vitest';
import { buildTree, expandableIds, ROOT_ID } from './json-tree.ts';

const value = { name: 'x', list: [1, { deep: true }], empty: {}, nothing: null };

const summary = (rows: ReturnType<typeof buildTree>['rows']): string[] =>
  rows.map((row) => `${row.depth}:${row.kind}:${row.label ?? ''}:${row.tokens.map((token) => token.text).join('')}`);

describe('buildTree', () => {
  it('shows a collapsed root with its member count', () => {
    const { rows } = buildTree(value, { expanded: new Set() });
    expect(summary(rows)).toEqual(['0:node::{4}']);
    expect(rows[0]).toMatchObject({ expandable: true, open: false, count: 4, array: false });
    expect(buildTree([1, 2, 3, 4, 5], { expanded: new Set() }).rows[0]!.tokens[0]!.text).toBe('[5]');
  });

  it('expands the root into labelled children with counts for collapsed containers', () => {
    const { rows } = buildTree(value, { expanded: new Set([ROOT_ID]) });
    expect(summary(rows)).toEqual([
      '0:node::{',
      '1:node:"name":"x"',
      '1:node:"list":[2]',
      '1:node:"empty":{}',
      '1:node:"nothing":null',
      '0:close::}',
    ]);
    expect(rows.find((row) => row.label === '"empty"')!.expandable).toBe(false);
  });

  it('expands nested nodes by id and labels array items with their index', () => {
    const { rows } = buildTree(value, { expanded: new Set([ROOT_ID, '$/1', '$/1/1']) });
    expect(summary(rows)).toContain('2:node:0:1');
    expect(summary(rows)).toContain('2:node:1:{');
    expect(summary(rows)).toContain('3:node:"deep":true');
    expect(rows.filter((row) => row.kind === 'close')).toHaveLength(3);
  });

  it('marks nodes past the depth cap as non-expandable summaries', () => {
    const deep = { a: { b: { c: { d: 1 } } } };
    const all = expandableIds(deep, { maxDepth: 2 });
    expect([...all]).toEqual([ROOT_ID, '$/0']);
    const { rows } = buildTree(deep, { expanded: new Set([ROOT_ID, '$/0', '$/0/0']), maxDepth: 2 });
    const capped = rows.find((row) => row.label === '"b"')!;
    expect(capped.expandable).toBe(false);
    expect(capped.tokens[0]!.text).toBe('{1}');
    expect(rows.some((row) => row.label === '"c"')).toBe(false);
  });

  it('limits children per node and total rows', () => {
    const list = Array.from({ length: 50 }, (_, index) => index);
    const wide = buildTree(list, { expanded: new Set([ROOT_ID]), maxChildren: 10 });
    expect(wide.truncated).toBe(true);
    const more = wide.rows.find((row) => row.kind === 'more')!;
    expect(more.count).toBe(40);
    const tall = buildTree(list, { expanded: new Set([ROOT_ID]), maxRows: 5 });
    expect(tall.rows).toHaveLength(5);
    expect(tall.truncated).toBe(true);
  });

  it('survives cycles, undefined members and a 5000 row document', () => {
    const loop: Record<string, unknown> = { gone: undefined };
    loop['me'] = loop;
    const rows = buildTree(loop, { expanded: new Set([ROOT_ID]) }).rows;
    expect(summary(rows)).toEqual(['0:node::{', '1:node:"me":"[Circular]"', '0:close::}']);
    const big = { items: Array.from({ length: 5000 }, (_, index) => ({ id: index })) };
    const started = performance.now();
    const model = buildTree(big, { expanded: expandableIds(big), maxRows: 6000, maxChildren: 6000 });
    expect(model.rows.length).toBeGreaterThan(5000);
    expect(performance.now() - started).toBeLessThan(1500);
    expect(buildTree(undefined, { expanded: new Set() }).rows).toEqual([]);
  });
});
