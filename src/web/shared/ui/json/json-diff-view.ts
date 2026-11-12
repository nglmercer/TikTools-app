import { foldRows, type CharRange, type DiffRow } from './json-diff.ts';
import type { JsonLine } from './json-tokens.ts';

export type DiffViewItem =
  | { type: 'row'; key: number; row: DiffRow }
  | { type: 'fold'; key: number; count: number };

export interface DiffViewLine {
  key: string;
  kind: 'unchanged' | 'added' | 'removed';
  number: number | null;
  marker: string;
  tokens: JsonLine;
  ranges: readonly CharRange[];
}
export type UnifiedItem = { type: 'fold'; key: number; count: number } | { type: 'line'; key: string; line: DiffViewLine; oldNumber: number | null; newNumber: number | null };

export const sideOld = (row: DiffRow): DiffViewLine | null => (row.oldTokens === null ? null : {
  key: `${row.oldNumber}`, kind: row.kind === 'unchanged' ? 'unchanged' : 'removed', number: row.oldNumber, marker: row.kind === 'unchanged' ? ' ' : '-', tokens: row.oldTokens, ranges: row.oldRanges,
});
export const sideNew = (row: DiffRow): DiffViewLine | null => (row.newTokens === null ? null : {
  key: `${row.newNumber}`, kind: row.kind === 'unchanged' ? 'unchanged' : 'added', number: row.newNumber, marker: row.kind === 'unchanged' ? ' ' : '+', tokens: row.newTokens, ranges: row.newRanges,
});

export function visibleDiffRows(rows: readonly DiffRow[], context: number, unfolded: ReadonlySet<number>): DiffViewItem[] {
  const out: DiffViewItem[] = [];
  for (const item of foldRows(rows, { context })) {
    if (item.type === 'row') out.push({ type: 'row', key: item.index, row: item.row });
    else if (unfolded.has(item.start)) {
      for (let at = item.start; at < item.start + item.count; at += 1) out.push({ type: 'row', key: at, row: rows[at]! });
    } else out.push({ type: 'fold', key: item.start, count: item.count });
  }
  return out;
}

export function unifiedDiffRows(items: readonly DiffViewItem[]): UnifiedItem[] {
  const out: UnifiedItem[] = [];
  for (const item of items) {
    if (item.type === 'fold') { out.push(item); continue; }
    const { row } = item;
    if (row.kind === 'unchanged') {
      out.push({ type: 'line', key: `u${item.key}`, line: sideNew(row)!, oldNumber: row.oldNumber, newNumber: row.newNumber });
      continue;
    }
    const removed = sideOld(row);
    const added = sideNew(row);
    if (removed) out.push({ type: 'line', key: `r${item.key}`, line: removed, oldNumber: row.oldNumber, newNumber: null });
    if (added) out.push({ type: 'line', key: `a${item.key}`, line: added, oldNumber: null, newNumber: row.newNumber });
  }
  return out;
}
