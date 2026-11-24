/** Build once, then run Rust tests in parallel processes.
 *
 * Preferred path: `cargo nextest run` gives every test its own process, so
 * PID-scoped `TIKTOOLS_HOME` fixtures isolate themselves and the scheduler
 * work-steals across the pool. Test binaries also build with reduced
 * debuginfo (`[profile.test]`), which keeps per-test exec cheap.
 *
 * Fallback (no cargo-nextest on PATH, or TIKTOOLS_RUST_TEST_SHARDS=0):
 * Cargo runs test binaries one after another, and the workspace guard uses
 * `--test-threads=1` because several test homes are process-global. Two
 * escapes are safe:
 *   - binaries whose fixtures use per-fixture state may run their own cases
 *     in parallel (validated allowlist below);
 *   - large binaries can be partitioned into `--exact` shards; each shard is
 *     a fresh process with its own PID-scoped `TIKTOOLS_HOME`.
 */
import { dirname, relative, resolve } from 'node:path';
import { partitionTestNames } from './lib/test-shards.ts';

const root = resolve(import.meta.dir, '..');
// Test processes are mostly sleep/I/O bound, so oversubscribing the core
// count still wins; TIKTOOLS_RUST_TEST_POOL tunes it per machine.
const poolSize = Math.max(1, Math.min(16, Number(process.env.TIKTOOLS_RUST_TEST_POOL ?? '8') || 8));
const shardSize = Math.max(1, Math.min(16, Number(process.env.TIKTOOLS_RUST_TEST_SHARDS ?? '8') || 8));
const shardThreshold = Math.max(1, Number(process.env.TIKTOOLS_RUST_TEST_SHARD_THRESHOLD ?? '200') || 200);
const filters = process.env.TIKTOOLS_RUST_TEST_FILTER?.split(',').map((part) => part.trim()).filter(Boolean);
// TIKTOOLS_RUST_TEST_SHARDS=0 forces the legacy sharded runner (e.g. on
// machines without cargo-nextest, or to A/B the two schedulers).
const preferNextest = process.env.TIKTOOLS_RUST_TEST_SHARDS !== '0';

async function nextestAvailable(): Promise<boolean> {
  const probe = Bun.spawn(['cargo', 'nextest', '--version'], { cwd: root, stdout: 'ignore', stderr: 'ignore' });
  return (await probe.exited) === 0;
}

async function runNextest(): Promise<number> {
  const args = ['nextest', 'run', '--workspace', '--locked', '-j', String(poolSize)];
  if (filters?.length) {
    // Positional filters substring-match binary display names and test
    // names. Emit underscore/dash variants so `tiktools_core` still reaches
    // the `tiktools-core` binary; extra alternatives only widen the OR set.
    const variants = new Set<string>();
    for (const part of filters) {
      variants.add(part);
      variants.add(part.replaceAll('_', '-'));
      variants.add(part.replaceAll('-', '_'));
    }
    args.push(...variants);
  }
  const run = Bun.spawn(['node', 'scripts/cargo-with-linker.mjs', ...args], { cwd: root, stdout: 'inherit', stderr: 'inherit' });
  const code = await run.exited;
  if (code !== 0 || filters?.length) return code;
  // Nextest does not run doc tests; keep them serial as before.
  const docs = Bun.spawn(['node', 'scripts/cargo-with-linker.mjs', 'test', '--workspace', '--locked', '--doc', '--', '--test-threads=1'], {
    cwd: root,
    stdout: 'inherit',
    stderr: 'inherit',
  });
  return (await docs.exited) === 0 ? code : 1;
}

if (preferNextest && (await nextestAvailable())) {
  console.log(`Running Rust tests via cargo-nextest (pool=${poolSize}).`);
  process.exit(await runNextest());
}

// Targets whose tests use per-fixture state instead of process-global env can
// safely run their own cases in parallel. Each entry was validated with a
// small incremental run before being added. Keyed by source path so same-named
// targets from other crates (which may use set_var) stay serial.
const DEFAULT_PARALLEL_TARGETS = [
  'crates/tiktools-control-api/tests/automation_effects.rs',
  'crates/tiktools-control-api/tests/automation_manifest.rs',
  'crates/tiktools-control-api/tests/automation_provenance.rs',
  'crates/tiktools-control-api/tests/automation_versions.rs',
  'crates/tiktools-control-api/tests/owner_adoption.rs',
  'crates/tiktools-control-api/tests/owner_approval.rs',
  'crates/tiktools-control-api/tests/owner_definition.rs',
  'crates/tiktools-control-api/tests/owner_library.rs',
  'crates/tiktools-control-api/tests/remote_competitions.rs',
  'crates/tiktools-control-api/tests/remote_points_commands.rs',
  // Three independent CLI invocations with per-test isolated homes.
  'crates/tiktools-cli/tests/globals.rs',
];
const parallelTargets = new Set([
  ...DEFAULT_PARALLEL_TARGETS,
  ...(process.env.TIKTOOLS_RUST_TEST_PARALLEL?.split(',').map((part) => part.trim()).filter(Boolean) ?? []),
]);
const cargoArgs = ['test', '--workspace', '--locked', '--no-run', '--message-format=json'];

