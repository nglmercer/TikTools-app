/**
 * Plugin iframe asset loading: the production failure mode the inline
 * broker fixture cannot reproduce.
 *
 * The built plugin bundle reaches the frame as external module scripts
 * and stylesheets, and the production frame is `sandbox="allow-scripts"`
 * (opaque origin). Strict engines CORS-check those subresources — the
 * blank production iframe was exactly this: 200 responses without
 * `Access-Control-Allow-Origin`, rejected as `Origin null`. These specs
 * pin the requirement from the browser side (assets must be CORS-readable
 * by the opaque-origin document);
 * the Rust asset-server header tests pin
 * the production server that meets it.
 */
import { expect, test } from '@playwright/test';

import type { Page } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  emitPush,
  installFakeHost,
  waitForFrontend,
} from './fixtures/tiktools-host.ts';
import {
  installPluginUiOverride,
  PLUGIN_EXTERNAL_BASE,
  compiledPluginUiAvailable,
  routeCompiledPluginUi,
  routeExternalAssetFixture,
} from './fixtures/plugin-frame.ts';
import { webviewPageState } from './fixtures/states.ts';
import { SEL, PLUGIN_TITLE } from './support/selectors.ts';
import { WAIT_MS } from './support/timeouts.ts';

test.skip(!compiledPluginUiAvailable, 'needs the plugins-example-tiktools checkout (set TIKTOOLS_PLUGINS_REPO)');

test('external module and stylesheet load in the sandboxed frame with CORS', async ({
  page,
}) => {
  await routeExternalAssetFixture(page, { withCors: true });
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => {
    errors.push(error instanceof Error ? error.message : String(error));
  });

  await page.goto(`${PLUGIN_EXTERNAL_BASE}harness.html`);
  const frame = page.frameLocator('#plugin-frame');
  await expect(frame.locator('#external')).toHaveText('external-js-ok', { timeout: WAIT_MS.frame });
  await expect(frame.locator('#external')).toHaveCSS('color', 'rgb(1, 2, 3)');

  expect(errors).toEqual([]);
});

test('external module is blocked in the sandboxed frame without CORS', async ({ page }) => {
  // Negative control for the spec above: the same document served with a
  // non-matching CORS origin reproduces the production blank iframe — the
  // module never runs (marker stays pending) and the console reports an
  // `Origin null` CORS block. This proves the positive spec actually
  // exercises the CORS behavior rather than passing regardless of headers.
  await routeExternalAssetFixture(page, { withCors: false });
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });

  await page.goto(`${PLUGIN_EXTERNAL_BASE}harness.html`);
  // Poll the collector attached above (the block may already have fired
  // before a post-navigation listener could subscribe).
  await expect
    .poll(() => errors.filter((text) => /cors/i.test(text)).length, { timeout: WAIT_MS.frame })
    .toBeGreaterThan(0);
  const frame = page.frameLocator('#plugin-frame');
  await expect(frame.locator('#external')).toHaveText('(pending)');
});

test('inline tab renders the real compiled SonicBoom bundle', async ({ page }) => {
  // Full production path with real assets: the compiled `ui/dist/` bundle
  // (external module + stylesheet, sandboxed opaque-origin frame) boots
  // through the real `PluginFrame` + `PluginWebviewHost` + control client
  // and renders the actual SonicBoom UI — not just an iframe element.
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routeCompiledPluginUi(page);
  await page.goto('/');
  await waitForFrontend(page);

  const tab = page.getByRole('link', { name: PLUGIN_TITLE.tts });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(page.getByRole('heading', { name: PLUGIN_TITLE.tts }).first()).toBeVisible();

  const frame = page.frameLocator(SEL.pluginFrame);
  await expect(frame.getByLabel('Default voice')).toBeVisible({ timeout: WAIT_MS.long });
  await expect(frame.getByLabel('Audio output').first()).toBeVisible({ timeout: WAIT_MS.frame });
  await expect(frame.getByRole('button', { name: 'Play' })).toBeVisible();

  // The iframe fills the tab area instead of a fixed-height box.
  const frameBox = await page.locator(SEL.pluginFrame).boundingBox();
  expect(frameBox?.height).toBeGreaterThan(500);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test.describe('responsive plugin layout', () => {
  test.use({ viewport: { width: 800, height: 520 } });

  test('small viewport keeps bottom controls reachable via internal scroll', async ({
    page,
  }) => {
    await expectScrollableInlineUi(page);
  });

  test('very short viewport still reaches every control', async ({ page }) => {
    // 400px tall: far less than the plugin content. The frame keeps a
    // usable height and the plugin scroller exposes every control.
    await page.setViewportSize({ width: 800, height: 400 });
    await expectScrollableInlineUi(page, { minFrameHeight: 180 });
  });
});

async function expectScrollableInlineUi(
  page: Page,
  options: { minFrameHeight?: number } = {},
): Promise<void> {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routeCompiledPluginUi(page);
  await page.goto('/');
  await waitForFrontend(page);

  // Narrow windows collapse navigation into the sidebar drawer; the native shell menu opens it.
  await emitPush(page, { type: 'shell-menu-action', id: 'toggle-sidebar' });
  const tab = page.getByRole('link', { name: PLUGIN_TITLE.tts });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(page.getByRole('heading', { name: PLUGIN_TITLE.tts }).first()).toBeVisible();

  const frame = page.frameLocator(SEL.pluginFrame);
  await expect(frame.getByLabel('Default voice')).toBeVisible({ timeout: WAIT_MS.long });

  // The plugin document is the single content scroller: at these sizes
  // the content overflows and must scroll instead of clipping.
  const metrics = await frame.locator('#app').evaluate((el) => {
    const style = getComputedStyle(el);
    return {
      scrollHeight: el.scrollHeight,
      clientHeight: el.clientHeight,
      scrollWidth: el.scrollWidth,
      clientWidth: el.clientWidth,
      overflowY: style.overflowY,
      boxSizing: style.boxSizing,
    };
  });
  expect(metrics.overflowY).toBe('auto');
  expect(metrics.boxSizing).toBe('border-box');
  expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);
  expect(metrics.scrollWidth).toBeLessThanOrEqual(metrics.clientWidth + 1);

  // Bottom controls are reachable by scrolling inside the frame.
  const play = frame.getByRole('button', { name: 'Play' });
  await play.scrollIntoViewIfNeeded();
  await expect(play).toBeVisible();

  // The full scroll range is traversable: scrolling to the very bottom
  // lands at the maximum (no clipped strip swallows the end of the
  // content, which border-box + internal scroll guarantee together).
  await frame.locator('#app').evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  const bottom = await frame.locator('#app').evaluate((el) => ({ top: el.scrollTop, max: el.scrollHeight - el.clientHeight }));
  expect(bottom.max).toBeGreaterThan(0);
  expect(bottom.top).toBeGreaterThanOrEqual(bottom.max - 2);

  // The iframe keeps a usable height and still fills the tab area.
  const frameBox = await page.locator(SEL.pluginFrame).boundingBox();
  const scrollBox = await page.locator('.plugin-page__panel .base-panel__body').boundingBox();
  expect(frameBox?.height).toBeGreaterThan(options.minFrameHeight ?? 0);
  expect(Math.abs((frameBox?.height ?? 0) - (scrollBox?.height ?? 0))).toBeLessThan(40);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
}
