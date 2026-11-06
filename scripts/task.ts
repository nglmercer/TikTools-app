import { readdir, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { assertWidgetBundle } from './lib/gateway-widgets.ts';
import { runTargetedTests } from './lib/test-runner.ts';

const root = join(import.meta.dir, '..');
const bun = process.execPath;
const isWindows = process.platform === 'win32';

const NAMED_BATCHED_TEST =
  'e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates';

const USAGE =
  'Usage: bun scripts/task.ts <start|dev|build|lint|ci|test> [options]\n' +
  'Public scripts (package.json): start (desktop dev), dev (headless dev), build, lint, ci, test.\n' +
  'Targeted tests: `test` (fast units), `test rust <crate> [filter] [--features <list>]`,\n' +
  '`test web [filter] [--watch|--changed]`, `test bun <area> [filter]`,\n' +
  '`test plugin <name> [filter]`, `test e2e [filter]`, `test --changed`.\n' +
  'Internal steps used by CI and guides: frontend-lint, frontend-test, frontend-build,\n' +
  'rust-lint, rust <cargo args>, widgets, plugins <args>, e2e [args], package,\n' +
  'version [check|bump <v>], contracts [args], serve [args], packaged, prepare-plugins.';

// Every spawned child is tracked so SIGINT/SIGTERM terminate the whole
// launcher tree instead of orphaning Vite, the desktop host, or cargo.
const children = new Set<ReturnType<typeof Bun.spawn>>();
for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => {
    process.exitCode = signal === 'SIGINT' ? 130 : 143;
    for (const child of [...children]) {
      try {
        child.kill(signal);
      } catch {
        // Already exited; nothing to stop.
      }
    }
  });
}

async function run(...cmd: string[]): Promise<void> {
  if (cmd.length === 0) throw new Error('refusing to run an empty command');
  console.log(`\n> ${cmd.join(' ')}`);
  const child = Bun.spawn(cmd, {
    cwd: root,
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
    env: process.env,
  });
  children.add(child);
  try {
    const code = await child.exited;
    // A forwarded signal already set the canonical 130/143 exit code: stop
    // immediately instead of reporting the killed child as a plain failure.
    if (process.exitCode === 130 || process.exitCode === 143) process.exit(process.exitCode);
    if (code !== 0) throw new Error(`command failed (exit ${code ?? 'signal'}): ${cmd.join(' ')}`);
  } finally {
    children.delete(child);
  }
}

const script = (name: string, ...args: string[]) => run(bun, `scripts/${name}`, ...args);
const tool = (name: string, ...args: string[]) => run(bun, 'x', '--no-install', name, ...args);
const cargo = (...args: string[]) => run(bun, 'scripts/cargo-with-linker.mjs', ...args);

function needNoArgs(command: string, args: string[]): void {
  if (args.length > 0) {
    throw new Error(
      `\`bun scripts/task.ts ${command}\` takes no arguments (got: ${args.join(' ')}).\n${USAGE}`,
    );
  }
}

async function assertFile(relative: string, hint: string): Promise<void> {
  const exists = await stat(join(root, relative)).then(
    (info) => info.isFile(),
    () => false,
  );
  if (!exists) throw new Error(`expected artifact missing: ${relative} — ${hint}`);
}

async function widgets(): Promise<void> {
  const dirs = await readdir(join(root, 'src/widgets'), { withFileTypes: true });
  const configs: string[] = [];
  for (const dir of dirs)
    if (dir.isDirectory() && (await Bun.file(join(root, 'src/widgets', dir.name, 'vite.config.ts')).exists()))
      configs.push(dir.name);
  if (!configs.length) throw new Error('No widget build configurations found');
  for (const name of configs.sort()) await tool('vite', 'build', '--config', `src/widgets/${name}/vite.config.ts`);
  // Fail loudly when a widget bundle is absent instead of letting the
  // gateway or the widget suites serve a 404.
  await assertWidgetBundle(join(root, 'dist', 'widgets'));
}

async function frontendLint(): Promise<void> {
  await tool('eslint', 'src', 'scripts', 'vite.config.ts', 'vitest.config.ts', '--max-warnings=0');
  await tool('vue-tsc', '--noEmit');
  await tool('eslint', 'packages', '--max-warnings=0');
  await tool('vue-tsc', '--noEmit', '-p', 'packages/plugin-sdk/tsconfig.json');
  await tool('vue-tsc', '--noEmit', '-p', 'packages/plugin-ui/tsconfig.json');
}

async function rustLint(): Promise<void> {
  await cargo('fmt', '--all', '--', '--check');
  await cargo('clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings');
}

async function tests(): Promise<void> {
  // One invocation per area so an unmatched pattern fails loudly instead
  // of hiding behind a sibling area that still matches.
  await run(bun, 'test', 'src/automation', 'src/plugin-ui', 'src/shared', 'src/widgets', 'scripts');
  // Plugin packages live in the external plugins-example-tiktools
  // repository and run their own suites there.
  // Browser specs (tests/e2e, widget/gateway .e2e.ts) need
  // built assets and Playwright browsers; run them explicitly with
  // `bun scripts/task.ts e2e ...` after `bun run build`.
  await tool('vitest', 'run');
  await tool('vitest', 'run', '--config', 'vitest.widgets.config.ts');
}

/** Fail when a required Rust regression test no longer exists: a bare
 * filtered `cargo test <name>` would otherwise pass with 0 tests run. */
