import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  countCalls,
  installFakeHost,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { SONICBOOM_ID, SPEAK_ACTION } from './fixtures/plugins.ts';
import { actionFailureState, ttsPageState } from './fixtures/states.ts';

async function openTts(page: Parameters<typeof installFakeHost>[0]) {
  const tab = page.getByRole('button', { name: 'Text to Speech' });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(page.getByRole('heading', { name: 'Text to Speech' }).first()).toBeVisible();
}

test('TTS basic flow: voices load, speak executes, log renders', async ({ page }) => {
  const state = ttsPageState();
  state.actionDelayMs = 400;
  const consoleCapture = await installFakeHost(page, state);
  await page.goto('/');
  await openTts(page);

  // Voices loaded through the option pipeline.
  await expect(page.getByLabel('Default voice')).toBeVisible();
  await expect(page.getByText('3 voices')).toBeVisible();

  // Select voice, type text, speak.
  await page.getByLabel('Voice', { exact: true }).selectOption('M1');
  await page.getByLabel('Text', { exact: true }).fill('hello from e2e');
  await page.getByRole('button', { name: 'Play' }).click();

  // Speaking state appears while the action is in flight.
  await expect(page.getByRole('button', { name: 'Speaking…' })).toBeVisible();

  // Result arrives → log renders with the spoken line.
  await expect(page.getByRole('log')).toContainText('spoken', { timeout: 5_000 });

  const host = await readHostState(page);
  const executed = host.calls.filter((call) => call.method === 'plugins.action.execute');
  expect(executed.length).toBeGreaterThanOrEqual(1);
  expect(executed[0]?.params['actionType']).toBe(SPEAK_ACTION);
  expect(executed[0]?.params['config']).toMatchObject({
    text: 'hello from e2e',
    voice: 'M1',
  });

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('TTS action failure surfaces in the log without crashing', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, actionFailureState());
  await page.goto('/');
  await openTts(page);

  await page.getByLabel('Text', { exact: true }).fill('this will fail');
  await page.getByRole('button', { name: 'Play' }).click();

  await expect(page.getByRole('log')).toContainText('voice not found', { timeout: 5_000 });
  await expect(page.getByLabel('Default voice')).toBeVisible();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('rapid TTS setting changes are coalesced before persistence', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await openTts(page);

  const volume = page.getByLabel(/Volume/);
  await expect(volume).toBeVisible();

  // Simulate a slider drag: many input events in quick succession.
  for (let i = 0; i < 20; i += 1) {
    await volume.fill(String(Math.round((0.3 + i * 0.01) * 100) / 100));
  }

  // Local UI reflects the final value immediately…
  await expect(page.getByText('49%')).toBeVisible();

  // …while the host receives coalesced writes, not one per input event.
  await expect
    .poll(async () => countCalls(page, 'app.state.set'), { timeout: 5_000 })
    .toBeGreaterThanOrEqual(1);
  await page.waitForTimeout(600);
  const writes = await countCalls(page, 'app.state.set');
  expect(writes).toBeLessThan(20);

  // Final persisted settings equal final UI state.
  const host = await readHostState(page);
  const persisted = host.appState[`tts.settings:${SONICBOOM_ID}`] ?? '';
  expect(persisted).toContain('0.49');

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
