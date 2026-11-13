import { describe, expect, test } from 'bun:test';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import {
  SITE_MANIFEST, loadSiteArtifact, parseSiteManifest, planSiteUpdate,
  publicationRepository, publishSite, publishUpdateFeed, reapplyWithCas,
  type ExistingFile, type SiteArtifact, type SiteManifest,
} from './publish-site.ts';

const base = '/TikTools-app/';
const encoder = new TextEncoder();
const bytes = (value: string) => encoder.encode(value);
const hash = (value: Uint8Array) => createHash('sha256').update(value).digest('hex');
const source = 'a'.repeat(40);

function artifact(values: Record<string, string | Uint8Array>, revision = source): SiteArtifact {
  const files = new Map(Object.entries(values).map(([path, value]) => [path, typeof value === 'string' ? bytes(value) : value]));
  const manifest: SiteManifest = { version: 1, sourceCommit: revision, dirty: false, base,
    files: Object.fromEntries([...files].map(([path, value]) => [path, hash(value)])) };
  return { manifest, manifestBytes: bytes(JSON.stringify(manifest) + '\n'), files };
}

function existing(values: Record<string, string | Uint8Array>, site?: SiteArtifact) {
  const files = new Map<string, ExistingFile>();
  for (const [path, value] of Object.entries(values)) {
    files.set(path, { mode: '100644', read: async () => typeof value === 'string' ? bytes(value) : value });
  }
  if (site) {
    for (const [path, value] of site.files) files.set(path, { mode: '100644', read: async () => value });
    files.set(SITE_MANIFEST, { mode: '100644', read: async () => site.manifestBytes });
  }
  return files;
}

async function inTemporaryDirectory<T>(run: (directory: string) => Promise<T>): Promise<T> {
  const directory = await mkdtemp(join(tmpdir(), 'tiktools-site-test-'));
  try { return await run(directory); }
  finally { await rm(directory, { recursive: true, force: true }); }
}

