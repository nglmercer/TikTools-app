/**
 * Cut a release end to end: bump the version, regenerate the docs contract
 * provenance the bump invalidates, verify, commit, push both remotes and tag.
 *
 * The manual sequence this replaces already failed in practice: the bump
 * touches every crate manifest, and the docs provenance hashes those files,
 * so forgetting the regeneration breaks the post-release site deploy. Ordering
 * and remote sync are not negotiable — the release workflow refuses to publish
 * unless the tag target exists in the release repository — so they live in
 * exactly one place here.
 *
 * Usage: bun scripts/task.ts release cut v4.5.3
 */
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dir, '..');

const BRANCH = 'remake';
/** Remote whose actions run the release workflow (the fork). */
const ACTIONS_REMOTE = process.env.TIKTOOLS_ACTIONS_REMOTE ?? 'fork';
/** Canonical release repository remote the assets publish to. */
const RELEASE_REMOTE = process.env.TIKTOOLS_RELEASE_REMOTE ?? 'nglmercer';

function fail(message: string): never {
  console.error(`release cut failed: ${message}`);
  process.exit(1);
}

async function run(command: string[], options: { capture?: boolean } = {}): Promise<string> {
  const process = Bun.spawn(command, {
    cwd: repositoryRoot,
    stdout: options.capture ? 'pipe' : 'inherit',
    stderr: options.capture ? 'pipe' : 'inherit',
  });
  const code = await process.exited;
  const out = options.capture ? await new Response(process.stdout).text() : '';
  if (code !== 0) {
    const err = options.capture ? await new Response(process.stderr).text() : '';
    fail(`${command.join(' ')} exited ${code}${err ? `\n${err.slice(-1500)}` : ''}`);
  }
  return out.trim();
}

const version = process.argv.slice(2).find((arg) => !arg.startsWith('-'))?.replace(/^v/, '');
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  fail('expected exactly one semver argument, e.g. `bun scripts/task.ts release cut 4.5.3`');
}
const tag = `v${version}`;

// A halfway release commit is worse than none: require a clean tree up front.
if ((await run(['git', 'status', '--porcelain'], { capture: true })) !== '') {
  fail('working tree is not clean; commit or stash everything first');
}
const branch = await run(['git', 'rev-parse', '--abbrev-ref', 'HEAD'], { capture: true });
if (branch !== BRANCH) fail(`expected to be on ${BRANCH}, not ${branch}`);
if (await run(['git', 'tag', '--list', tag], { capture: true })) fail(`tag ${tag} already exists locally`);

console.log(`== bump ${version}`);
await run(['bun', 'scripts/bump-version.ts', version]);

console.log('== regenerate contract provenance (the bump invalidates it)');
await run(['bun', 'docs/scripts/regenerate-control.ts']);

console.log('== verify version consistency');
await run(['bun', 'scripts/check-version.ts']);

console.log('== commit and push both remotes');
await run(['git', 'add', 'Cargo.toml', 'Cargo.lock', 'Packager.toml', 'package.json', 'crates', 'docs/contracts']);
if ((await run(['git', 'status', '--porcelain'], { capture: true })) === '') {
  fail('version bump produced no changes; already at this version?');
}
await run(['git', 'commit', '-m', `chore: release v${version}`]);
await run(['git', 'push', ACTIONS_REMOTE, BRANCH]);
await run(['git', 'push', RELEASE_REMOTE, BRANCH]);

const head = await run(['git', 'rev-parse', 'HEAD'], { capture: true });
for (const remote of [ACTIONS_REMOTE, RELEASE_REMOTE]) {
  const remoteSha = await run(['git', 'ls-remote', remote, `refs/heads/${BRANCH}`], { capture: true });
  if (!remoteSha.startsWith(head)) fail(`${remote}/${BRANCH} is not at ${head.slice(0, 8)} after push — refusing to tag`);
}

console.log(`== tag ${tag} and push to ${ACTIONS_REMOTE}`);
await run(['git', 'tag', '-a', tag, '-m', `Release ${tag}`]);
await run(['git', 'push', ACTIONS_REMOTE, tag]);

console.log(`Release ${tag} cutting: the release workflow is running on ${ACTIONS_REMOTE}.`);
console.log('Following the pipeline — Ctrl+C detaches safely at any time, CI continues.\n');
await run(['bun', 'scripts/release-watch.ts']);
