import { h, type VNode, type VNodeChild } from 'vue';

/**
 * Markdown-lite for plugin copy: `**bold**`, `*italic*`, `` `code` ``,
 * `[label](https://…)` links, bare http(s) URLs, paragraphs, and `- ` lists.
 *
 * The parser builds an AST of plain strings and the renderer turns it into
 * VNodes, so markup can never smuggle HTML through: `<script>`, event
 * handlers, and `javascript:` URLs render as inert text. Manifest strings
 * stay data — this is the only formatting plugin copy ever receives.
 */

export type MarkdownSpan = {
  text: string;
  bold?: boolean;
  italic?: boolean;
  code?: boolean;
  href?: string;
};

export type MarkdownBlock =
  | { kind: 'paragraph'; spans: MarkdownSpan[] }
  | { kind: 'list'; items: MarkdownSpan[][] };

const MAX_SOURCE_CHARS = 20_000;
const MAX_BLOCKS = 200;

const URL_PATTERN = /https?:\/\/[^\s<>"'`]+/g;

function isSafeHref(url: string): boolean {
  return /^https?:\/\/\S+$/i.test(url.trim());
}

function autolink(text: string, base: Omit<MarkdownSpan, 'text'>): MarkdownSpan[] {
  const spans: MarkdownSpan[] = [];
  let cursor = 0;
  URL_PATTERN.lastIndex = 0;
  for (;;) {
    URL_PATTERN.lastIndex = cursor;
    const found = URL_PATTERN.exec(text);
    if (!found || found.index < cursor) break;
    if (found.index > cursor) spans.push({ ...base, text: text.slice(cursor, found.index) });
    let url = found[0];
    // Trailing punctuation belongs to the sentence, not the link.
    const trailing = url.match(/[.,;:!?)\]]+$/);
    let suffix = '';
    if (trailing) {
      suffix = trailing[0];
      url = url.slice(0, -suffix.length);
    }
    if (url.length > 0) spans.push({ ...base, text: url, href: url });
    if (suffix) spans.push({ ...base, text: suffix });
    cursor = found.index + found[0].length;
  }
  if (cursor < text.length) spans.push({ ...base, text: text.slice(cursor) });
  return spans.filter((span) => span.text.length > 0);
}

/** Inline pass: code, bold, italic, links, autolinks. One line, no blocks. */
export function parseMarkdownSpans(source: string, allowLinks = true): MarkdownSpan[] {
  const spans: MarkdownSpan[] = [];
  const pushText = (text: string, style: Omit<MarkdownSpan, 'text'> = {}): void => {
    if (!text) return;
    if (style.code || !allowLinks) {
      spans.push({ ...style, text });
      return;
    }
    spans.push(...autolink(text, style));
  };

  let literal = '';
  let index = 0;
  const flush = (style: Omit<MarkdownSpan, 'text'> = {}): void => {
    pushText(literal, style);
    literal = '';
  };

  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1];

    // Backslash escapes the next character.
    if (char === '\\' && next !== undefined) {
      literal += next;
      index += 2;
      continue;
    }
    // `code` is terminal: no formatting inside.
    if (char === '`') {
      const end = source.indexOf('`', index + 1);
      if (end > index + 1) {
        flush();
        spans.push({ text: source.slice(index + 1, end), code: true });
        index = end + 1;
        continue;
      }
    }
    // **bold** (recursive for one nested level of italic/code).
    if (char === '*' && next === '*') {
      const end = source.indexOf('**', index + 2);
      if (end > index + 2) {
        flush();
        for (const inner of parseMarkdownSpans(source.slice(index + 2, end), allowLinks)) {
          spans.push({ ...inner, bold: true });
        }
        index = end + 2;
        continue;
      }
    }
    // *italic* — a lone star or `a*b` stays literal.
    if (char === '*') {
      const end = source.indexOf('*', index + 1);
      if (end > index + 1 && source[index + 1] !== ' ' && source[end - 1] !== ' ') {
        flush();
        for (const inner of parseMarkdownSpans(source.slice(index + 1, end), allowLinks)) {
          if (!inner.code) spans.push({ ...inner, italic: true });
          else spans.push(inner);
        }
        index = end + 1;
        continue;
      }
    }
    // [label](url) — http(s) only, no nested links.
    if (allowLinks && char === '[') {
      const labelEnd = source.indexOf(']', index + 1);
      const parenOpen = labelEnd !== -1 && source[labelEnd + 1] === '(' ? labelEnd + 1 : -1;
      const parenEnd = parenOpen !== -1 ? source.indexOf(')', parenOpen + 1) : -1;
      if (labelEnd > index + 1 && parenEnd > parenOpen + 1) {
        const url = source.slice(parenOpen + 1, parenEnd).trim();
        if (isSafeHref(url)) {
          flush();
          for (const inner of parseMarkdownSpans(source.slice(index + 1, labelEnd), false)) {
            spans.push({ ...inner, href: inner.code ? undefined : url });
          }
          index = parenEnd + 1;
          continue;
        }
      }
    }
    literal += char;
    index += 1;
  }
  flush();
  return spans;
}

