/**
 * Follow a release pipeline end to end with visible phases and timeouts,
 * so a release never needs the GitHub web UI to know what is happening.
 *
 *   bun scripts/task.ts release watch [run-id]
 *
 * Without arguments it attaches to the newest release workflow run. It then
 * reports job-level transitions as they happen, waits for the live update
 * feed, follows the auto-dispatched documentation deploy, and extracts the
 * failing step's error lines itself when anything goes red. Ctrl+C is always
 * safe: the pipeline lives in CI, this only detaches the terminal.
 */
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dir, '..');

function fail(message: string): never {
  console.error(`\nrelease watch failed: ${message}`);
  process.exit(1);
}

async function gh(args: string[]): Promise<string> {
  const process = Bun.spawn(['gh', ...args], { cwd: repositoryRoot, stdout: 'pipe', stderr: 'pipe' });
  const code = await process.exited;
  const out = await new Response(process.stdout).text();
  if (code !== 0) fail(`gh ${args.join(' ')}: ${(await new Response(process.stderr).text()).slice(-400)}`);
  return out.trim();
}

function remoteRepo(remote: string): string {
  // The remotes are maintainer-local names; read the URL instead of hardcoding.
  const url = process.env[`TIKTOOLS_${remote.toUpperCase()}_REPO`];
  if (url) return url;
  throw new Error('unresolved');
}

async function repoOf(remote: string): Promise<string> {
  try { return remoteRepo(remote); } catch { /* fall through */ }
  const process = Bun.spawn(['git', 'remote', 'get-url', remote], { cwd: repositoryRoot, stdout: 'pipe', stderr: 'pipe' });
  if (await process.exited !== 0) fail(`git remote ${remote} not found`);
  const url = (await new Response(process.stdout).text()).trim();
  const match = url.match(/github\.com[:/]([^/]+\/[^/.]+)/);
  if (!match) fail(`cannot parse owner/repo from ${remote} url: ${url}`);
  return match[1]!;
}

interface Job { name: string; status: string; conclusion: string | null; started_at: string | null; steps: Array<{ name: string; conclusion: string | null; status: string }> }

function elapsed(since: number): string {
  const seconds = Math.max(0, Math.round((Date.now() - since) / 1000));
  return `${Math.floor(seconds / 60)}m${String(seconds % 60).padStart(2, '0')}s`;
}
function sinceStart(): string {
  return elapsed(startGlobal);
}

async function extractFailure(runId: string, repo: string): Promise<void> {
  console.error('\n--- failing step error lines ---');
  const process = Bun.spawn(['gh', 'run', 'view', runId, '-R', repo, '--log-failed'], { cwd: repositoryRoot, stdout: 'pipe', stderr: 'pipe' });
  const log = await new Response(process.stdout).text();
  const lines = log.split('\n').filter((line) => /error|failed|✗|✘/i.test(line) && !/0 (errors|failed)/.test(line)).map((line) => line.replace(/^.*Z /, '').trim()).filter(Boolean);
  for (const line of lines.slice(-12)) console.error(`  ${line.slice(0, 180)}`);
  if (!lines.length) console.error('  (no error lines extracted)');
  console.error(`--- full log: gh run view ${runId} -R ${repo} --log-failed ---`);
}

async function watchRun(runId: string, repo: string, label: string, timeoutMinutes: number): Promise<void> {
  console.log(`\n== ${label} (run ${runId}, timeout ${timeoutMinutes}m)`);
  const states = new Map<string, string>();
  const deadline = Date.now() + timeoutMinutes * 60_000;
  let heartbeat = 0;
  for (;;) {
    const payload = JSON.parse(await gh(['api', `repos/${repo}/actions/runs/${runId}/jobs`, '--jq', '.jobs | map({name, status, conclusion, started_at, steps: (.steps | map({name, status, conclusion}))})'])) as Job[];
    let allDone = true;
    let anyFailed: Job | undefined;
    for (const job of payload) {
      const key = job.name;
      const state = job.conclusion ?? job.status;
      if (states.get(key) !== state) {
        states.set(key, state);
        const active = !job.conclusion ? job.steps.find((step) => step.status === 'in_progress') : undefined;
        console.log(`  [${sinceStart()}] ${job.name}: ${state}${active ? ` — ${active.name}` : ''}`);
      }
      if (!job.conclusion) allDone = false;
      else if (job.conclusion !== 'success' && job.conclusion !== 'skipped') anyFailed = job;
    }
    if (anyFailed) {
      console.error(`\n  ✗ ${anyFailed.name}: ${anyFailed.conclusion} at step "${anyFailed.steps.find((s) => s.conclusion === 'failure')?.name ?? 'unknown'}"`);
      await extractFailure(runId, repo);
      fail(`${label} failed`);
    }
    if (allDone && payload.length > 0) {
      console.log(`  ✓ ${label} green`);
      return;
    }
    if (Date.now() > deadline) fail(`${label} exceeded its ${timeoutMinutes}m budget — inspect run ${runId}`);
    if (++heartbeat % 6 === 0) console.log(`  … still running (${sinceStart()} total)`);
    await Bun.sleep(20_000);
  }
}

const startGlobal = Date.now();
const actionsRepo = await repoOf(process.env.TIKTOOLS_ACTIONS_REMOTE ?? 'fork');
const releaseRepo = await repoOf(process.env.TIKTOOLS_RELEASE_REMOTE ?? 'nglmercer');

let runId = process.argv.slice(2).find((arg) => !arg.startsWith('-'));
if (!runId) {
  runId = await gh(['run', 'list', '-R', actionsRepo, '--workflow', 'release.yml', '--limit', '1', '--json', 'databaseId', '--jq', '.[0].databaseId']);
  if (!runId) fail('no release workflow runs found');
}
const meta = JSON.parse(await gh(['run', 'view', runId, '-R', actionsRepo, '--json', 'displayTitle,headBranch,status'])) as { displayTitle: string; headBranch: string; status: string };
const version = meta.headBranch.replace(/^v/, '');
console.log(`release pipeline ${meta.displayTitle} on ${meta.headBranch} — Ctrl+C detaches safely, CI continues`);

await watchRun(runId, actionsRepo, 'build + publish', 45);

// The publish job waits for the feed internally, but this confirms what users
// and the updater actually see, not what the workflow believes.
console.log('\n== update feed (timeout 10m)');
const feedUrl = `https://${releaseRepo.split('/')[0]}.github.io/${releaseRepo.split('/')[1]}/updates/stable.json`;
const feedDeadline = Date.now() + 10 * 60_000;
for (;;) {
  const response = await fetch(feedUrl).catch(() => null);
  if (response?.ok) {
    const data = await response.json() as { version?: string };
    if (data.version === version) { console.log(`  ✓ feed serving ${version}`); break; }
  }
  if (Date.now() > feedDeadline) fail(`feed ${feedUrl} not serving ${version} after 10m`);
  await Bun.sleep(20_000);
}

const siteRun = await gh(['run', 'list', '-R', actionsRepo, '--workflow', 'site.yml', '--limit', '1', '--json', 'databaseId', '--jq', '.[0].databaseId']);
await watchRun(siteRun, actionsRepo, 'documentation deploy', 20);

console.log(`\nrelease ${meta.headBranch} fully published: assets live, feed serving, docs deployed (${sinceStart()} total)`);
