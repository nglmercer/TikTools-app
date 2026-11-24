/**
 * Pure TypeScript mirror used by contract tests and the explicitly simulated public demo.
 *
 * The contract is `crates/tiktools-core/src/services/automation/filters.rs`:
 * the real application never evaluates filters in TypeScript (the editor stores them,
 * the host runs them). Template and moderation tests still want to see real
 * rows pass or fail, so this mirror reproduces the host semantics exactly,
 * including its sharp edges, and `filter-corpus.test.ts` runs the corpus
 * shared with the Rust tests against it so the two cannot drift.
 *
 * Deliberately NOT the JavaScript idioms the old matcher used:
 * - numbers parse like Rust `f64::from_str` (no surrounding whitespace, no
 *   hex, `inf`/`nan` accepted), never `Number()`;
 * - case folding is ASCII-only (`to_ascii_lowercase`), never `toLowerCase()`;
 * - `trim()` is Unicode White_Space, which differs from JS around U+0085 and
 *   U+FEFF;
 * - paths resolve against the event envelope, not `readEventPath` (which
 *   serves templates and indexes arrays).
 * Filter values stay literal strings: nothing here expands placeholders.
 */

import type { FilterOutcome } from './types.ts';

const RUST_WHITESPACE =
  '\t-\r \\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000';
const RUST_TRIM = new RegExp(`^[${RUST_WHITESPACE}]+|[${RUST_WHITESPACE}]+$`, 'g');
/** The grammar `f64::from_str` accepts. */
const RUST_F64 = /^[+-]?(?:inf|infinity|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)$/i;

/** Mirrors `MAX_MATCH_CAPTURES` in the Rust matcher. */
export const MAX_MATCH_CAPTURES = 8;
const MAX_CAPTURE_VALUE_CHARS = 200;
const CAPTURE_ALIAS = /^[a-zA-Z][a-zA-Z0-9_]{0,31}$/;
const RESERVED_CAPTURE_KEYS = new Set(['match', 'event', 'globals', 'data', 'user']);
const CONTROL_CHARS = /\p{Cc}/gu;

/** `str::trim`: Unicode White_Space only. */
function rustTrim(text: string): string {
  return text.replace(RUST_TRIM, '');
}

/** `str::to_ascii_lowercase`: A-Z only, every other code point untouched. */
function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (letter) => String.fromCharCode(letter.charCodeAt(0) + 32));
}