/** Block pass: blank-line paragraphs plus `- ` bullet lists. */
export function parseMarkdownLite(source: string): MarkdownBlock[] {
  const lines = source.replace(/\r\n?/g, '\n').slice(0, MAX_SOURCE_CHARS).split('\n');
  const blocks: MarkdownBlock[] = [];
  let paragraph: string[] = [];
  let list: string[][] = [];
  const flushParagraph = (): void => {
    if (paragraph.length > 0) {
      blocks.push({ kind: 'paragraph', spans: parseMarkdownSpans(paragraph.join(' ')) });
      paragraph = [];
    }
  };
  const flushList = (): void => {
    if (list.length > 0) {
      blocks.push({ kind: 'list', items: list.map((item) => parseMarkdownSpans(item.join(' '))) });
      list = [];
    }
  };
  for (const line of lines) {
    if (blocks.length >= MAX_BLOCKS) break;
    const trimmed = line.trim();
    if (!trimmed) {
      flushParagraph();
      flushList();
      continue;
    }
    const bullet = trimmed.match(/^[-*]\s+(.+)$/);
    if (bullet?.[1]) {
      flushParagraph();
      list.push([bullet[1]]);
      continue;
    }
    // An indented line after a bullet continues that item.
    if (list.length > 0 && /^\s+\S/.test(line)) {
      list[list.length - 1]?.push(trimmed);
      continue;
    }
    flushList();
    paragraph.push(trimmed);
  }
  flushParagraph();
  flushList();
  return blocks;
}

function renderSpan(span: MarkdownSpan, key: string): VNodeChild {
  let node: VNodeChild = span.text;
  if (span.code) node = h('code', { class: 'plg-md__code', key: `c${key}` }, node);
  if (span.bold) node = h('strong', { key: `b${key}` }, node);
  if (span.italic) node = h('em', { key: `i${key}` }, node);
  if (span.href) {
    node = h(
      'a',
      {
        class: 'plg-md__link',
        href: span.href,
        target: '_blank',
        rel: 'noopener noreferrer',
        key: `a${key}`,
      },
      node,
    );
  }
  return node;
}

function renderSpans(spans: MarkdownSpan[], keyPrefix: string): VNodeChild[] {
  return spans.map((span, index) => renderSpan(span, `${keyPrefix}${index}`));
}

/**
 * Render plugin copy. Block mode returns `<p>`/`<ul>` elements for the
 * Details panel; inline mode flattens everything into one span-safe run for
 * the clamped card snippet (lists joined with `·`).
 */
export function renderMarkdownLite(source: string, inline = false): VNode[] {
  const blocks = parseMarkdownLite(source);
  if (inline) {
    const children: VNodeChild[] = [];
    blocks.forEach((block, blockIndex) => {
      if (blockIndex > 0) children.push(' ');
      if (block.kind === 'paragraph') {
        children.push(...renderSpans(block.spans, `${blockIndex}-`));
      } else {
        block.items.forEach((item, itemIndex) => {
          if (itemIndex > 0) children.push(' · ');
          children.push(...renderSpans(item, `${blockIndex}-${itemIndex}-`));
        });
      }
    });
    return [h('span', { class: 'plg-md__inline' }, children)];
  }
  return blocks.map((block, index) =>
    block.kind === 'paragraph'
      ? h('p', { class: 'plg-md__p', key: index }, renderSpans(block.spans, `${index}-`))
      : h(
          'ul',
          { class: 'plg-md__list', key: index },
          block.items.map((item, itemIndex) =>
            h('li', { key: itemIndex }, renderSpans(item, `${index}-${itemIndex}-`)),
          ),
        ),
  );
}
