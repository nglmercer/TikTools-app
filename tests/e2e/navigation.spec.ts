import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);
import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  emitTopic,
  installFakeHost,
  waitForFrontend,
  readHostState,
  writeHostState,
} from './fixtures/tiktools-host.ts';
import { emptyState, ttsPageState } from './fixtures/states.ts';

test('boots to the feed with named navigation and an active tab marker', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);

  await expect(page.getByRole('navigation', { name: 'Main Navigation' })).toBeVisible();
  const feedTab = page.getByRole('link', { name: t('navigation.feed') });
  await expect(feedTab).toBeVisible();
  await expect(feedTab).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('link', { name: 'Plugins' })).toBeVisible();
  await expect(page.getByText(t('feed.empty'), { exact: false })).toBeVisible();

  const state = await readHostState(page);
  expect(state.frontendReady).toBe(true);
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('dynamic plugin tab appears, renders, and falls back when removed', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await waitForFrontend(page);

  // Host snapshot contains the plugin page → tab appears.
  const ttsTab = page.getByRole('link', { name: 'Text to Speech' });
  await expect(ttsTab).toBeVisible();

  // Click tab → correct plugin page renders.
  await ttsTab.click();
  await expect(ttsTab).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('heading', { name: 'Text to Speech' }).first()).toBeVisible();

  // Host drops the plugin → refresh → tab falls back to Plugins, no crash.
  const dropped = await readHostState(page);
  const snapshot = dropped.automationSnapshot as { pluginPages: unknown[]; plugins: unknown[] };
  snapshot.pluginPages = [];
  snapshot.plugins = [];
  await writeHostState(page, dropped);
  await emitTopic(page, 'plugin.uninstalled', { pluginId: 'sonicboom.server' });
  await expect(page.getByRole('link', { name: 'Text to Speech' })).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Plugins' })).toHaveAttribute(
    'aria-current',
    'page',
  );
  await expect(page.getByRole('heading', { name: 'Plugins' }).first()).toBeVisible();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
