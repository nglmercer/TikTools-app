import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { SONICBOOM_ID } from './fixtures/plugins.ts';
import { ttsPageState, webviewPageState } from './fixtures/states.ts';

async function openTts(page: Parameters<typeof installFakeHost>[0]) {
  const tab = page.getByRole('button', { name: 'Text to Speech' });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(page.getByRole('heading', { name: 'Text to Speech' }).first()).toBeVisible();
}

test('legacy TTS section degrades to a status note without plugin UI', async ({ page }) => {
  // No `ui` descriptor: the legacy `tts` section kind renders the neutral
  // placeholder. Nothing plugin-specific (voices, tester, outputs) may
  // load in the main frontend.
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await openTts(page);

  await expect(page.getByText('This panel moved to the plugin view.')).toBeVisible();
  await expect(page.getByLabel('Default voice')).toHaveCount(0);
  await expect(page.getByLabel('Voice', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Text', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Play' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Open plugin view' })).toHaveCount(0);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('webview page opens and closes through desktop host messages', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await page.goto('/');
  await openTts(page);

  // The launcher explains the isolated view; the plugin bundle itself is
  // never loaded here.
  await expect(
    page.getByText('This plugin renders its own view in a separate window.'),
  ).toBeVisible();
  const open = page.getByRole('button', { name: 'Open plugin view' });
  await expect(open).toBeVisible();

  await open.click();
  await expect
    .poll(async () => (await readHostState(page)).legacyMessages.length)
    .toBe(1);
  let host = await readHostState(page);
  expect(host.legacyMessages[0]).toMatchObject({
    type: 'plugin-ui-open',
    pluginId: SONICBOOM_ID,
    pageId: 'tts',
  });

  const close = page.getByRole('button', { name: 'Close plugin view' });
  await expect(close).toBeVisible();
  await close.click();
  await expect
    .poll(async () => (await readHostState(page)).legacyMessages.length)
    .toBe(2);
  host = await readHostState(page);
  expect(host.legacyMessages[1]).toMatchObject({
    type: 'plugin-ui-close',
    pluginId: SONICBOOM_ID,
    pageId: 'tts',
  });

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
