import {  } from '../../src/web/shared/testing/host-messages.ts';
const t = (key: string): string => testMessage('en', key);
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  emitTopic,
  installFakeHost,
  waitForFrontend,
  legacyMessagesOf,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { SONICBOOM_ID, SPEAK_ACTION } from './fixtures/plugins.ts';
import { installPluginUiOverride, routePluginFixture } from './fixtures/plugin-frame.ts';
import { ttsPageState, webviewPageState } from './fixtures/states.ts';
import { SEL, PLUGIN_TITLE } from './support/selectors.ts';
import { WAIT_MS } from './support/timeouts.ts';
import { TOPIC } from '../shared/index.ts';

async function openTts(page: Parameters<typeof installFakeHost>[0]) {
  const tab = page.getByRole('link', { name: PLUGIN_TITLE.tts });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(page.getByRole('heading', { name: PLUGIN_TITLE.tts }).first()).toBeVisible();
}

test('legacy TTS section degrades to a status note without plugin UI', async ({ page }) => {
  // No `ui` descriptor: the legacy `tts` section kind renders the neutral
  // placeholder. Nothing plugin-specific (voices, tester, outputs) may
  // load in the main frontend.
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await waitForFrontend(page);
  await openTts(page);

  await expect(page.locator('.plugin-node__status')).toBeVisible();
  await expect(page.getByLabel('Default voice')).toHaveCount(0);
  await expect(page.getByLabel('Voice', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Text', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Play' })).toHaveCount(0);
  await expect(page.locator('iframe.plugin-frame')).toHaveCount(0);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin UI renders inline and round-trips through the broker', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await waitForFrontend(page);
  await openTts(page);

  const frame = page.frameLocator(SEL.pluginFrame);
  // Settings flow from the fake host into the frame through the real shim.
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: WAIT_MS.frame });

  // Actions execute live through the scoped backend.
  await frame.locator('#speak').click();
  await expect(frame.locator('#log')).toContainText('ok', { timeout: WAIT_MS.short });
  const host = await readHostState(page);
  const executed = host.calls.filter((call) => call.method === 'plugins.action.execute');
  expect(executed.length).toBeGreaterThanOrEqual(1);
  expect(executed[0]?.params).toMatchObject({
    actionType: SPEAK_ACTION,
    live: true,
    pluginId: SONICBOOM_ID,
  });

  // Subscribed plugin events reach the frame; other plugins' do not.
  await emitTopic(page, TOPIC.pluginEvent, { pluginId: SONICBOOM_ID, eventType: 'speech.state' });
  await expect(frame.locator('#events')).toContainText('speech.state', { timeout: WAIT_MS.short });
  await emitTopic(page, TOPIC.pluginEvent, { pluginId: 'someone.else', eventType: 'intruder' });
  await page.waitForTimeout(300);
  await expect(frame.locator('#events')).not.toContainText('intruder');

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('inline page pops out through desktop host messages', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await waitForFrontend(page);
  await openTts(page);

  const frame = page.frameLocator(SEL.pluginFrame);
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: WAIT_MS.frame });

  await page.getByRole('button', { name: t('plugins.page.openWindow') }).click();
  await expect
    .poll(async () => (await legacyMessagesOf(page, 'plugin-ui-open')).length)
    .toBe(1);
  expect((await legacyMessagesOf(page, 'plugin-ui-open'))[0]).toMatchObject({
    type: 'plugin-ui-open',
    pluginId: SONICBOOM_ID,
    pageId: 'tts',
  });

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
