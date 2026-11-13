import { afterEach, expect, test } from 'bun:test';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { migrationErrors } from './migration.ts';

const roots: string[] = [];
afterEach(async () => { await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true }))); });
async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-docs-migration-')); roots.push(root);
  const original = '# Original guide\n'; const image = 'original image';
  const hash = (value: string) => createHash('sha256').update(value).digest('hex');
  const sourceCommit = 'a'.repeat(40);
  await mkdir(join(root, 'docs/internal/archive/docs'), { recursive: true });
  await mkdir(join(root, 'docs/content/en/user'), { recursive: true });
  await writeFile(join(root, 'docs/internal/archive/docs/OLD.md'), original);
  await writeFile(join(root, 'docs/internal/archive/docs/old.png'), image);
  await writeFile(join(root, 'docs/content/en/user/index.md'), '# Current guide\n');
  const entry = { source: 'docs/OLD.md', destinations: ['docs/content/en/user/index.md'], archive: 'docs/internal/archive/docs/OLD.md', sourceSha256: hash(original), audience: 'user', status: 'available', migrationStatus: 'migrated', linkTreatment: { kind: 'archived-source-removed', legacyPathPreserved: false, anchorDestinations: { 'original-guide': ['docs/content/en/user/index.md'] } } };
  await writeFile(join(root, 'docs/inventory.json'), JSON.stringify({ sourceCommit, documents: [{ source: entry.source, sha256: hash(original), anchors: ['original-guide'] }], assets: [{ source: 'docs/old.png', sha256: hash(image) }] }));
  const migration = { sourceCommit, entries: [entry], assets: [{ source: 'docs/old.png', sourceSha256: hash(image), archive: 'docs/internal/archive/docs/old.png', linkTreatment: { kind: 'archived-copy-only', legacySourceRemoved: true } }] };
  const save = () => writeFile(join(root, 'docs/migration-map.json'), JSON.stringify(migration));
  await save(); return { root, migration, save };
}

test('removed legacy documents and images remain covered by exact archives and current destinations', async () => {
  const { root } = await fixture(); expect(await migrationErrors(root)).toEqual([]);
});
test('migration destinations validate generated heading fragments', async () => {
  const { root, migration, save } = await fixture();
  const entry = migration.entries[0]!;
  entry.destinations = ['docs/content/en/user/index.md#current-guide'];
  await save();
  expect(await migrationErrors(root)).toEqual([]);
  entry.destinations = ['docs/content/en/user/index.md#missing'];
  await save();
  expect((await migrationErrors(root)).join('\n')).toContain('Missing migration anchor');
});
test('topic migration targets cannot bypass fragment validation or omit an original topic', async () => {
  const { root, migration, save } = await fixture();
  migration.entries[0]!.linkTreatment.anchorDestinations['original-guide'] = ['docs/content/en/user/index.md#missing'];
  await save();
  expect((await migrationErrors(root)).join('\n')).toContain('Missing migration anchor');
  migration.entries[0]!.linkTreatment.anchorDestinations['original-guide'] = [];
  await save();
  expect((await migrationErrors(root)).join('\n')).toContain('Missing topic migration');
});
test('cleanup policy rejects reintroduced legacy copies', async () => {
  const { root } = await fixture();
  await writeFile(join(root, 'docs/OLD.md'), 'duplicate'); await writeFile(join(root, 'docs/old.png'), 'duplicate');
  const errors = (await migrationErrors(root)).join('\n');
  expect(errors).toContain('Retired legacy document was restored'); expect(errors).toContain('Retired legacy asset was restored');
});
test('removing compatibility files cannot hide a missing or changed archive', async () => {
  const { root } = await fixture(); await writeFile(join(root, 'docs/internal/archive/docs/OLD.md'), 'changed');
  await rm(join(root, 'docs/internal/archive/docs/old.png'));
  const errors = (await migrationErrors(root)).join('\n');
  expect(errors).toContain('Changed historical bytes'); expect(errors).toContain('Missing historical asset');
});
test('the removal policy cannot silently retire maintained repository entry points', async () => {
  const { root, migration, save } = await fixture();
  const entry = migration.entries[0]!; entry.source = 'README.md';
  const inventory = JSON.parse(await Bun.file(join(root, 'docs/inventory.json')).text()); inventory.documents[0].source = entry.source;
  await writeFile(join(root, 'docs/inventory.json'), JSON.stringify(inventory)); await save();
  expect((await migrationErrors(root)).join('\n')).toContain('Invalid removed-source policy');
});
