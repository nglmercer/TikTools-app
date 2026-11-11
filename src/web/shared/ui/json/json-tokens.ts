/**
 * Dependency-free JSON tokenizer and pretty-printer.
 *
 * The output is a list of lines, each a list of typed tokens, so views can colour them and diffs can
 * compare them. The concatenated text of all lines is byte-for-byte what `JSON.stringify(value, null, 2)`
 * prints (keys keep the order they were given in unless `sortKeys` is set). Work is bounded: output stops
 * at `maxLines`/`maxChars` and any single token is clipped to `maxLineChars`, with `truncated` set, so a
 * huge document can never freeze the UI.
 */

export type JsonTokenKind = 'key' | 'string' | 'number' | 'boolean' | 'null' | 'punct' | 'ws' | 'text';

export interface JsonToken {
  kind: JsonTokenKind;
  text: string;
}

export type JsonLine = readonly JsonToken[];

export interface JsonLines {
  lines: JsonLine[];
  /** True when output stopped early or a token was clipped because of the limits. */
  truncated: boolean;
  /** True when the input was raw text that is not valid JSON and is shown verbatim (`text` tokens). */
  plain: boolean;
}

export interface TokenizeOptions {
  /** Maximum number of lines produced. Default 5000. */
  maxLines?: number;
  /** Maximum number of characters produced (indentation included). Default 1,000,000. */
  maxChars?: number;
  /** Longest single line token kept before it is clipped with an ellipsis. Default 20,000. */
  maxLineChars?: number;
  /** Order object keys by code unit so equal documents print identically. Default false. */
  sortKeys?: boolean;
}

export const DEFAULT_MAX_LINES = 5000;
export const DEFAULT_MAX_CHARS = 1_000_000;
export const DEFAULT_MAX_LINE_CHARS = 20_000;
const INDENT = '  ';

export function lineText(line: JsonLine): string {
  let text = '';
  for (const token of line) text += token.text;
  return text;
}

export function linesText(lines: readonly JsonLine[]): string {
  return lines.map(lineText).join('\n');
}

export type JsonLeaf = { type: 'leaf'; kind: JsonTokenKind; text: string };
export type JsonContainer = { type: 'container'; array: boolean; source: object };
export type Resolved = JsonLeaf | JsonContainer | { type: 'skip' };

/** Applies `toJSON` and maps a value to what JSON.stringify would print (or skip). */
export function resolveJsonValue(input: unknown): Resolved {
  let value = input;
  if (value !== null && typeof value === 'object' && typeof (value as { toJSON?: unknown }).toJSON === 'function') {
    try { value = (value as { toJSON: () => unknown }).toJSON(); } catch { return { type: 'skip' }; }
  }
  switch (typeof value) {
    case 'string': return { type: 'leaf', kind: 'string', text: JSON.stringify(value) };
    case 'number': return Number.isFinite(value) ? { type: 'leaf', kind: 'number', text: JSON.stringify(value) } : { type: 'leaf', kind: 'null', text: 'null' };
    case 'bigint': return { type: 'leaf', kind: 'number', text: value.toString() };
    case 'boolean': return { type: 'leaf', kind: 'boolean', text: value ? 'true' : 'false' };
    case 'object':
      if (value === null) return { type: 'leaf', kind: 'null', text: 'null' };
      return { type: 'container', array: Array.isArray(value), source: value };
    default: return { type: 'skip' };
  }
}

export interface JsonChild {
  /** Property name for objects, `null` for array items. */
  key: string | null;
  value: Resolved;
}

/** Children as JSON.stringify would print them: object members with unsupported values are dropped, array holes become null. */
export function jsonChildren(container: JsonContainer, sortKeys = false): JsonChild[] {
  if (container.array) {
    const items = container.source as unknown[];
    const out: JsonChild[] = new Array<JsonChild>(items.length);
    for (let index = 0; index < items.length; index += 1) {
      const resolved = resolveJsonValue(items[index]);
      out[index] = { key: null, value: resolved.type === 'skip' ? { type: 'leaf', kind: 'null', text: 'null' } : resolved };
    }
    return out;
  }
  const record = container.source as Record<string, unknown>;
  const keys = Object.keys(record);
  if (sortKeys) keys.sort();
  const out: JsonChild[] = [];
  for (const key of keys) {
    const resolved = resolveJsonValue(record[key]);
    if (resolved.type !== 'skip') out.push({ key, value: resolved });
  }
  return out;
}

interface Frame {
  source: object;
  array: boolean;
  children: JsonChild[];
  index: number;
  depth: number;
}

