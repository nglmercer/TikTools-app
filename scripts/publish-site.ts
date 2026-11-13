import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { lstat, mkdtemp, readFile, readdir, rm, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { normalizeSiteBase, siteConfig } from './lib/site-config.ts';
import { normalizeVersion } from './bump-version.ts';

export const SITE_MANIFEST = 'site-manifest.json';
const PAGES_BRANCH = 'gh-pages';
const MAX_MANIFEST_BYTES = 8 * 1024 * 1024;
const sha256 = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');

export interface SiteManifest {
  version: 1;
  sourceCommit: string;
  dirty: false;
  base: string;
  files: Record<string, string>;
}

export interface SiteArtifact {
  manifest: SiteManifest;
  manifestBytes: Uint8Array;
  files: ReadonlyMap<string, Uint8Array>;
}

/** Reading lazily avoids loading foreign assets or update-feed bytes into memory. */
export interface ExistingFile {
  mode: string;
  read(): Promise<Uint8Array>;
}
export type ExistingTree = ReadonlyMap<string, ExistingFile>;

export interface SitePlan {
  writes: Array<{ path: string; bytes: Uint8Array }>;
  deletes: string[];
  unchanged: string[];
  preservedForeignFiles: number;
  usedOwnershipSeed: boolean;
}

function protectedPath(path: string): boolean {
  const first = path.split('/')[0]?.toLowerCase();
  return first === 'updates' || first === 'cname' || first === '.nojekyll';
}

function safePath(path: string): void {
  if (!path || path.includes('\\') || path.includes(':')
      || [...path].some((character) => character.charCodeAt(0) <= 31 || character.charCodeAt(0) === 127)
      || path.split('/').some((part) => !part || part === '.' || part === '..'
        || part.toLowerCase() === '.git' || /[. ]$/.test(part))) {
    throw new Error(`Unsafe manifest path: ${JSON.stringify(path)}`);
  }
  if (protectedPath(path)) throw new Error(`Protected gh-pages path: ${path}`);
}

function filePaths(files: Record<string, string>): void {
  const names = new Set<string>();
  for (const [path, digest] of Object.entries(files)) {
    safePath(path);
    if (path.toLowerCase() === SITE_MANIFEST) throw new Error('The manifest cannot own itself in files.');
    if (!/^[a-f0-9]{64}$/.test(digest)) throw new Error(`Invalid SHA-256 for ${path}`);
    const folded = path.toLowerCase();
    if (names.has(folded)) throw new Error(`Case-colliding manifest path: ${path}`);
    names.add(folded);
  }
  for (const path of names) {
    const parts = path.split('/');
    for (let index = 1; index < parts.length; index += 1) {
      if (names.has(parts.slice(0, index).join('/'))) throw new Error(`File/directory collision: ${path}`);
    }
  }
}

export function parseSiteManifest(bytes: Uint8Array, expectedBase: string, expectedCommit?: string): SiteManifest {
  if (bytes.byteLength > MAX_MANIFEST_BYTES) throw new Error('Site manifest is too large.');
  const value: unknown = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid site manifest.');
  const data = value as Record<string, unknown>;
  if (data.version !== 1 || data.dirty !== false || typeof data.sourceCommit !== 'string'
      || !/^[a-f0-9]{40}$/.test(data.sourceCommit) || typeof data.base !== 'string'
      || normalizeSiteBase(data.base) !== data.base || data.base !== expectedBase
      || !data.files || typeof data.files !== 'object' || Array.isArray(data.files)
      || Object.values(data.files).some((digest) => typeof digest !== 'string')) {
    throw new Error('Expected a clean version-1 site manifest with the configured base and SHA-256 file hashes.');
  }
  if (expectedCommit && data.sourceCommit !== expectedCommit) throw new Error('Artifact source commit does not match this build.');
  const manifest = data as unknown as SiteManifest;
  filePaths(manifest.files);
  return manifest;
}

function verifyArtifact(artifact: SiteArtifact): void {
  const parsed = parseSiteManifest(artifact.manifestBytes, artifact.manifest.base, artifact.manifest.sourceCommit);
  if (JSON.stringify(parsed) !== JSON.stringify(artifact.manifest)) throw new Error('Artifact metadata differs from its manifest bytes.');
  if (artifact.files.size !== Object.keys(parsed.files).length) throw new Error('Artifact has unlisted or missing files.');
  for (const [path, digest] of Object.entries(parsed.files)) {
    const bytes = artifact.files.get(path);
    if (!bytes || sha256(bytes) !== digest) throw new Error(`Artifact hash mismatch or missing file: ${path}`);
  }
}

export async function loadSiteArtifact(directory: string, expectedBase: string, expectedCommit?: string): Promise<SiteArtifact> {
  const root = resolve(directory);
  const rootStat = await lstat(root);
  if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) throw new Error('Artifact root must be a real directory.');
  const files = new Map<string, Uint8Array>();
  async function visit(relative: string): Promise<void> {
    for (const entry of await readdir(join(root, relative), { withFileTypes: true })) {
      const path = relative ? `${relative}/${entry.name}` : entry.name;
      if (entry.isSymbolicLink()) throw new Error(`Artifact symlink is forbidden: ${path}`);
      if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) files.set(path, await readFile(join(root, path)));
      else throw new Error(`Artifact special file is forbidden: ${path}`);
    }
  }
  await visit('');
  const manifestBytes = files.get(SITE_MANIFEST);
  if (!manifestBytes) throw new Error(`Missing ${SITE_MANIFEST}`);
  files.delete(SITE_MANIFEST);
  const artifact = { manifest: parseSiteManifest(manifestBytes, expectedBase, expectedCommit), manifestBytes, files };
  verifyArtifact(artifact);
  return artifact;
}

