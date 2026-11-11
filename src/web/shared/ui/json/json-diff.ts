import {
  jsonChildren,
  lineText,
  resolveJsonValue,
  type Resolved,
  tokenizeJson,
  type JsonLine,
  type JsonToken,
  type TokenizeOptions,
} from './json-tokens.ts';

/**
 * Line-oriented diff of two JSON values, built on a Myers O(ND) shortest-edit-script over their
 * pretty-printed lines. Pure and dependency-free so a three-version review (base / local / cloud) is just two
 * `diffJson` calls. Absent sides (`undefined`) print no lines, so a created or deleted document diffs as
 * all-added / all-removed.
 */

export type DiffRowKind = 'unchanged' | 'added' | 'removed' | 'changed';

/** Half-open character range `[start, end)` inside a line's text. */
export type CharRange = readonly [start: number, end: number];

export interface DiffRow {
  kind: DiffRowKind;
  /** 1-based line number in the old document, `null` when the line only exists in the new one. */
  oldNumber: number | null;
  newNumber: number | null;
  oldTokens: JsonLine | null;
  newTokens: JsonLine | null;
  oldText: string | null;
  newText: string | null;
  /** Word-level changes inside a `changed` pair (empty when the lines are too dissimilar to be worth marking). */
  oldRanges: readonly CharRange[];
  newRanges: readonly CharRange[];
}

export interface DiffStats {
  added: number;
  removed: number;
  changed: number;
  unchanged: number;
}

export interface JsonDiff {
  rows: DiffRow[];
  stats: DiffStats;
  /** True when either side was cut by the tokenizer limits, so the diff covers only the shown part. */
  truncated: boolean;
  /** True when both sides print exactly the same lines. */
  identical: boolean;
}

export interface DiffJsonOptions extends TokenizeOptions {
  /** Largest edit distance searched before the differing middle block is reported as replaced. Default 2500. */
  maxEdits?: number;
}

export type EditOp = 'equal' | 'delete' | 'insert';

const DEFAULT_MAX_EDITS = 2500;

function intern(a: readonly string[], b: readonly string[]): [Int32Array, Int32Array] {
  const ids = new Map<string, number>();
  const map = (list: readonly string[]): Int32Array => {
    const out = new Int32Array(list.length);
    for (let index = 0; index < list.length; index += 1) {
      const item = list[index]!;
      let id = ids.get(item);
      if (id === undefined) { id = ids.size; ids.set(item, id); }
      out[index] = id;
    }
    return out;
  };
  return [map(a), map(b)];
}

/** Myers forward search; returns `null` when more than `maxEdits` edits are needed. */
function myers(a: Int32Array, b: Int32Array, from: number, toA: number, toB: number, maxEdits: number): EditOp[] | null {
  const n = toA - from;
  const m = toB - from;
  if (n === 0) return new Array<EditOp>(m).fill('insert');
  if (m === 0) return new Array<EditOp>(n).fill('delete');
  const limit = Math.min(n + m, maxEdits);
  const offset = limit + 1;
  const v = new Int32Array(2 * limit + 3);
  const trace: Int32Array[] = [];
  let found = -1;
  for (let d = 0; d <= limit && found < 0; d += 1) {
    for (let k = -d; k <= d; k += 2) {
      let x = k === -d || (k !== d && v[offset + k - 1]! < v[offset + k + 1]!) ? v[offset + k + 1]! : v[offset + k - 1]! + 1;
      let y = x - k;
      while (x < n && y < m && a[from + x] === b[from + y]) { x += 1; y += 1; }
      v[offset + k] = x;
      if (x >= n && y >= m) { found = d; break; }
    }
    trace.push(v.slice(offset - d, offset + d + 1));
  }
  if (found < 0) return null;
  const ops: EditOp[] = [];
  let x = n;
  let y = m;
  for (let d = found; d > 0; d -= 1) {
    const previous = trace[d - 1]!;
    const base = d - 1;
    const k = x - y;
    const down = k === -d || (k !== d && previous[k - 1 + base]! < previous[k + 1 + base]!);
    const prevK = down ? k + 1 : k - 1;
    const prevX = previous[prevK + base]!;
    const prevY = prevX - prevK;
    const midX = down ? prevX : prevX + 1;
    const midY = midX - k;
    while (x > midX && y > midY) { ops.push('equal'); x -= 1; y -= 1; }
    ops.push(down ? 'insert' : 'delete');
    x = prevX;
    y = prevY;
  }
  while (x > 0 && y > 0) { ops.push('equal'); x -= 1; y -= 1; }
  return ops.reverse();
}

