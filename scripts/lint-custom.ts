#!/usr/bin/env bun
/**
 * Project-specific lint enforcements that oxlint cannot express as of 1.x
 * (no user-defined esquery selectors, no template-AST plugins). These run
 * next to `oxlint-vize` in `frontend-lint` and cost milliseconds.
 *
 * 1. `.vue` templates must not use native `<details>`/`<summary>` — the
 *    styled disclosure primitive is <BaseDisclosure>. Previously enforced
 *    through no-restricted-syntax VElement selectors in eslint.
 * 2. Tests must resolve translated text through testMessage(locale, key),
 *    never a `locale === 'x' ? 'literal'` ternary. Previously a
 *    no-restricted-syntax ConditionalExpression selector in eslint.
 * 3. Unit tests must take RPC methods, topics, event kinds, plugin ids, error codes, account states and
 *    fixture ids from tests/shared instead of retyping them (see scripts/test-literals.ts). E2E specs are
 *    exempt: page-side scripts cannot import, so their literals stay.
 *
 * If oxlint gains selector/plugin support for these (or vize grows native
 * template-element rules), this script should retire in their favor; track
 * upstream before extending it further.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import * as sharedTokens from '../tests/shared/index.ts';

const root = process.cwd();
const problems: string[] = [];

function* walk(dir: string): Generator<string> {
  for (const entry of readdirSync(dir)) {
    if (entry === 'node_modules' || entry === 'dist' || entry.startsWith('.')) continue;
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) yield* walk(path);
    else yield path;
  }
}

const lineOf = (text: string, index: number) => text.slice(0, index).split('\n').length;

// Native disclosure elements are banned in templates; a hit inside a script
// string is rare enough that flagging it is still acceptable (it should use
// the primitive too). Comments in templates with literal examples count as
// documentation, so `<details>` inside an HTML comment is ignored.
const DETAILS = /<(details|summary)[\s>/]/g;
for (const scope of ['src']) {
  for (const file of walk(join(root, scope))) {
    if (!file.endsWith('.vue')) continue;
    // Only the template region is enforced, matching the original template-AST
    // rule: mentions in script comments or documentation strings are fine.
    const source = readFileSync(file, 'utf8');
    const at = source.search(/<template[\s>]/);
    const template = source.match(/<template[\s\S]*?<\/template>/)?.[0] ?? '';
    const text = template.replace(/<!--[\s\S]*?-->/g, '');
    for (const match of text.matchAll(DETAILS)) {
      const line = at < 0 ? 1 : lineOf(source, at + match.index);
      problems.push(`${file}:${line} use <BaseDisclosure> instead of <${match[1]}>`);
    }
  }
}

// Hardcoded translated literals behind locale ternaries in tests.
const LOCALE_TERNARY = /\blocale\s*===?\s*['"][^'"]+['"]\s*\?\s*['"]/g;
for (const scope of ['tests', 'src']) {
  for (const file of walk(join(root, scope))) {
    if (!(file.includes('tests/') || file.endsWith('.test.ts'))) continue;
    if (!file.endsWith('.ts')) continue;
    const text = readFileSync(file, 'utf8');
    for (const match of text.matchAll(LOCALE_TERNARY)) {
      problems.push(`${file}:${lineOf(text, match.index)} use testMessage(locale, key) instead of a hardcoded translated string`);
    }
  }
}

// Shared test tokens must not be retyped in unit tests.
const SHARED_GROUPS = ['RPC', 'TOPIC', 'EVENT_KIND', 'PLUGIN_ID', 'ACTION_TYPE', 'ACCOUNT_STATE', 'ERROR_CODE', 'FIXTURE_ID', 'STORAGE_KEY'] as const;
const sharedValues = new Map<string, string>();
for (const group of SHARED_GROUPS) for (const [key, value] of Object.entries(sharedTokens[group])) sharedValues.set(value as string, `${group}.${key}`);
const IMPORT_OR_TYPE = /^\s*(import|export)\b.*\bfrom\b|^\s*(export\s+)?type\s|'\s*\|\s*'|`|vi\.mock\(/;
for (const file of walk(join(root, 'tests', 'unit'))) {
  if (!file.endsWith('.ts')) continue;
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((line, index) => {
    if (IMPORT_OR_TYPE.test(line)) return;
    for (const match of line.matchAll(/'([a-z][a-z0-9]*(?:[._:-][a-z0-9]+)+)'/g)) {
      const token = sharedValues.get(match[1]!);
      if (token) problems.push(`${file}:${index + 1} use ${token} from tests/shared instead of '${match[1]}'`);
    }
  });
}

if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}
console.log('custom lint checks passed');