async function regularBytes(file: ExistingFile | undefined, path: string): Promise<Uint8Array> {
  if (!file || !['100644', '100755'].includes(file.mode)) throw new Error(`Owned path is missing or not a regular Git blob: ${path}`);
  return file.read();
}

/** Only prior-manifest paths may be removed/overwritten. Foreign paths are never adopted implicitly. */
export async function planSiteUpdate(current: ExistingTree, artifact: SiteArtifact, ownershipSeed?: SiteManifest): Promise<SitePlan> {
  verifyArtifact(artifact);
  const previousFile = current.get(SITE_MANIFEST);
  if (previousFile && ownershipSeed) throw new Error('An ownership seed cannot replace an existing site manifest.');
  const previous = previousFile
    ? parseSiteManifest(await regularBytes(previousFile, SITE_MANIFEST), artifact.manifest.base)
    : ownershipSeed;
  if (ownershipSeed) parseSiteManifest(new TextEncoder().encode(JSON.stringify(ownershipSeed)), artifact.manifest.base);
  const owned = new Set(Object.keys(previous?.files ?? {}));
  for (const path of owned) {
    const bytes = await regularBytes(current.get(path), path);
    if (sha256(bytes) !== previous?.files[path]) throw new Error(`Previously owned path changed outside its manifest: ${path}`);
  }
  const deletes = [...owned].filter((path) => !artifact.files.has(path)).sort();
  const deleted = new Set(deletes);
  const desired = new Map(artifact.files);
  desired.set(SITE_MANIFEST, artifact.manifestBytes);
  // Git plumbing preserves symlink/gitlink modes for all foreign entries.
  // Refuse exact or parent/child collisions before touching the temporary index.
  for (const path of desired.keys()) {
    safePath(path);
    for (const existing of current.keys()) {
      if (deleted.has(existing)) continue;
      const a = path.toLowerCase();
      const b = existing.toLowerCase();
      const same = a === b;
      if (same && path === existing && (owned.has(existing) || (path === SITE_MANIFEST && previousFile))) continue;
      if (same || a.startsWith(`${b}/`) || b.startsWith(`${a}/`)) {
        throw new Error(`Refusing to overwrite or adopt foreign path: ${existing} (candidate ${path})`);
      }
    }
  }
  const writes: SitePlan['writes'] = [];
  const unchanged: string[] = [];
  for (const [path, bytes] of desired) {
    const existing = current.get(path);
    if (existing && sha256(await regularBytes(existing, path)) === sha256(bytes)) unchanged.push(path);
    else writes.push({ path, bytes });
  }
  return {
    writes: writes.sort((a, b) => a.path.localeCompare(b.path)), deletes, unchanged: unchanged.sort(),
    preservedForeignFiles: [...current.keys()].filter((path) => !owned.has(path) && path !== SITE_MANIFEST).length,
    usedOwnershipSeed: Boolean(ownershipSeed),
  };
}

