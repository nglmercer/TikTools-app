import { describe, expect, test } from 'bun:test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dir, '..', '..', '..');

function sync(target: string, source: string) {
  return Bun.spawn([process.execPath, 'scripts/sync-web-assets.ts', target, source], {
    cwd: root,
    stdout: 'pipe',
    stderr: 'pipe',
  });
}

describe('sync-web-assets', () => {
  test('replaces the packaged web directory with a complete fresh bundle', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'tiktools-sync-web-'));
    try {
      const source = join(dir, 'dist-web');
      const target = join(dir, 'app', 'web');
      await mkdir(join(source, 'assets'), { recursive: true });
      await writeFile(join(source, 'index.html'), '<html>new</html>');
      await writeFile(join(source, 'assets', 'app-new.css'), 'body{}');
      await mkdir(join(target, 'assets'), { recursive: true });
      await writeFile(join(target, 'index.html'), '<html>old</html>');
      await writeFile(join(target, 'assets', 'app-old.css'), 'old{}');

      const child = sync(target, source);
      expect(await child.exited).toBe(0);

      expect(await readFile(join(target, 'index.html'), 'utf8')).toBe('<html>new</html>');
      expect(await readFile(join(target, 'assets', 'app-new.css'), 'utf8')).toBe('body{}');
      expect(await readFile(join(target, 'assets', 'app-old.css'), 'utf8').catch(() => null)).toBeNull();
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test('refuses to delete a directory that contains the frontend build', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'tiktools-sync-web-guard-'));
    try {
      const source = join(dir, 'web');
      await mkdir(source, { recursive: true });
      await writeFile(join(source, 'index.html'), 'keep');
      const child = sync(dir, source);
      expect(await child.exited).not.toBe(0);
      expect(await readFile(join(source, 'index.html'), 'utf8')).toBe('keep');
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });
});