describe('manifest ownership and artifact provenance', () => {
  test('accepts the assembler format and binds base/source revision', () => {
    const candidate = artifact({ 'index.html': '<html>docs</html>', 'demo/index.html': 'demo' });
    expect(parseSiteManifest(candidate.manifestBytes, base, source)).toEqual(candidate.manifest);
    expect(() => parseSiteManifest(candidate.manifestBytes, '/', source)).toThrow('clean version-1');
    expect(() => parseSiteManifest(candidate.manifestBytes, base, 'b'.repeat(40))).toThrow('source commit');
    expect(() => parseSiteManifest(bytes(JSON.stringify({ ...candidate.manifest, dirty: true })), base)).toThrow('clean version-1');
  });

  test.each(['updates/stable.json', 'updates/beta.json.sig', 'Updates/old.json', 'CNAME', '.nojekyll'])('rejects ownership of protected path %s', (path) => {
    expect(() => parseSiteManifest(artifact({ [path]: 'protected' }).manifestBytes, base)).toThrow('Protected');
  });

  test.each(['../updates/stable.json', '/absolute', 'a//b', 'a\\b', '.git/config', 'a/../b', 'a:stream', 'name.', 'a/space ', 'a\0b'])('rejects unsafe path %s', (path) => {
    expect(() => parseSiteManifest(artifact({ [path]: 'unsafe' }).manifestBytes, base)).toThrow('Unsafe');
  });

  test('rejects self-ownership, case aliases and file/ancestor collisions', () => {
    const cases: Array<Record<string, string>> = [{ [SITE_MANIFEST]: 'x' }, { 'A.html': 'a', 'a.html': 'b' }, { assets: 'a', 'assets/app.js': 'b' }];
    for (const values of cases) {
      expect(() => parseSiteManifest(artifact(values).manifestBytes, base)).toThrow();
    }
  });

  test('rejects changed bytes, unlisted files and mismatched in-memory metadata', async () => {
    const candidate = artifact({ 'index.html': 'valid' });
    await expect(planSiteUpdate(existing({}), { ...candidate, files: new Map([['index.html', bytes('tampered')]]) })).rejects.toThrow('hash mismatch');
    await expect(planSiteUpdate(existing({}), { ...candidate, files: new Map([...candidate.files, ['extra.html', bytes('x')]]) })).rejects.toThrow('unlisted');
    await expect(planSiteUpdate(existing({}), { ...candidate, manifest: { ...candidate.manifest, sourceCommit: 'b'.repeat(40) } })).rejects.toThrow('source commit');
  });

  test('removes only previously owned files and never reads protected/foreign bytes', async () => {
    const prior = artifact({ 'index.html': 'old', 'assets/stale.js': 'stale', 'en/index.html': 'same' });
    const current = existing({}, prior);
    for (const path of ['updates/stable.json', 'updates/stable.json.sig', 'updates/beta.json', 'updates/beta.json.sig', 'CNAME', '.nojekyll', 'foreign/file.bin']) {
      current.set(path, { mode: '100644', read: async () => { throw new Error('Foreign content must not be read or rewritten.'); } });
    }
    const plan = await planSiteUpdate(current, artifact({ 'index.html': 'new', 'en/index.html': 'same', 'demo/index.html': 'demo' }));
    expect(plan.deletes).toEqual(['assets/stale.js']);
    expect(plan.writes.map((write) => write.path).sort()).toEqual(['demo/index.html', 'index.html', SITE_MANIFEST].sort());
    expect(plan.unchanged).toEqual(['en/index.html']);
    expect(plan.preservedForeignFiles).toBe(7);
  });

  test('fails closed when another writer changes a previously owned file', async () => {
    const prior = artifact({ 'index.html': 'old' });
    const current = existing({}, prior);
    current.set('index.html', { mode: '100644', read: async () => bytes('foreign edit') });
    await expect(planSiteUpdate(current, artifact({ 'index.html': 'new' }))).rejects.toThrow('changed outside its manifest');
  });

  test('bootstrap keeps unowned paths, even when bytes happen to match', async () => {
    const candidate = artifact({ 'index.html': 'same' });
    await expect(planSiteUpdate(existing({ 'index.html': 'same' }), candidate)).rejects.toThrow('foreign path');
    const plan = await planSiteUpdate(existing({ 'updates/stable.json': 'feed', CNAME: 'domain' }), candidate);
    expect(plan.deletes).toEqual([]);
    expect(plan.preservedForeignFiles).toBe(2);
  });

  test('a reviewed ownership seed can migrate legacy paths only at their exact hashes', async () => {
    const legacy = artifact({ 'index.html': 'legacy', 'assets/legacy.js': 'legacy asset' });
    const current = existing({ 'index.html': 'legacy', 'assets/legacy.js': 'legacy asset', 'unrelated/file': 'foreign' });
    const candidate = artifact({ 'index.html': 'new' });
    const plan = await planSiteUpdate(current, candidate, legacy.manifest);
    expect(plan.usedOwnershipSeed).toBe(true);
    expect(plan.deletes).toEqual(['assets/legacy.js']);
    expect(plan.preservedForeignFiles).toBe(1);
    await expect(planSiteUpdate(existing({ 'index.html': 'changed' }), candidate, legacy.manifest)).rejects.toThrow('changed outside');
    await expect(planSiteUpdate(existing({}, legacy), candidate, legacy.manifest)).rejects.toThrow('cannot replace');
  });

  test('does not follow foreign symlinks or replace foreign descendant paths', async () => {
    const current = existing({ 'assets/foreign.js': 'foreign' });
    await expect(planSiteUpdate(current, artifact({ assets: 'replace directory' }))).rejects.toThrow('foreign path');
    const linked = new Map<string, ExistingFile>([['assets', { mode: '120000', read: async () => bytes('/outside') }]]);
    await expect(planSiteUpdate(linked, artifact({ 'assets/app.js': 'new' }))).rejects.toThrow('foreign path');
    const owned = artifact({ 'index.html': 'old' });
    const bad = existing({}, owned);
    bad.set('index.html', { mode: '120000', read: async () => bytes('old') });
    await expect(planSiteUpdate(bad, artifact({ 'index.html': 'new' }))).rejects.toThrow('not a regular Git blob');
  });

  test('loads exact on-disk artifacts and rejects any unlisted file or symlink', async () => inTemporaryDirectory(async (directory) => {
    const candidate = artifact({ 'index.html': 'site', 'demo/index.html': 'demo' });
    for (const [path, value] of [...candidate.files, [SITE_MANIFEST, candidate.manifestBytes] as const]) {
      await mkdir(dirname(join(directory, path)), { recursive: true });
      await writeFile(join(directory, path), value);
    }
    expect((await loadSiteArtifact(directory, base, source)).manifest).toEqual(candidate.manifest);
    await writeFile(join(directory, 'unlisted.txt'), 'unlisted');
    await expect(loadSiteArtifact(directory, base)).rejects.toThrow('unlisted');
    await rm(join(directory, 'unlisted.txt'));
    await symlink(join(directory, 'index.html'), join(directory, 'alias.html'));
    await expect(loadSiteArtifact(directory, base)).rejects.toThrow('symlink');
  }));

  test('resolves the actual release repository instead of silently choosing the workflow fork', () => {
    expect(publicationRepository({ RELEASE_REPO: 'nglmercer/TikTools-app', GITHUB_REPOSITORY: 'hernan-lc/TikTools-app' })).toBe('nglmercer/TikTools-app');
    expect(publicationRepository({ GITHUB_REPOSITORY: 'nglmercer/TikTools-app' })).toBe('nglmercer/TikTools-app');
    expect(publicationRepository({ RELEASE_REPO: 'wrong/repo' }, 'nglmercer/TikTools-app')).toBe('nglmercer/TikTools-app');
    for (const target of ['https://github.com/a/b', '../bad', 'a/b/c', 'a/..']) expect(() => publicationRepository({}, target)).toThrow();
  });
});