async function verifyRustTestExists(pkg: string, features: string, name: string): Promise<void> {
  console.log(`\n> cargo test -p ${pkg} --features ${features} -- --list (verifying ${name})`);
  const child = Bun.spawn(
    [bun, 'scripts/cargo-with-linker.mjs', 'test', '-p', pkg, '--locked', '--features', features, '--lib', '--', '--list'],
    { cwd: root, stdout: 'pipe', stderr: 'inherit', env: process.env },
  );
  const output = await new Response(child.stdout).text();
  const code = await child.exited;
  if (code !== 0) throw new Error(`could not list Rust tests for ${pkg} (exit ${code ?? 'signal'})`);
  if (!output.split('\n').some((line) => line.includes(`${name}: test`))) {
    throw new Error(
      `required Rust regression test missing: ${name} (package ${pkg}, features ${features})`,
    );
  }
  console.log(`Found required Rust test: ${name}`);
}

/** Sandbox control API verification (mirrors the CI rust job): builds the
 * CLI and runs `api verify --sandbox` against an isolated runtime. */
async function controlApiVerify(): Promise<void> {
  await cargo('build', '-p', 'tiktools-cli', '--locked');
  // The debug CLI is what developers and CI verify against; the release
  // desktop binary is asserted separately in build().
  const cli = isWindows ? 'target/debug/tiktools.exe' : 'target/debug/tiktools';
  await assertFile(cli, 'rebuild the CLI first');
  await run(join(root, cli), 'api', 'verify', '--sandbox');
}

async function build(): Promise<void> {
  // Widget bundles are TikTools-owned OBS widgets (served by the external
  // Event Gateway plugin); build them explicitly first.
  await widgets();
  await tool('vite', 'build');
  await assertFile('dist/web/index.html', 'run `bun run build` again');
  // Plugins are not part of the application build: they live in the
  // external plugins-example-tiktools repository and install separately.
  await cargo('build', '-p', 'tiktools-desktop', '--release', '--locked');
  const desktopBinary = isWindows ? 'target/release/tiktools-desktop.exe' : 'target/release/tiktools-desktop';
  await assertFile(desktopBinary, 'rebuild the desktop release binary first');
  if (isWindows) {
    await run('pwsh', '-NoProfile', '-File', 'scripts/check-windows-pe.ps1', `target/release/tiktools-desktop.exe`);
  }
}

async function ci(): Promise<void> {
  await script('generate-contracts.ts', '--check');
  await script('check-tiktok-signer-revision.ts');
  await run(bun, 'scripts/check-universal.mjs');
  await frontendLint();
  await rustLint();
  await tests();
  await cargo('test', '--workspace', '--locked');
  await verifyRustTestExists('tiktools-core', 'native-tiktok', NAMED_BATCHED_TEST);
  await cargo('test', '-p', 'tiktools-core', '--locked', '--features', 'native-tiktok', '--lib', NAMED_BATCHED_TEST);
  await cargo('test', '-p', 'tiktools-cli', '--features', 'local-web', '--locked');
  await controlApiVerify();
  // MSRV and the Windows GUI-subsystem checks stay platform/toolchain
  // specific: they run in the dedicated CI jobs, not in the local gate.
  await build();
}

async function versionCommand(args: string[]): Promise<void> {
  if (args.length === 0 || args[0] === 'check') {
    if (args.length > 1) throw new Error(`\`version check\` takes no arguments.\n${USAGE}`);
    await script('check-version.ts');
  } else if (args[0] === 'bump') {
    await script('bump-version.ts', ...args.slice(1));
  } else {
    throw new Error(`unknown version subcommand: ${args[0]} (expected check or bump).\n${USAGE}`);
  }
}

const [command, ...args] = process.argv.slice(2);
try {
  switch (command) {
    case 'start':
      needNoArgs('start', args);
      await script('start-dev.ts');
      break;
    case 'dev':
      needNoArgs('dev', args);
      process.env.TIKTOOLS_SKIP_DEV_PLUGINS = '1';
      await script('start-web-dev.ts');
      break;
    case 'build':
      needNoArgs('build', args);
      await build();
      break;
    case 'lint':
      needNoArgs('lint', args);
      await frontendLint();
      await rustLint();
      break;
    case 'ci':
      needNoArgs('ci', args);
      await ci();
      break;
    case 'test':
      await runTargetedTests(args);
      break;
    case 'frontend-lint':
      needNoArgs('frontend-lint', args);
      await frontendLint();
      break;
    case 'frontend-test':
      needNoArgs('frontend-test', args);
      await tests();
      break;
    case 'frontend-build':
      await tool('vite', 'build', ...args);
      break;
    case 'rust-lint':
      needNoArgs('rust-lint', args);
      await rustLint();
      break;
    case 'rust':
      if (!args.length) throw new Error(`\`bun scripts/task.ts rust\` needs cargo arguments.\n${USAGE}`);
      await cargo(...args);
      break;
    case 'widgets':
      needNoArgs('widgets', args);
      await widgets();
      break;
    case 'plugins':
      await script('build-plugin.ts', ...args);
      break;
    case 'e2e':
      await tool('playwright', ...args.length ? args : ['test']);
      break;
    case 'package':
      needNoArgs('package', args);
      await script('package-release.ts');
      break;
    case 'version':
      await versionCommand(args);
      break;
    case 'contracts':
      await script('generate-contracts.ts', ...args);
      break;
    case 'serve':
      await tool('vite', '--host', '127.0.0.1', ...args);
      break;
    case 'packaged':
      needNoArgs('packaged', args);
      await script('start-packaged.ts');
      break;
    case 'prepare-plugins':
      needNoArgs('prepare-plugins', args);
      await script('prepare-dev-plugins.ts');
      break;
    default:
      console.error(command ? `Unknown command: ${command}\n${USAGE}` : USAGE);
      process.exitCode = 2;
  }
} catch (error) {
  console.error(`task ${command ?? '(none)'} failed: ${error instanceof Error ? error.message : error}`);
  process.exitCode = 1;
}
