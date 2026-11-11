import { afterAll, describe, expect, test } from 'bun:test';

/**
 * The retired `/__plugins/` Vite middleware read the removed in-repo
 * `plugins/` directory and nothing requests it any more: plugin UIs load
 * over `tiktools-plugin://` (desktop), through the local web adapter (the
 * `tiktools-plugin-assets` meta + proxy) or the E2E base override. These
 * tests pin that the supported adapter route stays and the dead one does not
 * come back.
 */
const previous = {
  adapter: process.env.TIKTOOLS_DEV_WEB_ADAPTER,
  port: process.env.TIKTOOLS_WEB_PORT,
};

function restore(): void {
  for (const [key, value] of [['TIKTOOLS_DEV_WEB_ADAPTER', previous.adapter], ['TIKTOOLS_WEB_PORT', previous.port]] as const) {
    if (value === undefined) delete process.env[key];
    else process.env[key] = value;
  }
}
afterAll(restore);

async function loadConfig(query: string, adapter: string | undefined) {
  if (adapter === undefined) delete process.env.TIKTOOLS_DEV_WEB_ADAPTER;
  else process.env.TIKTOOLS_DEV_WEB_ADAPTER = adapter;
  process.env.TIKTOOLS_WEB_PORT = '3999';
  const module = (await import(`../../vite.config.ts?${query}`)) as { default: { plugins: Array<{ name?: string }>; server: { proxy?: Record<string, unknown> } } };
  return module.default;
}

describe('vite dev server plugin assets', () => {
  test('no in-repo plugin asset middleware is installed', async () => {
    for (const [query, adapter] of [['plain', undefined], ['adapter', 'http://127.0.0.1:4010']] as const) {
      const config = await loadConfig(query, adapter);
      const names = config.plugins.flat().map((plugin) => plugin?.name);
      expect(names).not.toContain('tiktools-plugin-assets');
    }
  });

  test('the web adapter mode still proxies plugin assets and local transport', async () => {
    const config = await loadConfig('adapter-proxy', 'http://127.0.0.1:4010');
    expect(Object.keys(config.server.proxy ?? {}).sort()).toEqual(['/__local', '/__plugins']);
  });

  test('without the adapter nothing is proxied', async () => {
    const config = await loadConfig('plain-proxy', undefined);
    expect(config.server.proxy).toBeUndefined();
  });
});
