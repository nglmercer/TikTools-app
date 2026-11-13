import { readFile, access, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve, relative } from 'node:path';
interface Original { source: string; sha256: string; anchors?: string[] }
interface Entry { source: string; destinations: string[]; archive: string; sourceSha256: string; audience: string; status: string; migrationStatus: string; linkTreatment: { kind: string; legacyPathPreserved?: boolean; anchors?: string[] } }
interface Migration { sourceCommit: string; entries: Entry[]; assets: Array<{ source: string; archive: string; sourceSha256: string; linkTreatment: { kind: string; legacySourceRemoved?: boolean } }> }
/** Validate coverage and the historical bytes, not merely the number of rows. */
export async function migrationErrors(root: string): Promise<string[]> {
  const errors: string[] = [];
  const inventory = JSON.parse(await readFile(resolve(root, 'docs/inventory.json'), 'utf8')) as { sourceCommit: string; documents: Original[]; assets: Original[] };
  const migration = JSON.parse(await readFile(resolve(root, 'docs/migration-map.json'), 'utf8')) as Migration;
  if (migration.sourceCommit !== inventory.sourceCommit) errors.push('Migration provenance commit changed.');
  const entries = new Map(migration.entries.map((entry) => [entry.source, entry]));
  if (entries.size !== migration.entries.length) errors.push('Duplicate migration sources.');
  async function legacyDocuments(directory: string): Promise<void> {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      if (['content', 'internal', 'node_modules', '.vitepress', '.generated', 'examples', 'contracts', 'scripts'].includes(entry.name)) continue;
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) await legacyDocuments(path);
      else if (entry.name.endsWith('.md') && !entries.has(relative(root, path).replaceAll('\\', '/'))) errors.push(`Unmapped legacy document: ${relative(root, path)}`);
    }
  }
  await legacyDocuments(resolve(root, 'docs'));
  const safePath = (path: string) => { const target = resolve(root, path); if (relative(root, target).startsWith('..')) throw new Error('Migration path escapes repository.'); return target; };
  for (const document of inventory.documents) {
    const entry = entries.get(document.source);
    if (!entry) { errors.push(`Missing migration: ${document.source}`); continue; }
    if (entry.sourceSha256 !== document.sha256 || !entry.archive.startsWith('docs/internal/archive/')) errors.push(`Invalid provenance: ${document.source}`);
    try { const bytes = await readFile(safePath(entry.archive)); if (createHash('sha256').update(bytes).digest('hex') !== document.sha256) errors.push(`Changed historical bytes: ${entry.archive}`); } catch { errors.push(`Missing archive: ${entry.archive}`); }
    if (!['user', 'developer', 'mixed', 'internal'].includes(entry.audience) || !['available', 'experimental', 'historical', 'mixed'].includes(entry.status) || !['migrated', 'archived'].includes(entry.migrationStatus)) errors.push(`Invalid migration state: ${entry.source}`);
    if (!entry.destinations.length) errors.push(`Missing destination: ${entry.source}`);
    for (const destination of entry.destinations) try { await access(safePath(destination.split('#')[0]!)); } catch { errors.push(`Missing migration destination: ${destination}`); }
    if (entry.linkTreatment.kind === 'archived-source-removed') {
      if (!entry.source.startsWith('docs/') || entry.linkTreatment.legacyPathPreserved !== false) errors.push(`Invalid removed-source policy: ${entry.source}`);
      try { await access(safePath(entry.source)); errors.push(`Retired legacy document was restored: ${entry.source}`); } catch { /* The exact original remains in the validated archive. */ }
      continue;
    }
    if (!['maintained-root', 'bilingual-compatibility-stub'].includes(entry.linkTreatment.kind)) errors.push(`Invalid link treatment: ${entry.source}`);
    try {
      const stub = await readFile(safePath(entry.source), 'utf8');
      const anchors = new Set([...stub.matchAll(/\bid=["']([^"']+)["']/g)].map((match) => match[1]));
      for (const anchor of document.anchors ?? []) if (!anchors.has(anchor)) errors.push(`Lost legacy anchor: ${entry.source}#${anchor}`);
    } catch { errors.push(`Lost legacy path: ${entry.source}`); }
  }
  for (const asset of inventory.assets) {
    const entry = migration.assets.find((entry) => entry.source === asset.source);
    if (!entry || entry.sourceSha256 !== asset.sha256) { errors.push(`Unmapped asset: ${asset.source}`); continue; }
    if (entry.linkTreatment.legacySourceRemoved) {
      if (entry.linkTreatment.kind !== 'archived-copy-only') errors.push(`Invalid removed-asset policy: ${entry.source}`);
      try { await access(safePath(entry.source)); errors.push(`Retired legacy asset was restored: ${entry.source}`); } catch { /* Preserve only the byte-checked archived copy. */ }
    }
    for (const path of entry.linkTreatment.legacySourceRemoved ? [entry.archive] : [entry.source, entry.archive]) try { if (createHash('sha256').update(await readFile(safePath(path))).digest('hex') !== asset.sha256) errors.push(`Changed historical asset: ${path}`); } catch { errors.push(`Missing historical asset: ${path}`); }
  }
  return errors;
}