describe('CAS writer recovery', () => {
  test('reconstructs from a fresh current tree after a competing writer', async () => {
    let head = 'before';
    const bases: string[] = [];
    const result = await reapplyWithCas({
      readCurrent: async () => head, revision: (state: string) => state,
      prepare: async (state) => { bases.push(state); return { commit: `site-${state}`, value: state }; },
      push: async (commit, expected) => {
        if (expected === 'before') { head = 'new-feed'; throw new Error('non-fast-forward'); }
        head = commit;
      },
    });
    expect(bases).toEqual(['before', 'new-feed']);
    expect(result.attempts).toBe(2);
    expect(head).toBe('site-new-feed');
  });

  test('recovers an acknowledged-late successful push without another mutation', async () => {
    let head = 'before';
    let preparations = 0;
    const result = await reapplyWithCas({ readCurrent: async () => head, revision: (value: string) => value,
      prepare: async () => { preparations += 1; return { commit: 'published', value: true }; },
      push: async (commit) => { head = commit; throw new Error('response lost'); },
    });
    expect(result.commit).toBe('published');
    expect(preparations).toBe(1);
  });

  test('does not retry authentication/network failures against an unchanged head', async () => {
    let preparations = 0;
    await expect(reapplyWithCas({ readCurrent: async () => 'same', revision: (value: string) => value,
      prepare: async () => { preparations += 1; return { commit: 'site', value: true }; },
      push: async () => { throw new Error('authentication rejected'); },
    })).rejects.toThrow('authentication rejected');
    expect(preparations).toBe(1);
  });

  test('bounds repeated writer conflicts and never restores a former head', async () => {
    let head = 0;
    await expect(reapplyWithCas({ readCurrent: async () => String(head), revision: (value: string) => value,
      prepare: async () => ({ commit: 'site', value: true }),
      push: async () => { head += 1; throw new Error('competing writer'); },
    }, 3)).rejects.toThrow('all 3 publication attempts');
    expect(head).toBe(3);
  });
});

