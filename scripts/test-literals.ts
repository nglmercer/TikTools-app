/**
 * Read-only report of identifier-like string literals that tests repeat. It drives the shared-constants
 * refactor: `bun scripts/test-literals.ts [--threshold=4] [--dir=tests/unit] [--fail]`.
 *
 * Counted: dotted method/topic/kind names ('cloud.status'), kebab/colon ids and status words.
 * Ignored: import specifiers, lines that already import from tests/shared, and ALLOWED below.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const args = new Map(process.argv.slice(2).map((arg) => { const [key, value = 'true'] = arg.replace(/^--/, '').split('='); return [key, value] as const; }));
const threshold = Number(args.get('threshold') ?? 4);
const base = args.get('dir') ?? 'tests';
const failOnFindings = args.has('fail');

/** Literals that are fine to repeat: framework vocabulary, DOM names, language codes. */
const ALLOWED = new Set([
  'disabled', 'template', 'button', 'dialog', 'link', 'click', 'input', 'change', 'checked', 'value', 'text',
  'aria-label', 'aria-expanded', 'aria-pressed', 'aria-current', 'aria-checked', 'aria-disabled', 'aria-hidden', 'class', 'title', 'href', 'type',
  'string', 'number', 'boolean', 'object', 'function', 'undefined', 'true', 'false', 'null',
  'en', 'es', 'pt', 'fr', 'zh', 'ja', 'dark', 'light', 'chromium', 'visible', 'hidden', 'none', 'auto',
  'id', 'name', 'kind', 'key', 'label', 'error', 'warn', 'info', 'log',
]);
const IMPORT_LINE = /^\s*(import|export)\b.*\bfrom\b|^\s*import\s+['"]|require\(|vi\.mock\(|import\(/;
const LITERAL = /'([a-z][a-z0-9]*(?:[.:_-][a-z0-9]+)+)'/g;

async function* walk(directory: string): AsyncGenerator<string> {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) { if (entry.name !== 'node_modules' && entry.name !== 'test-results' && !entry.name.endsWith('-snapshots')) yield* walk(path); }
    else if (entry.name.endsWith('.ts') && !path.includes(`${join('tests', 'shared')}`)) yield path;
  }
}

const counts = new Map<string, { total: number; files: Set<string> }>();
for await (const file of walk(resolve(root, base))) {
  const lines = (await readFile(file, 'utf8')).split('\n');
  for (const line of lines) {
    if (IMPORT_LINE.test(line)) continue;
    for (const match of line.matchAll(LITERAL)) {
      const value = match[1]!;
      if (ALLOWED.has(value)) continue;
      const entry = counts.get(value) ?? { total: 0, files: new Set<string>() };
      entry.total += 1;
      entry.files.add(file.slice(root.length + 1));
      counts.set(value, entry);
    }
  }
}

const findings = [...counts].filter(([, entry]) => entry.total >= threshold && entry.files.size > 1).sort((a, b) => b[1].total - a[1].total);
for (const [value, entry] of findings.slice(0, Number(args.get('top') ?? 60))) {
  console.log(`${String(entry.total).padStart(4)}  ${String(entry.files.size).padStart(3)} files  '${value}'`);
}
console.log(`\n${findings.length} literal(s) repeated ${threshold}+ times across files under ${base}.`);
if (failOnFindings && findings.length > 0) process.exit(1);