/** Shortest edit script between two sequences of strings (common prefix/suffix are trimmed first). */
export function diffSequences(before: readonly string[], after: readonly string[], maxEdits = DEFAULT_MAX_EDITS): EditOp[] {
  const [a, b] = intern(before, after);
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start += 1;
  let endA = a.length;
  let endB = b.length;
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) { endA -= 1; endB -= 1; }
  const middleA = a.subarray(start, endA);
  const middleB = b.subarray(start, endB);
  const middle = myers(middleA, middleB, 0, middleA.length, middleB.length, maxEdits)
    ?? [...new Array<EditOp>(middleA.length).fill('delete'), ...new Array<EditOp>(middleB.length).fill('insert')];
  return [
    ...new Array<EditOp>(start).fill('equal'),
    ...middle,
    ...new Array<EditOp>(a.length - endA).fill('equal'),
  ];
}

const WORD = /[\p{L}\p{N}_]+|\s+|[^\s\p{L}\p{N}_]/gu;
const MAX_WORD_DIFF_CHARS = 2000;
const MIN_SIMILARITY = 0.4;

function mergeRanges(ranges: Array<[number, number]>): CharRange[] {
  const merged: Array<[number, number]> = [];
  for (const range of ranges) {
    const last = merged[merged.length - 1];
    if (last && last[1] === range[0]) last[1] = range[1];
    else merged.push([range[0], range[1]]);
  }
  return merged;
}

/** Word-level changed ranges for a pair of lines, or empty ranges when they share too little to be meaningful. */
export function wordRanges(oldText: string, newText: string): { oldRanges: CharRange[]; newRanges: CharRange[] } {
  const none = { oldRanges: [], newRanges: [] };
  if (oldText === newText || oldText.length > MAX_WORD_DIFF_CHARS || newText.length > MAX_WORD_DIFF_CHARS) return none;
  const oldWords = oldText.match(WORD) ?? [];
  const newWords = newText.match(WORD) ?? [];
  const ops = diffSequences(oldWords, newWords, 400);
  const oldRanges: Array<[number, number]> = [];
  const newRanges: Array<[number, number]> = [];
  let oldPos = 0;
  let newPos = 0;
  let oldIndex = 0;
  let newIndex = 0;
  let common = 0;
  for (const op of ops) {
    if (op === 'equal') {
      const word = oldWords[oldIndex]!;
      if (word.trim() !== '') common += word.length;
      oldPos += word.length; newPos += newWords[newIndex]!.length; oldIndex += 1; newIndex += 1;
    } else if (op === 'delete') {
      const word = oldWords[oldIndex]!;
      if (word.trim() !== '') oldRanges.push([oldPos, oldPos + word.length]);
      oldPos += word.length; oldIndex += 1;
    } else {
      const word = newWords[newIndex]!;
      if (word.trim() !== '') newRanges.push([newPos, newPos + word.length]);
      newPos += word.length; newIndex += 1;
    }
  }
  const total = oldText.replace(/\s+/g, '').length + newText.replace(/\s+/g, '').length;
  if (total === 0 || (2 * common) / total < MIN_SIMILARITY) return none;
  return { oldRanges: mergeRanges(oldRanges), newRanges: mergeRanges(newRanges) };
}

export interface DiffSegment {
  kind: JsonToken['kind'];
  text: string;
  /** True for the parts of a changed line that differ from the other side. */
  changed: boolean;
}