export interface CasDriver<State, Prepared> {
  readCurrent(): Promise<State>;
  revision(state: State): string;
  prepare(current: State): Promise<{ commit: string | null; value: Prepared }>;
  push(commit: string, expectedHead: string): Promise<void>;
}

/** Shared optimistic-writer loop. A feed writer can supply its own checked overlay.
 * Rebuild from the latest tree after a conflict; never cherry-pick stale feed bytes.
 * The driver must use a normal fast-forward push, not force/force-with-lease.
 */
export async function reapplyWithCas<State, Prepared>(driver: CasDriver<State, Prepared>, maxAttempts = 5) {
  if (!Number.isInteger(maxAttempts) || maxAttempts < 1 || maxAttempts > 20) throw new Error('CAS attempts must be between 1 and 20.');
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const current = await driver.readCurrent();
    const head = driver.revision(current);
    const prepared = await driver.prepare(current);
    if (!prepared.commit) return { head, commit: head, attempts: attempt, changed: false, value: prepared.value };
    try {
      await driver.push(prepared.commit, head);
      return { head, commit: prepared.commit, attempts: attempt, changed: true, value: prepared.value };
    } catch (error) {
      const observed = driver.revision(await driver.readCurrent());
      // A push can succeed even when its response is lost.
      if (observed === prepared.commit) return { head, commit: prepared.commit, attempts: attempt, changed: true, value: prepared.value };
      if (observed === head) throw error; // Authentication/network errors are not conflicts.
      if (attempt === maxAttempts) throw new Error(`gh-pages changed during all ${maxAttempts} publication attempts; nothing was rolled back.`, { cause: error });
    }
  }
  throw new Error('Unreachable CAS state.');
}

function git(directory: string, args: string[], input?: Uint8Array, extraEnv: Record<string, string> = {}): Buffer {
  const result = spawnSync('git', args, {
    cwd: directory, input, maxBuffer: 128 * 1024 * 1024, timeout: 120_000,
    env: { ...process.env, GIT_TERMINAL_PROMPT: '0', ...extraEnv },
  });
  if (result.error || result.status !== 0) throw new Error(`git ${args[0]} failed: ${String(result.stderr ?? result.error ?? '').trim().slice(0, 4000)}`);
  return Buffer.from(result.stdout);
}
const gitText = (directory: string, args: string[]) => git(directory, args).toString('utf8').trim();

function gitTree(directory: string, head: string): ExistingTree {
  const tree = new Map<string, ExistingFile>();
  for (const entry of git(directory, ['ls-tree', '-r', '-z', '--full-tree', head]).toString('utf8').split('\0')) {
    if (!entry) continue;
    const tab = entry.indexOf('\t');
    const [mode, type, object] = entry.slice(0, tab).split(' ');
    if (tab < 0 || !mode || !type || !object) throw new Error('Invalid Git tree entry.');
    tree.set(entry.slice(tab + 1), { mode, read: async () => {
      if (type !== 'blob') throw new Error('Owned path is not a Git blob.');
      return git(directory, ['cat-file', 'blob', object]);
    } });
  }
  return tree;
}

export interface PublishSiteOptions {
  artifact: SiteArtifact;
  remote: string;
  publish?: boolean;
  ownershipSeed?: SiteManifest;
  maxAttempts?: number;
  /** Used by isolated integration tests to introduce a competing local writer. */
  beforePush?: (head: string, commit: string, attempt: number) => Promise<void>;
}

interface OverlayOptions {
  remote: string;
  publish?: boolean;
  maxAttempts?: number;
  beforePush?: PublishSiteOptions['beforePush'];
  message: string;
  plan(current: ExistingTree): Promise<SitePlan>;
}

