import { describe, expect, it } from 'vitest';

import { parseMarkdownLite, parseMarkdownSpans } from './markdown-lite.ts';


describe('parseMarkdownSpans', () => {
  it('parses bold, italic, and code', () => {
    const spans = parseMarkdownSpans('Say **hello** *softly* with `code`.');
    expect(spans).toEqual([
      { text: 'Say ' },
      { text: 'hello', bold: true },
      { text: ' ' },
      { text: 'softly', italic: true },
      { text: ' with ' },
      { text: 'code', code: true },
      { text: '.' },
    ]);
  });

  it('code spans are terminal', () => {
    expect(parseMarkdownSpans('`**not bold**`')).toEqual([{ text: '**not bold**', code: true }]);
  });

  it('lone markers stay literal', () => {
    expect(parseMarkdownSpans('2 * 3 and **open')).toEqual([{ text: '2 * 3 and **open' }]);
    expect(parseMarkdownSpans('a*b')).toEqual([{ text: 'a*b' }]);
  });

  it('backslash escapes formatting', () => {
    expect(parseMarkdownSpans('\\*literal\\*')).toEqual([{ text: '*literal*' }]);
  });

  it('links only http(s) URLs', () => {
    const safe = parseMarkdownSpans('[docs](https://example.com/a)');
    expect(safe).toEqual([{ text: 'docs', href: 'https://example.com/a' }]);
    const unsafe = parseMarkdownSpans('[x](javascript:alert(1))');
    expect(unsafe).toEqual([{ text: '[x](javascript:alert(1))' }]);
  });

  it('autolinks bare URLs without trailing punctuation', () => {
    const spans = parseMarkdownSpans('See https://example.com/a, then go.');
    expect(spans).toEqual([
      { text: 'See ' },
      { text: 'https://example.com/a', href: 'https://example.com/a' },
      { text: ',' },
      { text: ' then go.' },
    ]);
  });

  it('markup characters are plain text', () => {
    expect(parseMarkdownSpans('<script>alert(1)</script>')).toEqual([
      { text: '<script>alert(1)</script>' },
    ]);
  });
});

describe('parseMarkdownLite', () => {
  it('splits paragraphs and bullet lists', () => {
    const blocks = parseMarkdownLite('First **line**.\n\n- one\n- two\n\nLast.');
    expect(blocks).toHaveLength(3);
    expect(blocks[0]?.kind).toBe('paragraph');
    expect(blocks[1]).toEqual({
      kind: 'list',
      items: [[{ text: 'one' }], [{ text: 'two' }]],
    });
    expect(blocks[2]?.kind).toBe('paragraph');
  });

  it('indented lines continue the list item', () => {
    const blocks = parseMarkdownLite('- one\n  continued\n- two');
    expect(blocks).toEqual([
      { kind: 'list', items: [[{ text: 'one continued' }], [{ text: 'two' }]] },
    ]);
  });

  it('empty input yields no blocks', () => {
    expect(parseMarkdownLite('   \n  ')).toEqual([]);
  });
});
