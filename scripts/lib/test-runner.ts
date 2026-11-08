/** Targeted test selection for `bun run test`.
 *
 * Pure parsing/dispatch lives here so `scripts/lib/test-runner.test.ts`
 * can cover it without spawning. Execution reuses the existing runners
 * (Cargo via `scripts/cargo-with-linker.mjs`, Bun, Vitest, Playwright)
 * and never falls back to "run everything" on invalid input.
 */

import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import { resolvePluginsRepository } from './external-plugins.ts';

export const TEST_USAGE =
  'Usage: bun run test [area] [target] [filter] [options]\n' +
  'Areas: rust <crate> [filter], web [filter], bun <area> [filter], plugin <name> [filter], e2e [filter].\n' +
  'Bun/plugin filters match test names; web/e2e filters match files; rust filters match test paths.\n' +
  'Options: --features <list> (rust only), --watch (web only), --changed (default/web/bun/plugin).\n' +
  'Examples: bun run test | bun run test rust tiktools-core batched_likes | bun run test web live-batch | bun run test e2e.';

export const FAST_BUN_ROOTS = ['src/automation', 'src/plugin-ui', 'src/shared', 'src/widgets', 'scripts', 'packages'];

/** Shorthand `bun <area>` directories (repo-relative). */
export const BUN_AREA_DIRS: Record<string, string> = {
  automation: 'src/automation',
  'plugin-ui': 'src/plugin-ui',
  shared: 'src/shared',
  widgets: 'src/widgets',
  scripts: 'scripts',
  packages: 'packages',
};

/**
 * `bun test` treats a bare positional as a path *substring* filter, so
 * `packages` would also pick up `src/web/.../packages-store.test.ts` (a
 * Vitest-only file). A `./` prefix makes Bun resolve it as a real path.
 * Pure.
 */
export function bunPathArg(dir: string): string {
  return dir.startsWith('./') || dir.startsWith('/') ? dir : `./${dir}`;
}

/** Bun ORs positional filters, so a narrowing filter must be a test-name pattern. */
function bunNameFilter(filter: string | undefined): string[] {
  return filter ? ['--test-name-pattern', filter] : [];
}

const SUPPORTED_AREAS = new Set(['rust', 'web', 'bun', 'plugin', 'e2e']);
const SAFE_TOKEN = /^[A-Za-z0-9][A-Za-z0-9_./-]*$/;
const SAFE_FEATURE = /^[A-Za-z0-9_-]+$/;
const SAFE_PLUGIN = /^[a-z0-9][a-z0-9._-]{1,127}$/i;

export type ParsedTestRequest =
  | { area: 'default'; watch: false; changed: boolean }
  | { area: 'rust'; crate: string; filter?: string; features?: string; watch: false; changed: false }
  | { area: 'web'; filter?: string; watch: boolean; changed: boolean }
  | { area: 'bun'; target: string; filter?: string; watch: false; changed: boolean }
  | { area: 'plugin'; plugin: string; filter?: string; watch: false; changed: boolean }
  | { area: 'e2e'; filter?: string; extraArgs: string[]; watch: false; changed: false };

function fail(message: string): never {
  throw new Error(`${message}\n${TEST_USAGE}`);
}

function assertSafeToken(value: string, label: string): void {
  if (!SAFE_TOKEN.test(value) || value.includes('..')) {
    fail(`invalid ${label}: ${JSON.stringify(value)}`);
  }
}

function assertSafePlugin(value: string): void {
  if (!SAFE_PLUGIN.test(value)) fail(`invalid plugin name: ${JSON.stringify(value)}`);
}

export function parseFeatureList(raw: string): string {
  const features = raw.trim();
  if (!features) fail('`--features` needs a comma-separated value (got an empty string)');
  for (const part of features.split(',')) {
    if (!SAFE_FEATURE.test(part.trim())) fail(`invalid feature: ${JSON.stringify(part)}`);
  }
  return features
    .split(',')
    .map((part) => part.trim())
    .join(',');
}