function git(directory: string, ...args: string[]): Buffer {
  return execFileSync('git', args, { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, GIT_TERMINAL_PROMPT: '0', GIT_CONFIG_NOSYSTEM: '1' } });
}
function textGit(directory: string, ...args: string[]) { return git(directory, ...args).toString('utf8').trim(); }

const protectedFiles = {
  'updates/stable.json': bytes('{ "version": "4.0.0", "spacing": true }\n'),
  'updates/stable.json.sig': new Uint8Array([0, 13, 10, 255, 1]),
  'updates/beta.json': bytes('{"version":"4.2.1-rc.3"}\n'),
  'updates/beta.json.sig': bytes('untrusted comment: exact bytes\r\nBASE64\r\n'),
  CNAME: bytes('example.invalid\n'), '.nojekyll': new Uint8Array(),
  'foreign/keep.bin': new Uint8Array([0, 1, 2, 255]),
};

async function seedRemote(directory: string) {
  const remote = join(directory, 'pages.git');
  const writer = join(directory, 'writer');
  await mkdir(remote); await mkdir(writer);
  git(remote, 'init', '--bare', '--quiet');
  git(writer, 'init', '--quiet');
  git(writer, 'config', 'user.name', 'Isolated test');
  git(writer, 'config', 'user.email', 'test@example.invalid');
  git(writer, 'config', 'commit.gpgsign', 'false');
  git(writer, 'checkout', '--quiet', '-b', 'gh-pages');
  const prior = artifact({ 'index.html': 'old', 'assets/stale.js': 'stale', 'en/index.html': 'unchanged' });
  const files = new Map([...Object.entries(protectedFiles), ...prior.files, [SITE_MANIFEST, prior.manifestBytes]]);
  for (const [path, value] of files) {
    await mkdir(dirname(join(writer, path)), { recursive: true });
    await writeFile(join(writer, path), value);
  }
  await symlink('foreign/keep.bin', join(writer, 'foreign-link'));
  git(writer, 'add', '--all');
  git(writer, 'commit', '--quiet', '-m', 'isolated seed');
  git(writer, 'remote', 'add', 'origin', remote);
  git(writer, 'push', '--quiet', 'origin', 'HEAD:refs/heads/gh-pages');
  return { remote, writer, prior };
}

