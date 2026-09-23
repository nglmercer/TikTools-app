import { describe, expect, test } from 'bun:test';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import { assertWidgetBundle, GATEWAY_WIDGET_KINDS } from './gateway-widgets.ts';

async function sandbox(): Promise<string> {
  const dir = join(tmpdir(), `tiktools-gateway-widgets-${Date.now()}-${Math.random().toString(36).slice(2)}`);
  await mkdir(dir, { recursive: true });
  return dir;
}

describe('assertWidgetBundle', () => {
  test('passes when every widget entry page exists', async () => {
    const dir = await sandbox();
    try {
      for (const kind of GATEWAY_WIDGET_KINDS) {
        await mkdir(join(dir, kind), { recursive: true });
        await writeFile(join(dir, kind, 'index.html'), '<html></html>');
      }
      await assertWidgetBundle(dir);
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test('fails loudly naming every missing widget', async () => {
    const dir = await sandbox();
    try {
      await mkdir(join(dir, 'follow'), { recursive: true });
      await writeFile(join(dir, 'follow', 'index.html'), '<html></html>');
      const failure = await assertWidgetBundle(dir).then(
        () => null,
        (error: unknown) => error as Error,
      );
      expect(failure).not.toBeNull();
      for (const kind of GATEWAY_WIDGET_KINDS) {
        if (kind === 'follow') {
          expect(failure?.message).not.toContain('follow/index.html');
        } else {
          expect(failure?.message).toContain(`${kind}/index.html`);
        }
      }
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test('fails on an empty directory', async () => {
    const dir = await sandbox();
    try {
      await expect(assertWidgetBundle(dir)).rejects.toThrow('follow/index.html');
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });
});