/** Parse raw `test ...` args. Pure: no filesystem or cargo access. */
export function parseTestArgs(args: string[]): ParsedTestRequest {
  let features: string | undefined;
  let watch = false;
  let changed = false;
  const positionals: string[] = [];
  const e2ePassthrough: string[] = [];

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === '--watch') {
      watch = true;
    } else if (arg === '--changed') {
      changed = true;
    } else if (arg === '--features') {
      const next = args[++i];
      if (next === undefined) fail('`--features` needs a value, e.g. `--features native-tiktok`');
      features = parseFeatureList(next);
    } else if (arg.startsWith('--features=')) {
      features = parseFeatureList(arg.slice('--features='.length));
    } else if (arg.startsWith('--') && positionals[0] !== 'e2e') {
      fail(`unsupported flag: ${JSON.stringify(arg)}`);
    } else if (positionals[0] === 'e2e' && arg.startsWith('--')) {
      e2ePassthrough.push(arg);
    } else if (arg.startsWith('-')) {
      fail(`unsupported flag: ${JSON.stringify(arg)}`);
    } else {
      positionals.push(arg);
    }
  }

  // `test --changed` (flags only) targets the fast unit default.
  if (positionals.length === 0) {
    if (features !== undefined) fail('`--features` needs a `rust <crate>` target');
    if (watch) fail('`--watch` needs a `web [filter]` target');
    return { area: 'default', watch: false, changed };
  }

  const [area, second, third, ...rest] = positionals;
  if (!SUPPORTED_AREAS.has(area!)) fail(`unknown test area: ${JSON.stringify(area)}`);
  if (rest.length > 0) fail(`too many arguments: ${JSON.stringify(positionals)}`);

  if (features !== undefined && area !== 'rust') fail('`--features` only applies to `rust <crate>`');
  if (watch && area !== 'web') fail('`--watch` only applies to `web [filter]`');
  if (changed && (area === 'rust' || area === 'e2e')) {
    fail(`\`--changed\` does not apply to \`${area}\``);
  }

  switch (area) {
    case 'rust': {
      if (!second) fail('`test rust` needs a crate, e.g. `test rust tiktools-core`');
      if (second.startsWith('-')) fail(`invalid crate: ${JSON.stringify(second)}`);
      if (third !== undefined) assertSafeToken(third, 'filter');
      return { area: 'rust', crate: second, filter: third, features, watch: false, changed: false };
    }
    case 'web': {
      if (third !== undefined) fail('`test web` takes at most one filter');
      if (second !== undefined) assertSafeToken(second, 'filter');
      return { area: 'web', filter: second, watch, changed };
    }
    case 'bun': {
      if (!second) fail('`test bun` needs an area, e.g. `test bun automation`');
      if (third !== undefined) assertSafeToken(third, 'filter');
      assertSafeToken(second, 'target');
      return { area: 'bun', target: second, filter: third, watch: false, changed };
    }
    case 'plugin': {
      if (!second) fail('`test plugin` needs a name, e.g. `test plugin sonicboom` (runs in the external plugin checkout)');
      if (third !== undefined) assertSafeToken(third, 'filter');
      assertSafePlugin(second);
      return { area: 'plugin', plugin: second, filter: third, watch: false, changed };
    }
    case 'e2e': {
      // Extra positionals beyond one filter are playwright passthrough only
      // when they look like flags is too clever; keep one optional filter and
      // let `--`-style args collected above pass through verbatim.
      void e2ePassthrough;
      const extraFromPositionals: string[] = [];
      // `third` here would be a second positional (rest already rejected
      // longer lists, so third is at most one extra positional).
      if (second !== undefined) assertSafeToken(second, 'filter');
      if (third !== undefined) assertSafeToken(third, 'filter');
      if (third !== undefined) extraFromPositionals.push(third);
      return {
        area: 'e2e',
        filter: second,
        extraArgs: [...extraFromPositionals, ...e2ePassthrough],
        watch: false,
        changed: false,
      };
    }
    default:
      fail(`unknown test area: ${JSON.stringify(area)}`);
  }
}

/** Extract workspace package names from `cargo metadata` JSON. Pure. */
export function parseCargoMetadata(jsonText: string): string[] {
  let parsed: { packages?: Array<{ name?: unknown }> };
  try {
    parsed = JSON.parse(jsonText) as typeof parsed;
  } catch {
    throw new Error('could not parse `cargo metadata` output');
  }
  const names = (parsed.packages ?? [])
    .map((pkg) => pkg?.name)
    .filter((name): name is string => typeof name === 'string' && name.length > 0);
  if (!names.length) throw new Error('`cargo metadata` reported no workspace crates');
  return [...new Set(names)].sort();
}

