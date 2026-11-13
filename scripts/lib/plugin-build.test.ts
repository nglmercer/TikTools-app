import { describe, expect, test } from 'bun:test';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildPluginUi, executableName, pluginCargoManifest, processBinaryName } from './plugin-build.ts';

describe('external plugin builds', () => {
  test('resolves a backend crate independently of its nested install entry', async () => {
    const root = await mkdtemp(join(tmpdir(), 'plugin-backend-'));
    try {
      await mkdir(join(root, 'backend'));
      const manifest = join(root, 'backend', 'Cargo.toml');
      await writeFile(manifest, '[package]\nname = "backend-package"\n[[bin]]\nname = "sonicboom-backend"\n');
      expect(await pluginCargoManifest(root)).toBe(manifest);
      expect(await processBinaryName(manifest, 'backend/dist/sonicboom-backend')).toBe('sonicboom-backend');
      expect(executableName('backend/dist/sonicboom-backend', true)).toBe('sonicboom-backend.exe');
      expect(executableName('backend\\dist\\sonicboom-backend.exe', true)).toBe('sonicboom-backend.exe');
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  test('rebuilds a UI even when an old committed bundle exists', async () => {
    const root = await mkdtemp(join(tmpdir(), 'plugin-ui-'));
    try {
      const directory = join(root, 'plugins', 'speech');
      await mkdir(join(directory, 'ui', 'dist'), { recursive: true });
      await writeFile(join(directory, 'ui', 'vite.config.ts'), '');
      const index = join(directory, 'ui', 'dist', 'index.html');
      await writeFile(index, 'stale');
      const result = await buildPluginUi({
        directory,
        manifestPath: join(directory, 'plugin.json'),
        manifest: { ui: { entry: 'ui/dist/index.html' } },
        exec: async (_command, args, cwd) => {
          expect(cwd).toBe(root);
          expect(args).toContain('--no-install');
          await writeFile(index, 'rebuilt');
        },
      });
      expect(result?.source).toBe(join(directory, 'ui', 'dist'));
      expect(await readFile(index, 'utf8')).toBe('rebuilt');
      await expect(buildPluginUi({
        directory, manifestPath: 'plugin.json', manifest: { ui: { entry: 'ui/../../outside.html' } },
        exec: () => { throw new Error('must not build an unsafe entry'); },
      })).rejects.toThrow('unsafe ui entry');
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
