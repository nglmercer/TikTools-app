/**
 * Sync `tests/fixtures/plugins/<name>.plugin.json` from the external
 * plugins-example-tiktools checkout. The fixtures are byte-exact copies of
 * the real packages so host tests pin what users actually install; editing
 * a fixture by hand is always drift waiting to happen.
 *
 *   bun scripts/sync-plugin-fixtures.ts          # copy (writes fixtures)
 *   bun scripts/sync-plugin-fixtures.ts --check  # fail when drifted
 *
 * The external checkout is optional: with no sibling repo the script is a
 * no-op success in both modes, because hosts must build without it.
 */
import { copyFile, readFile, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { resolvePluginsRepository } from './lib/external-plugins.ts';

const root = resolve(import.meta.dir, '..');
const checkOnly = process.argv.includes('--check');

/** fixture name -> package directory inside the external checkout. */
const FIXTURE_SOURCES: Record<string, string> = {
  'audio-process': 'plugins/audio-process/plugin.json',
  hotkeys: 'plugins/hotkeys/plugin.json',
  'minecraft-server': 'plugins/minecraft-server/plugin.json',
  'peak-game': 'plugins/peak-game/plugin.json',
  'repo-game': 'plugins/repo-game/plugin.json',
  sonicboom: 'plugins/sonicboom/plugin.json',
  'sonicboom-server': 'examples/sonicboom-server-declarative/plugin.json',
  textintel: 'plugins/textintel/plugin.json',
};

const pluginsRepository = await resolvePluginsRepository(root);
if (!pluginsRepository) {
  console.log('no external plugins checkout; fixture sync skipped');
  process.exit(0);
}

const drifted: string[] = [];
let synced = 0;
for (const [name, relative] of Object.entries(FIXTURE_SOURCES)) {
  const source = join(pluginsRepository, relative);
  const target = join(root, 'tests', 'fixtures', 'plugins', `${name}.plugin.json`);
  if (!(await stat(source).catch(() => null))) {
    // A missing upstream source means the committed fixture can no longer be
    // verified against the real package: in check mode that is drift, not a
    // skip — otherwise a renamed or deleted plugin would silently unpin the
    // fixture it is supposed to keep honest.
    if (checkOnly) {
      console.error(`${relative} missing in the external checkout; the committed fixture is unverifiable`);
      process.exit(1);
    }
    console.warn(`${relative} missing in the external checkout; skipped (commit the fixture removal explicitly if the plugin is gone)`);
    continue;
  }
  const [incoming, current] = await Promise.all([
    readFile(source, 'utf8'),
    readFile(target, 'utf8').catch(() => ''),
  ]);
  if (incoming === current) continue;
  drifted.push(name);
  if (!checkOnly) {
    await copyFile(source, target);
    synced += 1;
  }
}

if (drifted.length > 0) {
  const verb = checkOnly ? 'drifted' : 'synced';
  console.log(`${verb}: ${drifted.join(', ')}`);
  if (checkOnly) {
    console.error('fixtures are byte-exact copies of the external checkout; run bun scripts/sync-plugin-fixtures.ts');
    process.exit(1);
  }
}
console.log(`fixture sync done (${synced} updated, ${Object.keys(FIXTURE_SOURCES).length - drifted.length} already current)`);