export function validateRustCrate(crate: string, workspaceCrates: string[]): void {
  if (!workspaceCrates.includes(crate)) {
    fail(`unknown Rust crate: ${JSON.stringify(crate)} (workspace: ${workspaceCrates.join(', ')})`);
  }
}

/** True when `cargo test -- --list` output contains the requested filter. Pure. */
export function rustFilterMatchesList(listOutput: string, filter: string): boolean {
  return listOutput.split('\n').some((line) => line.includes(filter) && line.includes(': test'));
}

/** Map a `bun <target>` shorthand to a repo-relative directory. Pure. */
export function bunTargetDir(target: string): string {
  const mapped = BUN_AREA_DIRS[target];
  if (mapped) return mapped;
  // Allow an explicit repo-relative path (e.g. `src/automation/behavior`):
  // the caller verifies it exists before spawning.
  if (target.includes('/') || target.includes('.')) return target;
  fail(`unknown bun test target: ${JSON.stringify(target)} (try ${Object.keys(BUN_AREA_DIRS).join(', ')})`);
}

/**
 * Candidate bun test roots for a plugin in the external plugin checkout
 * (`pluginsRepository`). Pure; existence is checked by the caller.
 */
export function pluginTestRoots(plugin: string): string[] {
  assertSafePlugin(plugin);
  return [`plugins/${plugin}/shared`, `plugins/${plugin}/ui/src`, `plugins/${plugin}/tests`];
}