describe('real Git publication against temporary local repositories', () => {
  test('default planning creates no remote commit and preserves the source checkout', async () => inTemporaryDirectory(async (directory) => {
    const { remote, writer } = await seedRemote(directory);
    const head = textGit(remote, 'rev-parse', 'gh-pages');
    const result = await publishSite({ remote, artifact: artifact({ 'index.html': 'new' }, 'b'.repeat(40)) });
    expect(result.mode).toBe('plan');
    expect(result.changed).toBe(false);
    expect(result.value.deletes).toEqual(['assets/stale.js', 'en/index.html']);
    expect(textGit(remote, 'rev-parse', 'gh-pages')).toBe(head);
    expect(textGit(writer, 'status', '--porcelain')).toBe('');
  }));

  test('publishes only owned changes and preserves every foreign blob and mode', async () => inTemporaryDirectory(async (directory) => {
    const { remote } = await seedRemote(directory);
    const before = textGit(remote, 'ls-tree', '-r', 'gh-pages');
    const result = await publishSite({ remote, publish: true,
      artifact: artifact({ 'index.html': 'new', 'en/index.html': 'unchanged', 'demo/index.html': 'demo' }, 'b'.repeat(40)) });
    expect(result.attempts).toBe(1);
    expect(git(remote, 'show', 'gh-pages:index.html').toString()).toBe('new');
    expect(() => git(remote, 'show', 'gh-pages:assets/stale.js')).toThrow();
    for (const [path, value] of Object.entries(protectedFiles)) expect(git(remote, 'show', `gh-pages:${path}`)).toEqual(Buffer.from(value));
    const after = textGit(remote, 'ls-tree', '-r', 'gh-pages');
    for (const line of before.split('\n').filter((entry) => /\t(?:updates\/|CNAME$|\.nojekyll$|foreign\/|foreign-link$)/.test(entry))) expect(after.split('\n')).toContain(line);
  }));

  test('a live feed writer wins its race and the site is reapplied without feed rollback', async () => inTemporaryDirectory(async (directory) => {
    const { remote, writer } = await seedRemote(directory);
    const liveFeed = bytes('{"version":"4.2.2","writer":"release"}\n');
    const liveSignature = bytes('new exact signature\r\n');
    const result = await publishSite({ remote, publish: true,
      artifact: artifact({ 'index.html': 'new docs', 'demo/index.html': 'demo' }, 'c'.repeat(40)),
      beforePush: async (_head, _commit, attempt) => {
        if (attempt !== 1) return;
        await writeFile(join(writer, 'updates/stable.json'), liveFeed);
        await writeFile(join(writer, 'updates/stable.json.sig'), liveSignature);
        await writeFile(join(writer, 'foreign/live-writer.txt'), 'concurrent foreign asset');
        git(writer, 'add', '--all');
        git(writer, 'commit', '--quiet', '-m', 'isolated concurrent release');
        git(writer, 'push', '--quiet', 'origin', 'HEAD:refs/heads/gh-pages');
      },
    });
    expect(result.attempts).toBe(2);
    expect(git(remote, 'show', 'gh-pages:updates/stable.json')).toEqual(Buffer.from(liveFeed));
    expect(git(remote, 'show', 'gh-pages:updates/stable.json.sig')).toEqual(Buffer.from(liveSignature));
    expect(git(remote, 'show', 'gh-pages:updates/beta.json')).toEqual(Buffer.from(protectedFiles['updates/beta.json']));
    expect(git(remote, 'show', 'gh-pages:foreign/live-writer.txt').toString()).toBe('concurrent foreign asset');
    expect(git(remote, 'show', 'gh-pages:index.html').toString()).toBe('new docs');
    expect(textGit(remote, 'rev-parse', 'gh-pages^')).toBe(textGit(writer, 'rev-parse', 'HEAD'));
  }), 20_000);

  test('a second identical publication is a no-op', async () => inTemporaryDirectory(async (directory) => {
    const { remote, prior } = await seedRemote(directory);
    const head = textGit(remote, 'rev-parse', 'gh-pages');
    const result = await publishSite({ remote, publish: true, artifact: prior });
    expect(result.changed).toBe(false);
    expect(textGit(remote, 'rev-parse', 'gh-pages')).toBe(head);
  }));
});

