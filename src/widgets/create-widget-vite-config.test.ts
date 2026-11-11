import { describe, expect, test } from 'bun:test';
import { join, resolve } from 'node:path';
import { GATEWAY_WIDGET_KINDS } from '../../scripts/lib/gateway-widgets.ts';
import { createWidgetViteConfig } from './create-widget-vite-config.ts';

const repositoryRoot = resolve(import.meta.dir, '..', '..');

describe('createWidgetViteConfig', () => {
  test('builds one independent bundle per widget into dist/widgets/<kind>', () => {
    const config = createWidgetViteConfig(join(repositoryRoot, 'src', 'widgets', 'follow'), 'follow');
    expect(config.root).toBe(join(repositoryRoot, 'src', 'widgets', 'follow'));
    expect(config.base).toBe('./');
    expect(config.build).toEqual({
      outDir: join(repositoryRoot, 'dist', 'widgets', 'follow'),
      emptyOutDir: true,
      sourcemap: false,
    });
    expect(config.plugins).toHaveLength(1);
  });

  test('each call creates its own Vue plugin instance', () => {
    const a = createWidgetViteConfig('/r/src/widgets/a', 'a');
    const b = createWidgetViteConfig('/r/src/widgets/b', 'b');
    expect(a.plugins?.[0]).not.toBe(b.plugins?.[0]);
  });

  test('rejects a kind that is unsafe or does not match the widget directory', () => {
    expect(() => createWidgetViteConfig('/r/src/widgets/follow', '../follow')).toThrow(/invalid widget kind/);
    expect(() => createWidgetViteConfig('/r/src/widgets/follow', '')).toThrow(/invalid widget kind/);
    expect(() => createWidgetViteConfig('/r/src/widgets/gift', 'follow')).toThrow(/does not match/);
  });

  test('every gateway widget config delegates to it and stays inside its own output directory', async () => {
    for (const kind of GATEWAY_WIDGET_KINDS) {
      const { default: config } = await import(`./${kind}/vite.config.ts`);
      expect(config.build.outDir).toBe(join(repositoryRoot, 'dist', 'widgets', kind));
      expect(config.root).toBe(join(repositoryRoot, 'src', 'widgets', kind));
    }
  });
});