/** Splits a line's syntax tokens at the boundaries of the changed ranges so both can be styled together. */
export function segmentLine(tokens: JsonLine, ranges: readonly CharRange[]): DiffSegment[] {
  if (ranges.length === 0) return tokens.map((token) => ({ kind: token.kind, text: token.text, changed: false }));
  const out: DiffSegment[] = [];
  let offset = 0;
  let rangeIndex = 0;
  for (const token of tokens) {
    const end = offset + token.text.length;
    let cursor = offset;
    while (cursor < end) {
      while (rangeIndex < ranges.length && ranges[rangeIndex]![1] <= cursor) rangeIndex += 1;
      const range = ranges[rangeIndex];
      const inside = range !== undefined && range[0] <= cursor;
      const stop = Math.min(end, inside ? range[1] : (range?.[0] ?? end));
      out.push({ kind: token.kind, text: token.text.slice(cursor - offset, stop - offset), changed: inside });
      cursor = stop;
    }
    offset = end;
  }
  return out;
}

/** Diff already-tokenized documents (one `JsonLine` per line). */
export function diffLines(before: readonly JsonLine[], after: readonly JsonLine[], maxEdits = DEFAULT_MAX_EDITS): { rows: DiffRow[]; stats: DiffStats } {
  const oldText = before.map(lineText);
  const newText = after.map(lineText);
  const ops = diffSequences(oldText, newText, maxEdits);
  const rows: DiffRow[] = [];
  const stats: DiffStats = { added: 0, removed: 0, changed: 0, unchanged: 0 };
  let i = 0;
  let j = 0;
  let cursor = 0;
  const row = (kind: DiffRowKind, oi: number | null, nj: number | null, ranges?: ReturnType<typeof wordRanges>): void => {
    rows.push({
      kind,
      oldNumber: oi === null ? null : oi + 1,
      newNumber: nj === null ? null : nj + 1,
      oldTokens: oi === null ? null : before[oi]!,
      newTokens: nj === null ? null : after[nj]!,
      oldText: oi === null ? null : oldText[oi]!,
      newText: nj === null ? null : newText[nj]!,
      oldRanges: ranges?.oldRanges ?? [],
      newRanges: ranges?.newRanges ?? [],
    });
  };
  while (cursor < ops.length) {
    if (ops[cursor] === 'equal') { row('unchanged', i, j); stats.unchanged += 1; i += 1; j += 1; cursor += 1; continue; }
    const removed: number[] = [];
    const added: number[] = [];
    while (cursor < ops.length && ops[cursor] !== 'equal') {
      if (ops[cursor] === 'delete') { removed.push(i); i += 1; } else { added.push(j); j += 1; }
      cursor += 1;
    }
    const pairs = Math.min(removed.length, added.length);
    for (let index = 0; index < pairs; index += 1) {
      row('changed', removed[index]!, added[index]!, wordRanges(oldText[removed[index]!]!, newText[added[index]!]!));
      stats.changed += 1;
    }
    for (let index = pairs; index < removed.length; index += 1) { row('removed', removed[index]!, null); stats.removed += 1; }
    for (let index = pairs; index < added.length; index += 1) { row('added', null, added[index]!); stats.added += 1; }
  }
  return { rows, stats };
}

/** Diff two JSON values by their pretty-printed form. Key order is significant unless `sortKeys` is set. */
export function diffJson(before: unknown, after: unknown, options: DiffJsonOptions = {}): JsonDiff {
  const { maxEdits, ...tokenizeOptions } = options;
  const left = tokenizeJson(before, tokenizeOptions);
  const right = tokenizeJson(after, tokenizeOptions);
  const { rows, stats } = diffLines(left.lines, right.lines, maxEdits);
  return {
    rows,
    stats,
    truncated: left.truncated || right.truncated,
    identical: stats.added === 0 && stats.removed === 0 && stats.changed === 0,
  };
}

