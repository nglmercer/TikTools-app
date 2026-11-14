import { expect, test } from 'bun:test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { externalPluginDirectories, resolvePluginsRepository } from './external-plugins.ts';

test('a host checkout works without any sibling plugin repository', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-standalone-'));
  try {
    expect(await resolvePluginsRepository(root, {})).toBeNull();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('sibling discovery is optional and a custom checkout can live anywhere', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-plugin-paths-'));
  try {
    const host = join(root, 'host');
    const sibling = join(root, 'plugins-example-tiktools');
    const custom = join(root, 'elsewhere', 'custom plugins');
    await mkdir(host);
    await mkdir(join(sibling, 'plugins'), { recursive: true });
    await mkdir(join(custom, 'plugins', 'sample'), { recursive: true });
    await mkdir(join(custom, 'plugins', 'not-a-package'));
    await writeFile(join(custom, 'plugins', 'sample', 'plugin.json'), '{}');
    expect(await resolvePluginsRepository(host, {})).toBe(sibling);
    expect(await resolvePluginsRepository(host, { TIKTOOLS_PLUGINS_REPO: custom })).toBe(custom);
    expect(await resolvePluginsRepository(host, { TIKTOOLS_PLUGINS_REPO: '../elsewhere/custom plugins' })).toBe(custom);
    for (const disabled of ['', 'off', ' OFF ']) {
      expect(await resolvePluginsRepository(host, { TIKTOOLS_PLUGINS_REPO: disabled })).toBeNull();
    }
    expect(await externalPluginDirectories(custom)).toEqual([join(custom, 'plugins', 'sample')]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