/** `text.parse::<f64>().ok()`: undefined when Rust would refuse. */
function parseRustF64(text: string): number | undefined {
  if (!RUST_F64.test(text)) return undefined;
  const lower = asciiLower(text);
  if (lower.endsWith('nan')) return Number.NaN;
  if (lower.endsWith('inf') || lower.endsWith('infinity')) return lower.startsWith('-') ? -Infinity : Infinity;
  return Number(text);
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** `read_event_path`: the filter/template path language, one `event.` prefix at a time. */
function readFilterPath(event: unknown, rawPath: string): unknown {
  let path = rustTrim(rawPath);
  while (path.startsWith('{{')) path = path.slice(2);
  while (path.endsWith('}}')) path = path.slice(0, -2);
  path = rustTrim(path);
  if (path.startsWith('event.')) path = path.slice('event.'.length);
  // Sharp edge kept on purpose: a second, bare `event` prefix is stripped too.
  if (path.startsWith('event')) path = path.slice('event'.length);
  path = path.replace(/^\.+|\.+$/g, '');
  let current: unknown = event;
  for (const part of path.split('.')) {
    if (part === '') continue;
    if (!isObject(current) || !Object.hasOwn(current, part)) return undefined;
    current = current[part];
  }
  return current;
}

/** `value_to_string`: absent and null are empty, containers are compact JSON. */
function valueToString(value: unknown): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

function captureAlias(spec: Record<string, unknown>): string | undefined {
  const alias = spec.as;
  if (typeof alias !== 'string') return undefined;
  if (!CAPTURE_ALIAS.test(alias) || RESERVED_CAPTURE_KEYS.has(alias)) return undefined;
  return alias;
}

function captureKey(spec: Record<string, unknown>, index: number): string {
  return captureAlias(spec) ?? String(index);
}

/** Control characters never reach a template; the value is capped. */
function sanitizeCapture(value: string): string {
  return [...value.replace(CONTROL_CHARS, '')].slice(0, MAX_CAPTURE_VALUE_CHARS).join('');
}

function matchedOutcome(spec: Record<string, unknown>, index: number, candidate: string): FilterOutcome {
  return { matched: true, captures: { [captureKey(spec, index)]: sanitizeCapture(candidate) } };
}

const UNMATCHED: FilterOutcome = { matched: false, captures: {} };

/** String candidates: `values` wins over `value`; `in` reads `values` only. */
function candidates(spec: Record<string, unknown>, operator: string): string[] {
  const values = Array.isArray(spec.values)
    ? spec.values.filter((entry): entry is string => typeof entry === 'string')
    : undefined;
  if (operator === 'in') return values ?? [];
  if (values !== undefined) return values;
  return [typeof spec.value === 'string' ? spec.value : ''];
}

/** One operator predicate against one candidate, with host semantics. */
function predicateMatches(
  operator: string,
  left: string,
  leftNumber: number | undefined,
  candidate: string,
): boolean {
  const rightNumber = parseRustF64(candidate);
  const numeric = leftNumber !== undefined && rightNumber !== undefined && rustTrim(candidate) !== '';
  switch (operator) {
    case 'gte': return numeric && leftNumber! >= rightNumber!;
    case 'gt': return numeric && leftNumber! > rightNumber!;
    case 'lte': return numeric && leftNumber! <= rightNumber!;
    case 'lt': return numeric && leftNumber! < rightNumber!;
    case 'eq': return numeric ? leftNumber === rightNumber : left === candidate;
    case 'neq': return numeric ? leftNumber !== rightNumber : left !== candidate;
    case 'contains': return asciiLower(left).includes(asciiLower(candidate));
    case 'starts-with': return asciiLower(left).startsWith(asciiLower(candidate));
    default: return false;
  }
}

/**
 * One filter against one event with the host's semantics, including the
 * `match.*` capture of the winning value branch. `index` only names the
 * fallback capture key, never the verdict.
 */
export function filterOutcome(filter: object, event: unknown, index = 0): FilterOutcome {
  const spec = filter as Record<string, unknown>;
  const raw = readFilterPath(event, typeof spec.path === 'string' ? spec.path : '');
  const left = valueToString(raw);
  const leftNumber = parseRustF64(left);
  const operator = typeof spec.operator === 'string' ? spec.operator : 'eq';

  if (operator === 'is-true') {
    return { matched: raw === true || left === 'true' || left === '1', captures: {} };
  }
  if (operator === 'is-false') {
    return { matched: raw === false || left === 'false' || left === '0' || left === '', captures: {} };
  }
  if (operator === 'in') {
    // `in` keeps its own semantics (trim + ASCII fold of the whole value)
    // and ignores `quantifier`; the first matching entry is the capture.
    const target = asciiLower(rustTrim(left));
    for (const candidate of candidates(spec, operator)) {
      if (asciiLower(rustTrim(candidate)) === target) return matchedOutcome(spec, index, candidate);
    }
    return UNMATCHED;
  }

  const list = candidates(spec, operator);
  if (list.length === 0) return UNMATCHED;
  // `neq` with a list means «none of»: `any` is normalized to `all`.
  const requireAll = spec.quantifier === 'all' || operator === 'neq';
  if (requireAll) {
    if (list.every((candidate) => predicateMatches(operator, left, leftNumber, candidate))) {
      return matchedOutcome(spec, index, list[0]!);
    }
    return UNMATCHED;
  }
  for (const candidate of list) {
    if (predicateMatches(operator, left, leftNumber, candidate)) {
      return matchedOutcome(spec, index, candidate);
    }
  }
  return UNMATCHED;
}

/** One filter against one event, with the host's semantics. */
export function matchesFilter(filter: object, event: unknown): boolean {
  return filterOutcome(filter, event).matched;
}

/** Mirrors `MAX_ANYOF_GROUPS`/`MAX_ANYOF_GROUP_FILTERS` in the Rust matcher. */
export const MAX_ANYOF_GROUPS = 4;
export const MAX_ANYOF_GROUP_FILTERS = 12;

function mergeCaptures(into: Record<string, string>, from: Record<string, string>): void {
  for (const [key, value] of Object.entries(from)) {
    if (Object.keys(into).length >= MAX_MATCH_CAPTURES) break;
    if (!Object.hasOwn(into, key)) into[key] = value;
  }
}

/**
 * An event record's `filters` combined by `matchMode` (`and` by default,
 * `or` fires on the first passing filter), then `anyOf`: a flat list of
 * alternative groups whose filters AND internally. The first group whose
 * filters all pass contributes its captures; `anyOf` absent or empty is
 * `true`. Accepts a `{ filters, matchMode, anyOf }` record or a bare filters array.
 */
export function recordOutcome(record: unknown, event: unknown): FilterOutcome {
  const spec = Array.isArray(record) || !isObject(record)
    ? { filters: record }
    : (record as Record<string, unknown>);
  const captures: Record<string, string> = {};
  const requireAll = spec.matchMode !== 'or';
  if (Array.isArray(spec.filters)) {
    if (requireAll) {
      for (const [index, filter] of spec.filters.entries()) {
        const outcome = filterOutcome(filter as object, event, index);
        if (!outcome.matched) return UNMATCHED;
        mergeCaptures(captures, outcome.captures);
      }
    } else {
      // Global OR: the first passing filter wins and contributes its captures.
      let winner: Record<string, string> | null = null;
      for (const [index, filter] of spec.filters.entries()) {
        const outcome = filterOutcome(filter as object, event, index);
        if (outcome.matched) {
          winner = outcome.captures;
          break;
        }
      }
      if (spec.filters.length > 0) {
        if (winner === null) return UNMATCHED;
        mergeCaptures(captures, winner);
      }
    }
  }
  const groups = Array.isArray(spec.anyOf) ? spec.anyOf.slice(0, MAX_ANYOF_GROUPS) : [];
  if (groups.length > 0) {
    let winner: Record<string, string> | null = null;
    for (const group of groups) {
      if (!Array.isArray(group)) continue;
      const groupCaptures: Record<string, string> = {};
      let groupMatched = true;
      for (const [index, filter] of group.slice(0, MAX_ANYOF_GROUP_FILTERS).entries()) {
        const outcome = filterOutcome(filter as object, event, index);
        if (!outcome.matched) {
          groupMatched = false;
          break;
        }
        mergeCaptures(groupCaptures, outcome.captures);
      }
      if (groupMatched) {
        winner = groupCaptures;
        break;
      }
    }
    if (winner === null) return UNMATCHED;
    mergeCaptures(captures, winner);
  }
  return { matched: true, captures };
}

/** An event record's `filters`: AND over every row; absent or non-array always passes. */
export function matchesAllFilters(filters: unknown, event: unknown): boolean {
  return recordOutcome(filters, event).matched;
}