interface TestTarget { executable: string; cwd: string; name: string; key: string }
interface TestJob { target: TestTarget; names: string[] }

async function readChild(child: ReturnType<typeof Bun.spawn>): Promise<{ out: string; err: string; code: number }> {
  const [out, err, code] = await Promise.all([
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  return { out, err, code };
}

const build = Bun.spawn(['node', 'scripts/cargo-with-linker.mjs', ...cargoArgs], { cwd: root, stdout: 'pipe', stderr: 'inherit' });
const built = await readChild(build);
if (built.code !== 0) process.exit(built.code);

const targets: TestTarget[] = [];
for (const line of built.out.split('\n')) {
  if (!line.trim()) continue;
  let artifact: Record<string, unknown>;
  try { artifact = JSON.parse(line); } catch { continue; }
  if (artifact.reason !== 'compiler-artifact' || !artifact.profile || typeof artifact.profile !== 'object') continue;
  if (!(artifact.profile as Record<string, unknown>).test || typeof artifact.executable !== 'string') continue;
  const target = artifact.target as Record<string, unknown> | undefined;
  const manifest = artifact.manifest_path;
  if (!target || typeof target.name !== 'string' || typeof manifest !== 'string' || typeof target.src_path !== 'string') continue;
  targets.push({ executable: artifact.executable, cwd: dirname(manifest), name: target.name, key: relative(root, target.src_path) });
}

const selected = filters?.length ? targets.filter((target) => filters.some((part) => target.name.includes(part) || target.key.includes(part))) : targets;
const jobs: TestJob[] = [];
for (const target of selected) {
  if (parallelTargets.has(target.key)) {
    // Safe to run all cases in this process with 4 threads; no sharding.
    jobs.push({ target, names: [] });
    continue;
  }
  const listing = Bun.spawn([target.executable, '--list'], { cwd: target.cwd, stdout: 'pipe', stderr: 'pipe' });
  const listed = await readChild(listing);
  if (listed.code !== 0 || listed.out.trim().length === 0) {
    jobs.push({ target, names: [] });
    continue;
  }
  const names = listed.out.split('\n').map((line) => line.replace(/: test\s*$/, '')).filter((line) => line.trim().length > 0);
  if (names.length < shardThreshold) {
    jobs.push({ target, names: [] });
    continue;
  }
  for (const shard of partitionTestNames(names, shardSize)) {
    jobs.push({ target, names: shard });
  }
}

// Longest job first: big shards overlap with the many small binaries instead
// of starving them at the end of the queue.
jobs.sort((left, right) => right.names.length - left.names.length);
console.log(`Running ${jobs.length} test processes from ${selected.length} binaries (pool=${poolSize}, shards=${shardSize}).`);
let cursor = 0;
const failures: string[] = [];
async function worker(): Promise<void> {
  while (cursor < jobs.length) {
    const job = jobs[cursor++]!;
    const started = Date.now();
    const threads = parallelTargets.has(job.target.key) ? 4 : 1;
    const args = [`--test-threads=${threads}`, ...(job.names.length > 0 ? ['--exact', ...job.names] : [])];
    const child = Bun.spawn({ cmd: [job.target.executable, ...args], cwd: job.target.cwd, stdout: 'pipe', stderr: 'pipe' });
    const result = await readChild(child);
    const seconds = ((Date.now() - started) / 1000).toFixed(1);
    const label = job.names.length > 0 ? `${job.target.name}[shard ${job.names.length} tests]` : job.target.name;
    if (result.code === 0) {
      console.log(`ok   ${label} ${seconds}s (threads=${threads})`);
    } else {
      console.error(`FAIL ${label} ${seconds}s (exit ${result.code})`);
      console.error(result.out.slice(-6000));
      console.error(result.err.slice(-6000));
      failures.push(label);
    }
  }
}
await Promise.all(Array.from({ length: Math.min(poolSize, jobs.length) }, () => worker()));

// Doc tests have no binary of their own; keep them serial as before. Skip
// them for a filtered incremental run.
if (!filters?.length) {
  const docs = Bun.spawn(['node', 'scripts/cargo-with-linker.mjs', 'test', '--workspace', '--locked', '--doc', '--', '--test-threads=1'], {
    cwd: root,
    stdout: 'inherit',
    stderr: 'inherit',
  });
  if (await docs.exited !== 0) failures.push('doc tests');
}

if (failures.length > 0) {
  console.error(`Rust sharded test run failed: ${failures.join(', ')}`);
  process.exit(1);
}
console.log(`Rust sharded test run passed: ${jobs.length} processes${filters?.length ? '' : ' + doc tests'}.`);