/** Tokenize any JSON-serializable value. `undefined` (nothing to print) yields no lines. */
export function tokenizeJson(value: unknown, options: TokenizeOptions = {}): JsonLines {
  const maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
  const maxChars = options.maxChars ?? DEFAULT_MAX_CHARS;
  const maxLineChars = options.maxLineChars ?? DEFAULT_MAX_LINE_CHARS;
  const sortKeys = options.sortKeys ?? false;

  const lines: JsonToken[][] = [];
  let line: JsonToken[] = [];
  let chars = 0;
  let truncated = false;
  let stopped = false;

  const push = (kind: JsonTokenKind, text: string): void => {
    let kept = text;
    if (kept.length > maxLineChars) { kept = `${kept.slice(0, maxLineChars)}…`; truncated = true; }
    line.push({ kind, text: kept });
    chars += kept.length;
  };
  const endLine = (): void => {
    lines.push(line);
    line = [];
    chars += 1;
    if (lines.length >= maxLines || chars >= maxChars) stopped = true;
  };
  const indent = (depth: number): void => { if (depth > 0) push('ws', INDENT.repeat(depth)); };

  const root = resolveJsonValue(value);
  if (root.type === 'skip') return { lines, truncated: false, plain: false };

  const ancestors = new Set<object>();
  const stack: Frame[] = [];

  /** Prints one value on the current line (after any indentation/key). Containers open a frame. */
  const open = (resolved: Resolved, depth: number, comma: boolean): void => {
    if (resolved.type === 'skip') return;
    if (resolved.type === 'leaf') {
      push(resolved.kind, resolved.text);
      if (comma) push('punct', ',');
      endLine();
      return;
    }
    if (ancestors.has(resolved.source)) {
      push('string', '"[Circular]"');
      if (comma) push('punct', ',');
      endLine();
      return;
    }
    const children = jsonChildren(resolved, sortKeys);
    const [start, end] = resolved.array ? ['[', ']'] : ['{', '}'];
    if (children.length === 0) {
      push('punct', `${start}${end}`);
      if (comma) push('punct', ',');
      endLine();
      return;
    }
    push('punct', start);
    endLine();
    ancestors.add(resolved.source);
    stack.push({ source: resolved.source, array: resolved.array, children, index: 0, depth: depth + 1 });
  };

  open(root, 0, false);
  const closers: boolean[] = []; // comma flag a frame's closing bracket must print, parallel to `stack`
  closers.push(false);

  while (stack.length > 0 && !stopped) {
    const frame = stack[stack.length - 1]!;
    if (frame.index >= frame.children.length) {
      stack.pop();
      ancestors.delete(frame.source);
      indent(frame.depth - 1);
      push('punct', frame.array ? ']' : '}');
      if (closers.pop() === true) push('punct', ',');
      endLine();
      continue;
    }
    const child = frame.children[frame.index]!;
    frame.index += 1;
    const comma = frame.index < frame.children.length;
    indent(frame.depth);
    if (child.key !== null) {
      push('key', JSON.stringify(child.key));
      push('punct', ': ');
    }
    const before = stack.length;
    open(child.value, frame.depth, comma);
    if (stack.length > before) closers.push(comma);
  }

  if (stopped && stack.length > 0) truncated = true;
  if (line.length > 0) lines.push(line);
  return { lines, truncated, plain: false };
}

/** Raw text shown verbatim, split into lines, with the same limits as parsed JSON. */
export function plainTextLines(source: string, options: TokenizeOptions = {}): JsonLines {
  const maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
  const maxChars = options.maxChars ?? DEFAULT_MAX_CHARS;
  const maxLineChars = options.maxLineChars ?? DEFAULT_MAX_LINE_CHARS;
  const lines: JsonToken[][] = [];
  let truncated = false;
  let chars = 0;
  let from = 0;
  while (from <= source.length) {
    if (lines.length >= maxLines || chars >= maxChars) { truncated = true; break; }
    let to = source.indexOf('\n', from);
    if (to === -1) to = source.length;
    let text = source.slice(from, to);
    if (text.endsWith('\r')) text = text.slice(0, -1);
    if (text.length > maxLineChars) { text = `${text.slice(0, maxLineChars)}…`; truncated = true; }
    lines.push([{ kind: 'text', text }]);
    chars += text.length + 1;
    from = to + 1;
  }
  return { lines, truncated, plain: true };
}

/** Raw JSON text: parsed and pretty-printed when valid, otherwise shown as plain text. */
export function tokenizeJsonSource(source: string, options: TokenizeOptions = {}): JsonLines {
  let parsed: unknown;
  try { parsed = JSON.parse(source); } catch { return plainTextLines(source, options); }
  return tokenizeJson(parsed, options);
}

/** Full, untruncated pretty-printed text (what copy-to-clipboard uses). */
export function prettyJson(value: unknown, options: Pick<TokenizeOptions, 'sortKeys'> = {}): string {
  if (!options.sortKeys) {
    try {
      const text = JSON.stringify(value, null, 2);
      if (text !== undefined) return text;
    } catch { /* cyclic, bigint or hostile value: fall through to the tolerant printer */ }
  }
  return linesText(tokenizeJson(value, { ...options, maxLines: Infinity, maxChars: Infinity, maxLineChars: Infinity }).lines);
}
