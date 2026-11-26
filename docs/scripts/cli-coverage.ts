/**
 * CLI command coverage report: for every verb in the checked-in CLI help
 * contract, measure guide coverage (mentioned in English documentation prose
 * outside the generated reference) and test coverage (verb tokens exercised
 * in Rust test sources). Informational: always exits 0; adopt `--strict` once
 * the numbers mean something to gate on.
 *
 * Test coverage is heuristic — CLI tests pass verbs as separate args rather
 * than contiguous invocations, so a verb counts when its distinguishing
 * tokens co-occur in one test file. Treat it as a floor, not proof.
 */
import { readFile, readdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import type { CliHelp, CliVerb } from './cli-reference.ts';

const root = resolve(import.meta.dirname, '../..');
const help = JSON.parse(await readFile(resolve(root, 'docs/contracts/cli-help.json'), 'utf8')) as CliHelp;

async function files(directory: string, extension: RegExp): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const groups = await Promise.all(entries.filter((entry) => !entry.name.startsWith('.') && entry.name !== 'node_modules').map((entry) =>
    entry.isDirectory() ? files(join(directory, entry.name), extension) : extension.test(entry.name) ? [join(directory, entry.name)] : []));
  return groups.flat();
}

// Guide coverage: English prose, excluding the generated reference page itself.
const guideFiles = (await files(resolve(root, 'docs/content/en'), /\.md$/))
  .filter((path) => !path.endsWith('developer/cli.md'));

/** Readers see the rendered article, so coverage expands `@include` fragments
 * (one level is enough today: generated fragments never include further). */
async function expandIncludes(path: string, source: string): Promise<string> {
  const pattern = /<!--@include:\s*(\S+?)\s*-->/g;
  const matches = [...source.matchAll(pattern)];
  let expanded = source;
  for (const match of matches) {
    const target = resolve(path, '..', match[1]!);
    const fragment = await readFile(target, 'utf8').catch(() => '');
    expanded = expanded.replace(match[0], fragment);
  }
  return expanded;
}
const guides = new Map(await Promise.all(guideFiles.map(async (path) => [path, await expandIncludes(path, await readFile(path, 'utf8'))] as const)));

// Test coverage: unit tests live inside command modules, integration tests in tests/.
const testFiles = [
  ...await files(resolve(root, 'crates/tiktools-cli/src'), /\.rs$/),
  ...await files(resolve(root, 'crates/tiktools-cli/tests'), /\.rs$/),
];
const tests = new Map(await Promise.all(testFiles.map(async (path) => [path, await readFile(path, 'utf8')] as const)));

/** Minimal invocation tokens: the verb-path words. Flags stay when they are
 * part of the verb identity (`update --all`, `host --stdio`); positional
 * arguments never appear in the name, so nothing is dropped. */
function invocationTokens(domain: string, verb: CliVerb): string[] {
  const tokens = verb.name.split(' ');
  // Forms whose name repeats the domain (`host --stdio` in domain host, the
  // bare `web` serve form) invoke as `host --stdio`, not `host host --stdio`.
  if (tokens[0] === domain) tokens.shift();
  return tokens;
}

function guideHits(domain: string, verb: CliVerb): string[] {
  const escape = (token: string) => token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  // The usage line is the single source of truth for how a verb is invoked:
  // the domain grouping is organizational (`tiktools rpc` carries no `host`
  // prefix) and flag-compound forms keep their dashes (`web --print-code`).
  // Static words and flags must appear in order; positional arguments and
  // flag values become optional wildcard atoms so interleaved arguments and
  // prose mentions without arguments both count. Dynamic shapes degenerate
  // to the literal usage shape, which the generated tables carry verbatim.
  const staticShape = verb.usage.split(' [')[0]!.split(' | ')[0]!;
  const isStaticToken = (token: string) => /^[a-z0-9][a-z0-9-]*$/.test(token) || /^--[a-z]/.test(token);
  // No static verb token after the domain (`plugin <id> <command>`,
  // `rpc <method>`): the invocation shape is positional, so only the literal
  // usage shape documents it — anything weaker would match the bare domain.
  const shapeParts = staticShape.split(' ');
  if (!isStaticToken(shapeParts[1] ?? '')) {
    const literal = new RegExp(escape(staticShape));
    return [...guides.entries()].filter(([, text]) => literal.test(text)).map(([path]) => path);
  }
  const atoms: string[] = [];
  let previousFlag = false;
  for (const token of staticShape.split(' ')) {
    const staticToken = isStaticToken(token) && !(previousFlag && !token.startsWith('--'));
    if (atoms.length === 0) {
      atoms.push(staticToken ? escape(token) : '(?:\\S+)?');
    } else if (staticToken) {
      atoms.push('\\s+' + escape(token));
    } else {
      atoms.push('(?:\\s+\\S+)?');
    }
    previousFlag = token.startsWith('--');
  }
  const pattern = new RegExp(`(?<![a-z0-9-])${atoms.join('')}(?![a-z0-9-])`);
  return [...guides.entries()].filter(([, text]) => pattern.test(text)).map(([path]) => path);
}

function testHits(domain: string, verb: CliVerb): string[] {
  const tokens = invocationTokens(domain, verb).filter((token) => token.length > 2 && !token.startsWith('--'));
  if (!tokens.length) return [];
  return [...tests.entries()]
    .filter(([, source]) => source.includes('#[test]') && tokens.every((token) => source.includes(`"${token}"`)))
    .map(([path]) => path);
}

let totalVerbs = 0;
let totalGuide = 0;
let totalTest = 0;
const lines: string[] = [];
for (const domain of help.commands) {
  let guideCovered = 0;
  let testCovered = 0;
  const uncoveredGuide: string[] = [];
  for (const verb of domain.verbs) {
    totalVerbs += 1;
    const inGuides = guideHits(domain.domain, verb).length > 0;
    const inTests = testHits(domain.domain, verb).length > 0;
    if (inGuides) { guideCovered += 1; totalGuide += 1; } else uncoveredGuide.push(verb.name);
    if (inTests) { testCovered += 1; totalTest += 1; }
  }
  const pct = (part: number) => `${part}/${domain.verbs.length} (${Math.round((part / domain.verbs.length) * 100)}%)`;
  lines.push(`${domain.domain.padEnd(12)} guides: ${pct(guideCovered).padEnd(14)} tests: ${pct(testCovered)}${uncoveredGuide.length ? `  undocumented: ${uncoveredGuide.join(', ')}` : ''}`);
}
console.log('CLI verb coverage (heuristic; see script header)\n');
console.log(lines.join('\n'));
const pct = (part: number) => `${part}/${totalVerbs} (${Math.round((part / totalVerbs) * 100)}%)`;
console.log(`\n${'TOTAL'.padEnd(12)} guides: ${pct(totalGuide).padEnd(14)} tests: ${pct(totalTest)}`);
