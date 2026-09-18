import { describe, expect, test } from 'bun:test';
import { createSSRApp, h } from 'vue';
import { renderToString } from 'vue/server-renderer';

import {
  parseMarkdownLite,
  parseMarkdownSpans,
  renderMarkdownLite,
} from './plugin-markdown.ts';

async function renderHtml(source: string, inline = false): Promise<string> {
  const app = createSSRApp({ render: () => h('div', null, renderMarkdownLite(source, inline)) });
  app.config.warnHandler = () => {};
  return renderToString(app);
}

describe('parseMarkdownSpans', () => {
  test('parses bold, italic, and code', () => {
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

  test('code spans are terminal', () => {
    expect(parseMarkdownSpans('`**not bold**`')).toEqual([{ text: '**not bold**', code: true }]);
  });

  test('lone markers stay literal', () => {
    expect(parseMarkdownSpans('2 * 3 and **open')).toEqual([{ text: '2 * 3 and **open' }]);
    expect(parseMarkdownSpans('a*b')).toEqual([{ text: 'a*b' }]);
  });

  test('backslash escapes formatting', () => {
    expect(parseMarkdownSpans('\\*literal\\*')).toEqual([{ text: '*literal*' }]);
  });

  test('links only http(s) URLs', () => {
    const safe = parseMarkdownSpans('[docs](https://example.com/a)');
    expect(safe).toEqual([{ text: 'docs', href: 'https://example.com/a' }]);
    const unsafe = parseMarkdownSpans('[x](javascript:alert(1))');
    expect(unsafe).toEqual([{ text: '[x](javascript:alert(1))' }]);
  });

  test('autolinks bare URLs without trailing punctuation', () => {
    const spans = parseMarkdownSpans('See https://example.com/a, then go.');
    expect(spans).toEqual([
      { text: 'See ' },
      { text: 'https://example.com/a', href: 'https://example.com/a' },
      { text: ',' },
      { text: ' then go.' },
    ]);
  });

  test('markup characters are plain text', () => {
    expect(parseMarkdownSpans('<script>alert(1)</script>')).toEqual([
      { text: '<script>alert(1)</script>' },
    ]);
  });
});

describe('parseMarkdownLite', () => {
  test('splits paragraphs and bullet lists', () => {
    const blocks = parseMarkdownLite('First **line**.\n\n- one\n- two\n\nLast.');
    expect(blocks).toHaveLength(3);
    expect(blocks[0]?.kind).toBe('paragraph');
    expect(blocks[1]).toEqual({
      kind: 'list',
      items: [[{ text: 'one' }], [{ text: 'two' }]],
    });
    expect(blocks[2]?.kind).toBe('paragraph');
  });

  test('indented lines continue the list item', () => {
    const blocks = parseMarkdownLite('- one\n  continued\n- two');
    expect(blocks).toEqual([
      { kind: 'list', items: [[{ text: 'one continued' }], [{ text: 'two' }]] },
    ]);
  });

  test('empty input yields no blocks', () => {
    expect(parseMarkdownLite('   \n  ')).toEqual([]);
  });
});

describe('renderMarkdownLite', () => {
  test('block mode renders paragraphs, lists, code, and safe links', async () => {
    const html = await renderHtml('Say **hi** with `code`.\n\n- [docs](https://example.com)\n- plain');
    expect(html).toContain('<p class="plg-md__p">');
    expect(html).toContain('<strong>hi</strong>');
    expect(html).toContain('<code class="plg-md__code">code</code>');
    expect(html).toContain('<ul class="plg-md__list">');
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer"');
  });

  test('dangerous markup renders inert', async () => {
    const html = await renderHtml('<script>alert(1)</script> [x](javascript:alert(1))');
    expect(html).not.toContain('<script>');
    expect(html).toContain('&lt;script&gt;');
    // The unsafe URL survives only as literal text, never as a link.
    expect(html).toContain('[x](javascript:alert(1))');
    expect(html).not.toContain('<a');
    expect(html).not.toContain('href=');
  });

  test('inline mode flattens blocks into one span-safe run', async () => {
    const html = await renderHtml('First.\n\n- one\n- two', true);
    expect(html).not.toContain('<p');
    expect(html).not.toContain('<ul');
    expect(html).toContain('First.');
    expect(html).toContain('one · two');
  });
});
