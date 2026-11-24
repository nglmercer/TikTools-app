/** Build once, then run Rust test binaries in parallel processes.
 *
 * Cargo runs test binaries one after another, and the workspace guard uses
 * `--test-threads=1` because the test home is process-wide. Each test binary
 * is a separate process with its own PID-scoped home, so overlapping binaries
 * is safe while keeping each binary's tests serial.
 */
import { dirname, relative, resolve } from 'node:path';

const root = resolve(import.meta.dir, '..');
const poolSize = Math.max(1, Math.min(8, Number(process.env.TIKTOOLS_RUST_TEST_POOL ?? '4') || 4));
const filters = process.env.TIKTOOLS_RUST_TEST_FILTER?.split(',').map((part) => part.trim()).filter(Boolean);
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
];
const parallelTargets = new Set([
  ...DEFAULT_PARALLEL_TARGETS,
  ...(process.env.TIKTOOLS_RUST_TEST_PARALLEL?.split(',').map((part) => part.trim()).filter(Boolean) ?? []),
]);
const cargoArgs = ['test', '--workspace', '--locked', '--no-run', '--message-format=json'];

interface TestTarget { executable: string; cwd: string; name: string; key: string }

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
const parallelCount = selected.filter((target) => parallelTargets.has(target.key)).length;
console.log(`Running ${selected.length} Rust test binaries with ${poolSize} processes (${parallelCount} allowlisted for intra-binary parallelism).`);
let cursor = 0;
const failures: string[] = [];
async function worker(): Promise<void> {
  while (cursor < selected.length) {
    const target = selected[cursor++]!;
    const started = Date.now();
    const child = Bun.spawn({
      cmd: [target.executable, `--test-threads=${parallelTargets.has(target.key) ? 4 : 1}`],
      cwd: target.cwd,
      stdout: 'pipe',
      stderr: 'pipe',
    });
    const result = await readChild(child);
    const seconds = ((Date.now() - started) / 1000).toFixed(1);
    if (result.code === 0) {
      const threads = parallelTargets.has(target.key) ? 4 : 1;
      console.log(`ok   ${target.name} ${seconds}s (threads=${threads})`);
    } else {
      console.error(`FAIL ${target.name} ${seconds}s (exit ${result.code})`);
      console.error(result.out.slice(-6000));
      console.error(result.err.slice(-6000));
      failures.push(target.name);
    }
  }
}
await Promise.all(Array.from({ length: Math.min(poolSize, targets.length) }, () => worker()));

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
console.log(`Rust sharded test run passed: ${selected.length} binaries${filters?.length ? '' : ' + doc tests'}.`);