export type DiffItem =
  | { type: 'row'; index: number; row: DiffRow }
  | { type: 'fold'; /** Index of the first hidden row; unique per fold. */ start: number; count: number };

export interface FoldOptions {
  /** Unchanged lines kept around each change. Default 3. */
  context?: number;
  /** Runs hiding fewer lines than this stay visible. Default 3. */
  minHidden?: number;
}

/** Collapses long runs of unchanged rows into fold markers, keeping `context` lines around every change. */
export function foldRows(rows: readonly DiffRow[], options: FoldOptions = {}): DiffItem[] {
  const context = options.context ?? 3;
  const minHidden = options.minHidden ?? 3;
  const items: DiffItem[] = [];
  let index = 0;
  while (index < rows.length) {
    if (rows[index]!.kind !== 'unchanged') { items.push({ type: 'row', index, row: rows[index]! }); index += 1; continue; }
    let end = index;
    while (end < rows.length && rows[end]!.kind === 'unchanged') end += 1;
    const leading = index === 0 ? 0 : context;
    const trailing = end === rows.length ? 0 : context;
    const hidden = end - index - leading - trailing;
    if (hidden < minHidden) {
      for (let at = index; at < end; at += 1) items.push({ type: 'row', index: at, row: rows[at]! });
    } else {
      for (let at = index; at < index + leading; at += 1) items.push({ type: 'row', index: at, row: rows[at]! });
      items.push({ type: 'fold', start: index + leading, count: hidden });
      for (let at = end - trailing; at < end; at += 1) items.push({ type: 'row', index: at, row: rows[at]! });
    }
    index = end;
  }
  return items;
}

/** Structural equality of two JSON values; object key order does not matter. */
export function jsonEqual(a: unknown, b: unknown): boolean {
  return equalResolved(resolveJsonValue(a), resolveJsonValue(b), 0);
}

function equalResolved(left: Resolved, right: Resolved, depth: number): boolean {
  if (left.type === 'skip' || right.type === 'skip') return left.type === right.type;
  if (left.type === 'leaf') return right.type === 'leaf' && left.kind === right.kind && left.text === right.text;
  if (right.type !== 'container' || left.array !== right.array) return false;
  if (left.source === right.source) return true;
  if (depth > 500) return false;
  const mine = jsonChildren(left);
  const theirs = jsonChildren(right);
  if (mine.length !== theirs.length) return false;
  if (left.array) return mine.every((child, index) => equalResolved(child.value, theirs[index]!.value, depth + 1));
  const lookup = new Map(theirs.map((child) => [child.key, child.value] as const));
  return mine.every((child) => {
    const other = lookup.get(child.key);
    return other !== undefined && equalResolved(child.value, other, depth + 1);
  });
}

export interface KeyChanges {
  added: string[];
  removed: string[];
  changed: string[];
  unchanged: string[];
  /** Set when either side is not an object, so there are no top-level keys to compare. */
  rootChanged: boolean;
}

/** Which top-level keys differ between two values (e.g. which parts of an automation record changed). */
export function changedTopLevelKeys(before: unknown, after: unknown): KeyChanges {
  const left = resolveJsonValue(before);
  const right = resolveJsonValue(after);
  const out: KeyChanges = { added: [], removed: [], changed: [], unchanged: [], rootChanged: false };
  if (left.type !== 'container' || right.type !== 'container' || left.array || right.array) {
    out.rootChanged = !jsonEqual(before, after);
    return out;
  }
  const mine = new Map(jsonChildren(left).map((child) => [child.key!, child.value] as const));
  const theirs = new Map(jsonChildren(right).map((child) => [child.key!, child.value] as const));
  for (const [key, value] of mine) {
    const other = theirs.get(key);
    if (other === undefined) out.removed.push(key);
    else if (equalResolved(value, other, 0)) out.unchanged.push(key);
    else out.changed.push(key);
  }
  for (const key of theirs.keys()) if (!mine.has(key)) out.added.push(key);
  return out;
}
