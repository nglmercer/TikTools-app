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

const RUST_WHITESPACE =
  '\\t-\\r \\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000';
const RUST_TRIM = new RegExp(`^[${RUST_WHITESPACE}]+|[${RUST_WHITESPACE}]+$`, 'g');
/** The grammar `f64::from_str` accepts. */
const RUST_F64 = /^[+-]?(?:inf|infinity|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)$/i;

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

/** One filter against one event, with the host's semantics. */
export function matchesFilter(filter: object, event: unknown): boolean {
  const spec = filter as Record<string, unknown>;
  const raw = readFilterPath(event, typeof spec.path === 'string' ? spec.path : '');
  const left = valueToString(raw);
  const right = typeof spec.value === 'string' ? spec.value : '';
  const leftNumber = parseRustF64(left);
  const rightNumber = parseRustF64(right);
  const numeric = leftNumber !== undefined && rightNumber !== undefined && rustTrim(right) !== '';
  const operator = typeof spec.operator === 'string' ? spec.operator : 'eq';

  switch (operator) {
    case 'gte': return numeric && leftNumber >= rightNumber;
    case 'gt': return numeric && leftNumber > rightNumber;
    case 'lte': return numeric && leftNumber <= rightNumber;
    case 'lt': return numeric && leftNumber < rightNumber;
    case 'eq': return numeric ? leftNumber === rightNumber : left === right;
    case 'neq': return numeric ? leftNumber !== rightNumber : left !== right;
    case 'contains': return asciiLower(left).includes(asciiLower(right));
    case 'starts-with': return asciiLower(left).startsWith(asciiLower(right));
    case 'in': {
      const values = Array.isArray(spec.values) ? spec.values : [];
      const target = asciiLower(rustTrim(left));
      return values.some((entry) => typeof entry === 'string' && asciiLower(rustTrim(entry)) === target);
    }
    case 'is-true': return raw === true || left === 'true' || left === '1';
    case 'is-false': return raw === false || left === 'false' || left === '0' || left === '';
    default: return false;
  }
}

/** An event record's `filters`: AND over every row; absent or non-array always passes. */
export function matchesAllFilters(filters: unknown, event: unknown): boolean {
  if (!Array.isArray(filters)) return true;
  return filters.every((filter) => matchesFilter(filter as object, event));
}
