import { expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Regression tests for the rows <-> icons circular dependency.
 *
 * `rows.ts` needs `iconForSuggestion` and `icons.ts` needs
 * `groupForTemplatePath`; both now resolve grouping through the neutral
 * `autocomplete/groups.ts` module, which must import from neither. These
 * tests read the module sources (a runtime probe cannot see the cycle:
 * the shared bindings are hoisted function declarations) and fail if the
 * forbidden import direction returns.
 */

const dir = join(dirname(fileURLToPath(import.meta.url)), 'autocomplete');

function sourceOf(file: string): string {
  return readFileSync(join(dir, file), 'utf8');
}

/** Basenames (no extension) imported via `./name(.ts)` from one module. */
function siblingImports(file: string): string[] {
  const targets: string[] = [];
  for (const match of sourceOf(file).matchAll(/\bfrom\s+['"]\.\/([^'"]+)['"]/g)) {
    const specifier = match[1] ?? '';
    targets.push(specifier.replace(/\.ts$/, ''));
  }
  return targets;
}

function moduleFiles(): string[] {
  return readdirSync(dir).filter((entry) => entry.endsWith('.ts') && !entry.endsWith('.test.ts'));
}

test('groups.ts is neutral: it imports from neither rows nor icons', () => {
  const offenders = siblingImports('groups.ts').filter((name) => name === 'rows' || name === 'icons');
  expect(offenders).toEqual([]);
});

test('icons.ts never imports rows.ts (that edge closes the cycle)', () => {
  expect(siblingImports('icons.ts')).not.toContain('rows');
});

test('the autocomplete/ module graph is acyclic', () => {
  const files = moduleFiles().map((entry) => entry.replace(/\.ts$/, ''));
  const edges = new Map<string, string[]>();
  for (const name of files) {
    edges.set(name, siblingImports(`${name}.ts`).filter((target) => files.includes(target)));
  }
  const visiting = new Set<string>();
  const done = new Set<string>();
  const stack: string[] = [];
  const cycles: string[] = [];
  const visit = (node: string): void => {
    if (done.has(node)) return;
    if (visiting.has(node)) {
      cycles.push([...stack.slice(stack.indexOf(node)), node].join(' -> '));
      return;
    }
    visiting.add(node);
    stack.push(node);
    for (const next of edges.get(node) ?? []) visit(next);
    stack.pop();
    visiting.delete(node);
    done.add(node);
  };
  for (const name of files) visit(name);
  expect(cycles).toEqual([]);
});
