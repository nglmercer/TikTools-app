import { describe, expect, it } from 'vitest';
import { linesText, plainTextLines, prettyJson, tokenizeJson, tokenizeJsonSource } from './json-tokens.ts';

const sample = {
  name: 'Gift alert',
  enabled: true,
  count: -3.5e2,
  nothing: null,
  tags: ['a', 'b'],
  nested: { empty: {}, none: [], deep: [{ x: 1 }, [2, 3]] },
  quote: 'line\n"break" \\ ✓',
};

describe('tokenizeJson', () => {
  it('prints exactly what JSON.stringify prints with two-space indentation', () => {
    expect(linesText(tokenizeJson(sample).lines)).toBe(JSON.stringify(sample, null, 2));
    for (const value of [0, 'text', true, null, [], {}, [[]], [{}], { a: [] }, [1, [2, [3]]], 12345678901234, 1e21, -0]) {
      expect(linesText(tokenizeJson(value).lines), JSON.stringify(value)).toBe(JSON.stringify(value, null, 2));
    }
  });

  it('classifies tokens by kind', () => {
    const { lines } = tokenizeJson({ a: 'x', b: 1, c: false, d: null, e: [] });
    const kinds = lines.map((line) => line.filter((token) => token.kind !== 'ws').map((token) => token.kind));
    expect(kinds).toEqual([
      ['punct'],
      ['key', 'punct', 'string', 'punct'],
      ['key', 'punct', 'number', 'punct'],
      ['key', 'punct', 'boolean', 'punct'],
      ['key', 'punct', 'null', 'punct'],
      ['key', 'punct', 'punct'],
      ['punct'],
    ]);
    expect(lines[1]![0]).toEqual({ kind: 'ws', text: '  ' });
    expect(lines[1]![1]).toEqual({ kind: 'key', text: '"a"' });
  });

  it('keeps keys in the order they were given and optionally sorts them', () => {
    const value = { b: 1, a: { d: 1, c: 2 } };
    expect(linesText(tokenizeJson(value).lines)).toBe(JSON.stringify(value, null, 2));
    expect(linesText(tokenizeJson(value, { sortKeys: true }).lines)).toBe(JSON.stringify({ a: { c: 2, d: 1 }, b: 1 }, null, 2));
    expect(prettyJson(value, { sortKeys: true })).toBe(JSON.stringify({ a: { c: 2, d: 1 }, b: 1 }, null, 2));
  });

  it('follows JSON.stringify for unsupported values', () => {
    const value = { skipped: undefined, fn: () => 1, date: new Date(0), nan: Number.NaN, list: [undefined, () => 1, Symbol('s')], big: 12n };
    expect(linesText(tokenizeJson(value).lines)).toBe(
      ['{', '  "date": "1970-01-01T00:00:00.000Z",', '  "nan": null,', '  "list": [', '    null,', '    null,', '    null', '  ],', '  "big": 12', '}'].join('\n'),
    );
  });

  it('prints nothing for undefined and a placeholder for cycles', () => {
    expect(tokenizeJson(undefined).lines).toEqual([]);
    const loop: Record<string, unknown> = { name: 'loop' };
    loop['self'] = loop;
    expect(linesText(tokenizeJson(loop).lines)).toBe('{\n  "name": "loop",\n  "self": "[Circular]"\n}');
    expect(prettyJson(loop)).toBe('{\n  "name": "loop",\n  "self": "[Circular]"\n}');
  });

  it('does not treat a repeated (non-cyclic) object as a cycle', () => {
    const shared = { x: 1 };
    expect(linesText(tokenizeJson({ a: shared, b: shared }).lines)).toBe(JSON.stringify({ a: shared, b: shared }, null, 2));
  });

  it('stops at maxLines and reports truncation', () => {
    const value = Array.from({ length: 100 }, (_, index) => index);
    const result = tokenizeJson(value, { maxLines: 10 });
    expect(result.lines).toHaveLength(10);
    expect(result.truncated).toBe(true);
    expect(linesText(result.lines).startsWith('[\n  0,\n  1,')).toBe(true);
    expect(tokenizeJson(value, { maxLines: 102 }).truncated).toBe(false);
  });

  it('stops at maxChars and clips giant tokens', () => {
    const many = tokenizeJson(Array.from({ length: 1000 }, () => 'abcdefghij'), { maxChars: 500 });
    expect(many.truncated).toBe(true);
    expect(many.lines.length).toBeLessThan(40);
    const giant = tokenizeJson({ blob: 'x'.repeat(100_000) }, { maxLineChars: 100 });
    expect(giant.truncated).toBe(true);
    expect(linesText(giant.lines).length).toBeLessThan(200);
    expect(giant.lines[1]!.some((token) => token.text.endsWith('…"') || token.text.endsWith('…'))).toBe(true);
  });

  it('handles very deep nesting without overflowing the stack', () => {
    let value: unknown = 1;
    for (let depth = 0; depth < 20_000; depth += 1) value = [value];
    const result = tokenizeJson(value, { maxLines: 50_000, maxChars: 5_000_000 });
    expect(result.lines.length).toBeGreaterThan(1000);
  });

  it('tokenizes a 5000-line document quickly', () => {
    const value = { items: Array.from({ length: 1250 }, (_, index) => ({ id: index, name: `item ${index}`, on: index % 2 === 0 })) };
    const started = performance.now();
    const result = tokenizeJson(value);
    expect(result.lines.length).toBeGreaterThanOrEqual(5000);
    expect(result.truncated).toBe(true);
    expect(performance.now() - started).toBeLessThan(1500);
  });
});

describe('tokenizeJsonSource', () => {
  it('pretty-prints valid JSON text', () => {
    const result = tokenizeJsonSource('{"a":[1,2],"b":"x"}');
    expect(result.plain).toBe(false);
    expect(linesText(result.lines)).toBe('{\n  "a": [\n    1,\n    2\n  ],\n  "b": "x"\n}');
  });

  it('shows text that is not JSON verbatim as plain text', () => {
    const result = tokenizeJsonSource('{"a": 1,\r\n  oops');
    expect(result.plain).toBe(true);
    expect(result.lines.map((line) => line[0]!.kind)).toEqual(['text', 'text']);
    expect(linesText(result.lines)).toBe('{"a": 1,\n  oops');
  });

  it('applies the limits to plain text too', () => {
    const result = plainTextLines('a\nb\nc\nd', { maxLines: 2 });
    expect(result.lines).toHaveLength(2);
    expect(result.truncated).toBe(true);
    expect(plainTextLines('', {}).lines).toEqual([[{ kind: 'text', text: '' }]]);
  });
});

describe('prettyJson', () => {
  it('returns the full untruncated text, tolerating values JSON.stringify rejects', () => {
    expect(prettyJson({ a: [1] })).toBe('{\n  "a": [\n    1\n  ]\n}');
    expect(prettyJson({ big: 5n })).toBe('{\n  "big": 5\n}');
    expect(prettyJson(undefined)).toBe('');
  });
});
