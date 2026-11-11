import {
  jsonChildren,
  resolveJsonValue,
  type JsonContainer,
  type JsonToken,
} from './json-tokens.ts';

/**
 * Pure model for the collapsible tree view: a flat list of visible rows derived from a value and the set
 * of expanded node ids. Ids are index paths ("$", "$/0", "$/0/3") so they are cheap and stable for the
 * same value. Everything is bounded (depth, children per node, total rows).
 */

export interface TreeOptions {
  /** Ids of expanded containers. */
  expanded: ReadonlySet<string>;
  /** Containers deeper than this are shown as a non-expandable summary. Default 8. */
  maxDepth?: number;
  /** Children rendered per container before a "more" row. Default 500. */
  maxChildren?: number;
  /** Total rows rendered. Default 2000. */
  maxRows?: number;
  sortKeys?: boolean;
}

export interface TreeRow {
  id: string;
  depth: number;
  /** `node` is a value (leaf or container head), `close` the closing bracket, `more` an elision marker. */
  kind: 'node' | 'close' | 'more';
  /** Property name as JSON text (quoted) or the array index; null for the root and closing rows. */
  label: string | null;
  labelKind: 'key' | 'index' | null;
  tokens: JsonToken[];
  expandable: boolean;
  open: boolean;
  array: boolean;
  /** Number of members of a container; the number of omitted members for `more` rows. */
  count: number;
}

export interface TreeModel {
  rows: TreeRow[];
  truncated: boolean;
}

export const ROOT_ID = '$';
export const DEFAULT_TREE_DEPTH = 8;
const DEFAULT_MAX_CHILDREN = 500;
const DEFAULT_MAX_ROWS = 2000;

const brackets = (array: boolean): [string, string] => (array ? ['[', ']'] : ['{', '}']);

export function buildTree(value: unknown, options: TreeOptions): TreeModel {
  const maxDepth = options.maxDepth ?? DEFAULT_TREE_DEPTH;
  const maxChildren = options.maxChildren ?? DEFAULT_MAX_CHILDREN;
  const maxRows = options.maxRows ?? DEFAULT_MAX_ROWS;
  const sortKeys = options.sortKeys ?? false;
  const rows: TreeRow[] = [];
  let truncated = false;
  const ancestors = new Set<object>();

  const full = (): boolean => {
    if (rows.length >= maxRows) { truncated = true; return true; }
    return false;
  };

  const visit = (input: unknown, id: string, depth: number, label: string | null, labelKind: TreeRow['labelKind']): void => {
    if (full()) return;
    const resolved = resolveJsonValue(input);
    if (resolved.type === 'skip') return;
    const base = { id, depth, label, labelKind } as const;
    if (resolved.type === 'leaf') {
      rows.push({ ...base, kind: 'node', tokens: [{ kind: resolved.kind, text: resolved.text }], expandable: false, open: false, array: false, count: 0 });
      return;
    }
    if (ancestors.has(resolved.source)) {
      rows.push({ ...base, kind: 'node', tokens: [{ kind: 'string', text: '"[Circular]"' }], expandable: false, open: false, array: false, count: 0 });
      return;
    }
    expandContainer(resolved, base, id, depth);
  };

  const expandContainer = (
    container: JsonContainer,
    base: { id: string; depth: number; label: string | null; labelKind: TreeRow['labelKind'] },
    id: string,
    depth: number,
  ): void => {
    const children = jsonChildren(container, sortKeys);
    const [start, end] = brackets(container.array);
    const expandable = children.length > 0 && depth < maxDepth;
    const open = expandable && options.expanded.has(id);
    const summary = `${start}${children.length}${end}`;
    if (!open) {
      rows.push({ ...base, kind: 'node', tokens: [{ kind: 'punct', text: children.length === 0 ? `${start}${end}` : summary }], expandable, open: false, array: container.array, count: children.length });
      return;
    }
    rows.push({ ...base, kind: 'node', tokens: [{ kind: 'punct', text: start }], expandable, open: true, array: container.array, count: children.length });
    ancestors.add(container.source);
    const shown = Math.min(children.length, maxChildren);
    for (let index = 0; index < shown; index += 1) {
      if (full()) break;
      const child = children[index]!;
      const childId = `${id}/${index}`;
      const label = child.key === null ? String(index) : JSON.stringify(child.key);
      const labelKind = child.key === null ? 'index' : 'key';
      const value = child.value;
      if (value.type === 'skip') continue;
      if (value.type === 'leaf') {
        rows.push({ id: childId, depth: depth + 1, kind: 'node', label, labelKind, tokens: [{ kind: value.kind, text: value.text }], expandable: false, open: false, array: false, count: 0 });
      } else if (ancestors.has(value.source)) {
        rows.push({ id: childId, depth: depth + 1, kind: 'node', label, labelKind, tokens: [{ kind: 'string', text: '"[Circular]"' }], expandable: false, open: false, array: false, count: 0 });
      } else {
        expandContainer(value, { id: childId, depth: depth + 1, label, labelKind }, childId, depth + 1);
      }
    }
    if (children.length > shown && !full()) {
      truncated = true;
      rows.push({ id: `${id}:more`, depth: depth + 1, kind: 'more', label: null, labelKind: null, tokens: [], expandable: false, open: false, array: false, count: children.length - shown });
    }
    ancestors.delete(container.source);
    if (rows.length < maxRows) {
      rows.push({ id: `${id}:close`, depth, kind: 'close', label: null, labelKind: null, tokens: [{ kind: 'punct', text: end }], expandable: false, open: false, array: container.array, count: 0 });
    } else truncated = true;
  };

  visit(value, ROOT_ID, 0, null, null);
  return { rows, truncated };
}

/** Ids of every container that can be expanded within the depth and count limits (for "expand all"). */
export function expandableIds(value: unknown, options: Pick<TreeOptions, 'maxDepth' | 'sortKeys'> & { maxNodes?: number } = {}): Set<string> {
  const maxDepth = options.maxDepth ?? DEFAULT_TREE_DEPTH;
  const maxNodes = options.maxNodes ?? 5000;
  const ids = new Set<string>();
  const ancestors = new Set<object>();
  const walk = (input: unknown, id: string, depth: number): void => {
    if (ids.size >= maxNodes || depth >= maxDepth) return;
    const resolved = resolveJsonValue(input);
    if (resolved.type !== 'container' || ancestors.has(resolved.source)) return;
    const children = jsonChildren(resolved, options.sortKeys ?? false);
    if (children.length === 0) return;
    ids.add(id);
    ancestors.add(resolved.source);
    children.forEach((child, index) => {
      if (child.value.type === 'container') walk(child.value.source, `${id}/${index}`, depth + 1);
    });
    ancestors.delete(resolved.source);
  };
  walk(value, ROOT_ID, 0);
  return ids;
}