/** Resolve workspace crate names via `cargo metadata`. Reuses the normal toolchain. */
export async function getWorkspaceCrateNames(): Promise<string[]> {
  const child = Bun.spawn(['cargo', 'metadata', '--no-deps', '--format-version', '1'], {
    stdout: 'pipe',
    stderr: 'pipe',
  });
  const [out, err, code] = await Promise.all([
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  if (code !== 0) throw new Error(`could not read workspace metadata (exit ${code ?? 'signal'}): ${err.trim()}`);
  return parseCargoMetadata(out);
}

async function pathExists(root: string, relative: string): Promise<boolean> {
  if (relative.includes('..')) return false;
  const info = await stat(join(root, relative)).catch(() => null);
  return info !== null;
}

export type TestDeps = {
  root?: string;
  workspaceCrates?: string[];
  pluginRootsExist?: (roots: string[]) => Promise<string[]>;
  pluginsRepository?: string;
  run?: (cmd: string[]) => Promise<void>;
  verifyRustFilter?: (crate: string, features: string | undefined, filter: string) => Promise<void>;
};

/** Build spawn argv for a parsed request. Validates crate names against metadata. */
export function buildTestCommands(
  request: ParsedTestRequest,
  ctx: { workspaceCrates?: string[]; pluginRoots?: string[]; pluginsRepository?: string },
): string[][] {
  const bun = process.execPath;
  const changedFlag = (request.area !== 'rust' && request.area !== 'e2e' && request.changed) ? ['--changed'] : [];
  switch (request.area) {
    case 'default':
      return [
        [bun, 'test', ...changedFlag, ...FAST_BUN_ROOTS.map(bunPathArg)],
        [bun, 'x', '--no-install', 'vitest', 'run', ...(request.changed ? ['--changed'] : [])],
      ];
    case 'rust': {
      if (!ctx.workspaceCrates) throw new Error('workspace crate list is required for rust targets');
      validateRustCrate(request.crate, ctx.workspaceCrates);
      const cmd = [bun, 'scripts/cargo-with-linker.mjs', 'test', '-p', request.crate, '--locked'];
      if (request.features) cmd.push('--features', request.features);
      if (request.filter) cmd.push(request.filter);
      return [cmd];
    }
    case 'web': {
      if (request.watch) return [[bun, 'x', '--no-install', 'vitest', ...(request.filter ? [request.filter] : []), '--watch']];
      return [
        [bun, 'x', '--no-install', 'vitest', 'run', ...(request.filter ? [request.filter] : []), ...(request.changed ? ['--changed'] : [])],
      ];
    }
    case 'bun': {
      const dir = bunTargetDir(request.target);
      return [[bun, 'test', ...changedFlag, bunPathArg(dir), ...bunNameFilter(request.filter)]];
    }
    case 'plugin': {
      const roots = ctx.pluginRoots ?? pluginTestRoots(request.plugin);
      if (!roots.length) fail(`plugin has no bun test roots: ${JSON.stringify(request.plugin)}`);
      const cwd = ctx.pluginsRepository ? ['--cwd', ctx.pluginsRepository] : [];
      return [[bun, ...cwd, 'test', ...changedFlag, ...roots.map(bunPathArg), ...bunNameFilter(request.filter)]];
    }
    case 'e2e':
      return [
        [bun, 'x', '--no-install', 'playwright', 'test', ...(request.filter ? [request.filter] : []), ...request.extraArgs],
      ];
  }
}

/** Fail when a filtered Rust run would pass with 0 tests (mirrors CI). */
export async function defaultVerifyRustFilter(
  root: string,
  crate: string,
  features: string | undefined,
  filter: string,
): Promise<void> {
  const bun = process.execPath;
  const listArgs = [bun, 'scripts/cargo-with-linker.mjs', 'test', '-p', crate, '--locked'];
  if (features) listArgs.push('--features', features);
  listArgs.push('--', '--list');
  const child = Bun.spawn(listArgs, { cwd: root, stdout: 'pipe', stderr: 'inherit', env: process.env });
  const output = await new Response(child.stdout).text();
  const code = await child.exited;
  if (code !== 0) throw new Error(`could not list Rust tests for ${crate} (exit ${code ?? 'signal'})`);
  if (!rustFilterMatchesList(output, filter)) {
    throw new Error(`no Rust tests match ${JSON.stringify(filter)} in crate ${JSON.stringify(crate)}`);
  }
}

async function defaultRun(root: string, cmd: string[]): Promise<void> {
  console.log(`\n> ${cmd.join(' ')}`);
  const child = Bun.spawn(cmd, { cwd: root, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit', env: process.env });
  const code = await child.exited;
  if (code !== 0) throw new Error(`command failed (exit ${code ?? 'signal'}): ${cmd.join(' ')}`);
}

/** Parse, validate, and run a `test ...` invocation. Never falls back silently. */
export async function runTargetedTests(rawArgs: string[], deps: TestDeps = {}): Promise<void> {
  const root = deps.root ?? join(import.meta.dir, '..', '..');
  const request = parseTestArgs(rawArgs);
  const run = deps.run ?? ((cmd: string[]) => defaultRun(root, cmd));

  if (request.area === 'rust') {
    const workspaceCrates = deps.workspaceCrates ?? (await getWorkspaceCrateNames());
    validateRustCrate(request.crate, workspaceCrates);
    if (request.filter) {
      const verify =
        deps.verifyRustFilter ?? ((crate, features, filter) => defaultVerifyRustFilter(root, crate, features, filter));
      await verify(request.crate, request.features, request.filter);
    }
    for (const cmd of buildTestCommands(request, { workspaceCrates })) await run(cmd);
    return;
  }

  if (request.area === 'bun') {
    const dir = bunTargetDir(request.target);
    if (!(await pathExists(root, dir))) {
      fail(`bun test target not found: ${JSON.stringify(dir)}`);
    }
  }

  if (request.area === 'web' && request.filter && (request.filter.includes('/') || request.filter.endsWith('.ts'))) {
    if (!(await pathExists(root, request.filter)) && !(await pathExists(root, `src/web/${request.filter}`))) {
      fail(`web test target not found: ${JSON.stringify(request.filter)}`);
    }
  }

  let pluginRoots: string[] | undefined;
  let resolvedPluginsRepository: string | undefined;
  if (request.area === 'plugin') {
    const pluginsRepository = deps.pluginsRepository ?? (await resolvePluginsRepository(root));
    if (!pluginsRepository) {
      fail('`test plugin` needs the external plugin checkout: clone plugins-example-tiktools beside TikTools or set TIKTOOLS_PLUGINS_REPO');
    }
    resolvedPluginsRepository = pluginsRepository;
    const candidates = pluginTestRoots(request.plugin);
    if (deps.pluginRootsExist) {
      pluginRoots = await deps.pluginRootsExist(candidates);
    } else {
      pluginRoots = [];
      for (const candidate of candidates) {
        if (await pathExists(pluginsRepository, candidate)) pluginRoots.push(candidate);
      }
    }
    if (!pluginRoots.length) fail(`plugin has no bun test roots: ${JSON.stringify(request.plugin)}`);
  }

  const workspaceCrates = deps.workspaceCrates;
  for (const cmd of buildTestCommands(request, { workspaceCrates, pluginRoots, pluginsRepository: resolvedPluginsRepository })) await run(cmd);
}
