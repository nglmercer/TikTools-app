import { describe, expect, it } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';
import { routes } from '../../../../../src/web/app/router/routes.ts';
import { builtinTabPaths, tabFromRoute, tabLocation } from '../../../../../src/web/app/router/tab-routes.ts';

function createTestRouter() {
  return createRouter({ history: createMemoryHistory(), routes });
}

describe('tab routes', () => {
  it('maps every builtin tab to a route that resolves back to it', () => {
    const router = createTestRouter();
    for (const [tab, path] of Object.entries(builtinTabPaths)) {
      expect(tabLocation(tab as keyof typeof builtinTabPaths)).toBe(path);
      expect(tabFromRoute(router.resolve(path))).toBe(tab);
    }
  });

  it('round-trips plugin page tabs through route params', () => {
    const router = createTestRouter();
    const resolved = router.resolve(tabLocation('plugin:sonicboom:voices'));
    expect(resolved.path).toBe('/plugins/sonicboom/voices');
    expect(tabFromRoute(resolved)).toBe('plugin:sonicboom:voices');
  });

  it('falls back to the feed for unknown tabs', () => {
    expect(tabLocation('unknown' as never)).toBe('/feed');
  });

  it('redirects unknown paths to the home route', async () => {
    const router = createTestRouter();
    await router.push('/does-not-exist');
    expect(router.currentRoute.value.matched[0]?.path).toBe('/');
  });
});