test('site publication is manual, branch-bound and coordinated with the release writer', async () => {
  const workflow = await readFile(new URL('../.github/workflows/site.yml', import.meta.url), 'utf8');
  expect(workflow).toContain("github.event_name == 'workflow_dispatch' && inputs.publish && github.ref == 'refs/heads/remake'");
  expect(workflow).not.toContain('false &&');
  expect(workflow).toContain('vars.RELEASE_REPO || github.repository');
  expect(workflow).toContain('bun ci');
  expect(workflow).toContain('bun scripts/task.ts docs test');
  expect(workflow).toContain('bun scripts/task.ts site build');
  expect(workflow).not.toContain('cargo ');
  const release = await readFile(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  const feedStep = release.split('      - name: Publish update feed')[1]?.split('      - name: Wait for the live update feed')[0] ?? '';
  expect(feedStep).toContain('bun scripts/publish-site.ts --feed-candidate update-candidate');
  expect(feedStep).not.toContain('git clone');
  expect(feedStep).not.toContain('git push');
});

describe('both checked writers against real local Git conflicts', () => {
  const advanceBeta = async (current: ReadonlyMap<string, Uint8Array>) => {
    const next = new Map(current);
    next.set('beta.json', bytes('{"version":"4.2.2-rc.1","release":"fixture"}\n'));
    next.set('beta.json.sig', bytes('authenticated fixture signature\r\n'));
    return next;
  };

  test('feed CAS retries after the site writer and preserves site, stable pair and foreign objects', async () => inTemporaryDirectory(async (directory) => {
    const { remote } = await seedRemote(directory);
    const candidate = artifact({ 'index.html': 'new concurrent site', 'demo/index.html': 'new demo' }, 'd'.repeat(40));
    let stages = 0;
    const result = await publishUpdateFeed({ remote, publish: true,
      stage: async (current) => { stages += 1; return advanceBeta(current); },
      beforePush: async (_head, _commit, attempt) => {
        if (attempt === 1) await publishSite({ remote, publish: true, artifact: candidate });
      },
    });
    expect(result.attempts).toBe(2);
    expect(stages).toBe(2);
    expect(git(remote, 'show', 'gh-pages:index.html').toString()).toBe('new concurrent site');
    expect(git(remote, 'show', `gh-pages:${SITE_MANIFEST}`)).toEqual(Buffer.from(candidate.manifestBytes));
    for (const path of ['updates/stable.json', 'updates/stable.json.sig', 'CNAME', '.nojekyll', 'foreign/keep.bin'] as const) {
      expect(git(remote, 'show', `gh-pages:${path}`)).toEqual(Buffer.from(protectedFiles[path]));
    }
    expect(git(remote, 'show', 'gh-pages:updates/beta.json')).toEqual(Buffer.from((await advanceBeta(new Map())).get('beta.json')!));
    expect(git(remote, 'show', 'gh-pages:updates/beta.json.sig')).toEqual(Buffer.from((await advanceBeta(new Map())).get('beta.json.sig')!));
  }), 20_000);

  test('site CAS retries after the checked feed writer and retains its exact signed pairs', async () => inTemporaryDirectory(async (directory) => {
    const { remote } = await seedRemote(directory);
    const result = await publishSite({ remote, publish: true,
      artifact: artifact({ 'index.html': 'site after feed' }, 'e'.repeat(40)),
      beforePush: async (_head, _commit, attempt) => {
        if (attempt === 1) await publishUpdateFeed({ remote, publish: true, stage: advanceBeta });
      },
    });
    expect(result.attempts).toBe(2);
    for (const path of ['updates/stable.json', 'updates/stable.json.sig', 'CNAME', '.nojekyll', 'foreign/keep.bin'] as const) {
      expect(git(remote, 'show', `gh-pages:${path}`)).toEqual(Buffer.from(protectedFiles[path]));
    }
    const next = await advanceBeta(new Map());
    expect(git(remote, 'show', 'gh-pages:updates/beta.json')).toEqual(Buffer.from(next.get('beta.json')!));
    expect(git(remote, 'show', 'gh-pages:updates/beta.json.sig')).toEqual(Buffer.from(next.get('beta.json.sig')!));
    expect(git(remote, 'show', 'gh-pages:index.html').toString()).toBe('site after feed');
  }), 20_000);

  test('rejects stale, same-version-rewritten, incomplete and foreign feed snapshots before push', async () => inTemporaryDirectory(async (directory) => {
    const { remote } = await seedRemote(directory);
    const before = textGit(remote, 'rev-parse', 'gh-pages');
    for (const kind of ['rollback', 'same-version', 'incomplete', 'foreign'] as const) {
      await expect(publishUpdateFeed({ remote, publish: true, stage: async (current) => {
        const next = new Map(current);
        if (kind === 'rollback') next.set('stable.json', bytes('{"version":"1.0.0"}'));
        if (kind === 'same-version') next.set('stable.json.sig', bytes('different signature'));
        if (kind === 'incomplete') next.delete('beta.json.sig');
        if (kind === 'foreign') next.set('index.html', bytes('cannot replace site'));
        return next;
      } })).rejects.toThrow();
      expect(textGit(remote, 'rev-parse', 'gh-pages')).toBe(before);
    }
  }));
});