/** Both writers rebuild their own overlay on a fresh tree in a disposable repository. */
async function publishOverlay(options: OverlayOptions) {
  if (!options.remote || options.remote.startsWith('-') || /[\r\n\0]/.test(options.remote)) throw new Error('Invalid publication remote.');
  const directory = await mkdtemp(join(tmpdir(), 'tiktools-site-publish-'));
  try {
    git(directory, ['init', '--bare', '--quiet']);
    await mkdir(join(directory, 'disabled-hooks'));
    await mkdir(join(directory, 'empty-worktree'));
    git(directory, ['config', 'core.hooksPath', join(directory, 'disabled-hooks')]);
    git(directory, ['remote', 'add', 'origin', options.remote]);
    async function readCurrent() {
      git(directory, ['fetch', '--quiet', '--no-tags', 'origin', `refs/heads/${PAGES_BRANCH}`]);
      return gitText(directory, ['rev-parse', '--verify', 'FETCH_HEAD']);
    }
    let preparation = 0;
    async function prepare(head: string) {
      const plan = await options.plan(gitTree(directory, head));
      if (!options.publish || (!plan.writes.length && !plan.deletes.length)) return { commit: null, value: plan };
      preparation += 1;
      // update-index requires a work-tree context for removals. This empty
      // disposable directory is never checked out or populated with files.
      const env = { GIT_INDEX_FILE: join(directory, `site-index-${preparation}`),
        GIT_WORK_TREE: join(directory, 'empty-worktree') };
      git(directory, ['read-tree', head], undefined, env);
      for (const path of plan.deletes) git(directory, ['update-index', '--force-remove', '--', path], undefined, env);
      for (const write of plan.writes) {
        const blob = git(directory, ['hash-object', '-w', '--stdin'], write.bytes).toString('utf8').trim();
        git(directory, ['update-index', '--add', '--cacheinfo', '100644', blob, write.path], undefined, env);
      }
      const tree = git(directory, ['write-tree'], undefined, env).toString('utf8').trim();
      const commit = git(directory, ['commit-tree', tree, '-p', head, '-m', options.message], undefined, {
        GIT_AUTHOR_NAME: 'github-actions[bot]', GIT_COMMITTER_NAME: 'github-actions[bot]',
        GIT_AUTHOR_EMAIL: '41898282+github-actions[bot]@users.noreply.github.com',
        GIT_COMMITTER_EMAIL: '41898282+github-actions[bot]@users.noreply.github.com',
      }).toString('utf8').trim();
      return { commit, value: plan };
    }
    const result = await reapplyWithCas({ readCurrent, revision: (head: string) => head, prepare,
      push: async (commit, expectedHead) => {
        if (await readCurrent() !== expectedHead) throw new Error('gh-pages advanced before push.');
        await options.beforePush?.(expectedHead, commit, preparation);
        // Normal single-ref fast-forward push is the server-side race check.
        // A competing writer after readCurrent makes this push fail; CAS then
        // constructs a fresh child of that writer, preserving its feed bytes.
        git(directory, ['push', '--porcelain', 'origin', `${commit}:refs/heads/${PAGES_BRANCH}`]);
      },
    }, options.maxAttempts);
    return { mode: options.publish ? 'publish' as const : 'plan' as const, ...result };
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** Fetches and writes in a disposable bare repository; never mutates the source checkout. */
export async function publishSite(options: PublishSiteOptions) {
  verifyArtifact(options.artifact);
  return publishOverlay({ ...options, message: `site: ${options.artifact.manifest.sourceCommit}`,
    plan: (current) => planSiteUpdate(current, options.artifact, options.ownershipSeed) });
}

const FEED_FILES = ['stable.json', 'stable.json.sig', 'beta.json', 'beta.json.sig'] as const;
export type FeedHeads = ReadonlyMap<string, Uint8Array>;
export interface PublishFeedOptions {
  remote: string;
  candidateDirectory?: string;
  publish?: boolean;
  maxAttempts?: number;
  beforePush?: PublishSiteOptions['beforePush'];
  /** Tests inject already-authenticated fixture bytes, never a production verifier override. */
  stage?: (current: FeedHeads) => Promise<FeedHeads>;
}

function feedVersion(value: Uint8Array): string {
  const parsed: unknown = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(value));
  if (!parsed || typeof parsed !== 'object' || !('version' in parsed) || typeof parsed.version !== 'string'
      || normalizeVersion(parsed.version) !== parsed.version) throw new Error('Invalid authenticated feed version.');
  return parsed.version;
}

function validateFeedSnapshot(current: FeedHeads, next: FeedHeads): void {
  if (!next.has('stable.json') && !next.has('beta.json')) throw new Error('A feed snapshot must retain at least one channel.');
  for (const path of next.keys()) if (!(FEED_FILES as readonly string[]).includes(path)) throw new Error(`Feed snapshot contains a foreign path: ${path}`);
  for (const channel of ['stable', 'beta']) {
    const name = `${channel}.json`;
    const old = current.get(name);
    const replacement = next.get(name);
    if (Boolean(replacement) !== next.has(`${name}.sig`)) throw new Error(`Incomplete ${channel} snapshot pair.`);
    if (old && !replacement) throw new Error(`A feed publication cannot remove ${channel}.`);
    if (!replacement) continue;
    const version = feedVersion(replacement);
    if (channel === 'stable' && version.split('+', 1)[0]?.includes('-')) throw new Error('Stable feed cannot contain a prerelease.');
    if (old) {
      const order = Bun.semver.order(version, feedVersion(old));
      if (order < 0) throw new Error(`Refusing to roll back ${channel}.`);
      if (order === 0 && (sha256(old) !== sha256(replacement)
          || sha256(current.get(`${name}.sig`)!) !== sha256(next.get(`${name}.sig`)!))) {
        throw new Error(`An unchanged ${channel} version must retain its exact JSON and signature bytes.`);
      }
    }
  }
}

/** Existing signature/artifact authentication runs again against current heads on every CAS attempt. */
async function stageSignedFeed(candidate: string, current: FeedHeads): Promise<FeedHeads> {
  const directory = await mkdtemp(join(tmpdir(), 'tiktools-feed-stage-'));
  try {
    const heads = join(directory, 'heads');
    const snapshot = join(directory, 'snapshot');
    await mkdir(heads);
    for (const [path, value] of current) await writeFile(join(heads, path), value);
    const mode = !current.has('stable.json') && !current.has('beta.json') ? ['--bootstrap']
      : !current.has('stable.json') ? ['--allow-missing-stable'] : [];
    const child = spawnSync(process.execPath, [join(import.meta.dirname, 'stage-update-channels.ts'), resolve(candidate), heads, snapshot, ...mode], {
      cwd: resolve(import.meta.dirname, '..'), env: process.env, timeout: 20 * 60_000, maxBuffer: 16 * 1024 * 1024,
    });
    if (child.error || child.status !== 0) throw new Error(`Signed feed staging failed: ${String(child.stderr ?? child.error ?? '').trim().slice(0, 4000)}`);
    const next = new Map<string, Uint8Array>();
    for (const path of await readdir(snapshot)) {
      if (!(FEED_FILES as readonly string[]).includes(path)) throw new Error('Unexpected staged feed file.');
      const info = await lstat(join(snapshot, path));
      if (!info.isFile() || info.isSymbolicLink()) throw new Error('Feed staging requires regular files.');
      next.set(path, await readFile(join(snapshot, path)));
    }
    return next;
  } finally { await rm(directory, { recursive: true, force: true }); }
}

export async function publishUpdateFeed(options: PublishFeedOptions) {
  if (!options.stage && !options.candidateDirectory) throw new Error('A signed update candidate directory is required.');
  return publishOverlay({ ...options, message: `update feed: ${process.env.RELEASE_TAG ?? 'verified candidate'}`,
    plan: async (tree) => {
      if (tree.has('updates')) throw new Error('updates is a foreign file or symlink, not a directory.');
      const current = new Map<string, Uint8Array>();
      for (const path of FEED_FILES) {
        const file = tree.get(`updates/${path}`);
        if (file) current.set(path, await regularBytes(file, `updates/${path}`));
      }
      for (const channel of ['stable', 'beta']) {
        if (current.has(`${channel}.json`) !== current.has(`${channel}.json.sig`)) throw new Error(`Incomplete current ${channel} feed pair.`);
      }
      const next = options.stage ? await options.stage(current) : await stageSignedFeed(options.candidateDirectory!, current);
      validateFeedSnapshot(current, next);
      const writes: SitePlan['writes'] = [];
      const unchanged: string[] = [];
      for (const [path, value] of next) {
        const target = `updates/${path}`;
        for (const existing of tree.keys()) {
          if (existing !== target && (existing.toLowerCase() === target.toLowerCase()
              || existing.startsWith(`${target}/`))) throw new Error(`Foreign feed-path collision: ${existing}`);
        }
        if (current.has(path) && sha256(current.get(path)!) === sha256(value)) unchanged.push(target);
        else writes.push({ path: target, bytes: value });
      }
      return { writes, deletes: [], unchanged, usedOwnershipSeed: false,
        preservedForeignFiles: tree.size - current.size };
    },
  });
}

export function publicationRepository(environment: Record<string, string | undefined>, explicit?: string): string {
  const repository = explicit ?? environment.RELEASE_REPO ?? environment.GITHUB_REPOSITORY;
  if (!repository || !/^[A-Za-z0-9][A-Za-z0-9-]*\/[A-Za-z0-9_.-]+$/.test(repository)
      || repository.endsWith('/.') || repository.endsWith('/..')) throw new Error('Use an explicit owner/repository or RELEASE_REPO/GITHUB_REPOSITORY.');
  return repository;
}

async function main(args: string[]) {
  const values = new Map<string, string>();
  let publish = false;
  for (let index = 0; index < args.length; index += 1) {
    const key = args[index];
    if (key === '--publish') {
      if (publish) throw new Error('Duplicate --publish.');
      publish = true;
    } else if (key && ['--artifact', '--repository', '--source-commit', '--ownership-seed', '--feed-candidate'].includes(key)) {
      const value = args[++index];
      if (!value || value.startsWith('--') || values.has(key)) throw new Error(`Missing or duplicate ${key}.`);
      values.set(key, value);
    } else throw new Error('Usage: bun scripts/publish-site.ts [--artifact dist/site | --feed-candidate signed-directory] [--repository owner/repo] [--source-commit SHA] [--ownership-seed reviewed.json] [--publish]');
  }
  const base = siteConfig().base;
  const repository = publicationRepository(process.env, values.get('--repository'));
  const remote = `https://github.com/${repository}.git`;
  const feedCandidate = values.get('--feed-candidate');
  if (feedCandidate) {
    if (['--artifact', '--source-commit', '--ownership-seed'].some((key) => values.has(key))) throw new Error('Feed mode cannot accept site artifact or ownership options.');
    const result = await publishUpdateFeed({ remote, candidateDirectory: feedCandidate, publish });
    console.log(JSON.stringify({ mode: result.mode, repository, branch: PAGES_BRANCH, attempts: result.attempts,
      commit: result.commit, writes: result.value.writes.map((write) => write.path), preservedForeignFiles: result.value.preservedForeignFiles }, null, 2));
    return;
  }
  const artifact = await loadSiteArtifact(values.get('--artifact') ?? 'dist/site', base, values.get('--source-commit'));
  const seedPath = values.get('--ownership-seed');
  const ownershipSeed = seedPath ? parseSiteManifest(await readFile(seedPath), base) : undefined;
  const result = await publishSite({ artifact, ownershipSeed, remote, publish });
  console.log(JSON.stringify({ mode: result.mode, repository, branch: PAGES_BRANCH, attempts: result.attempts,
    previousHead: result.head, commit: result.commit, sourceCommit: artifact.manifest.sourceCommit,
    writes: result.value.writes.map((write) => write.path), deletes: result.value.deletes,
    preservedForeignFiles: result.value.preservedForeignFiles, usedOwnershipSeed: result.value.usedOwnershipSeed,
  }, null, 2));
}

if (import.meta.main) {
  try { await main(process.argv.slice(2)); }
  catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; }
}
