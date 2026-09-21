import { expect, test } from '@playwright/test';

import {
  baseFakeState,
  installFakeBroker,
  readBrokerCalls,
  readBrokerState,
} from './fixtures.ts';

test('boots from the broker: settings, voices, and outputs render', async ({ page }) => {
  const consoleCapture = await installFakeBroker(page, baseFakeState());
  await page.goto('/');

  await expect(page.getByLabel('Default voice')).toBeVisible();
  await expect(page.getByText('3 voices')).toBeVisible();
  await expect(page.getByLabel('Audio output').first()).toHaveValue('Speakers');

  const calls = await readBrokerCalls(page);
  const methods = calls.map((call) => call.method);
  expect(methods).toContain('settings.get');
  expect(methods).toContain('options.get');

  consoleCapture.assertClean();
});

test('voice tester executes speech and renders the log', async ({ page }) => {
  const state = baseFakeState();
  state.speakDelayMs = 400;
  const consoleCapture = await installFakeBroker(page, state);
  await page.goto('/');

  await expect(page.getByText('3 voices')).toBeVisible();
  await page.getByLabel('Voice', { exact: true }).selectOption('M1');
  await page.getByLabel('Text', { exact: true }).fill('hello from ui suite');
  await page.getByRole('button', { name: 'Play' }).click();

  await expect(page.getByRole('button', { name: 'Speaking…' })).toBeVisible();
  await expect(page.getByRole('log')).toContainText('spoken', { timeout: 5_000 });

  const calls = await readBrokerCalls(page);
  const executed = calls.filter((call) => call.method === 'actions.execute');
  expect(executed.length).toBeGreaterThanOrEqual(1);
  expect(executed[0]?.params['action']).toBe('sonicboom.server.speak');
  expect(executed[0]?.params['config']).toMatchObject({
    text: 'hello from ui suite',
    voice: 'M1',
  });

  consoleCapture.assertClean();
});

test('speech failure surfaces in the log without crashing', async ({ page }) => {
  const state = baseFakeState();
  state.speakOutcome = { ok: false, summary: 'synthesis failed', logs: [], error: 'voice not found' };
  const consoleCapture = await installFakeBroker(page, state);
  await page.goto('/');

  await expect(page.getByText('3 voices')).toBeVisible();
  await page.getByLabel('Text', { exact: true }).fill('this will fail');
  await page.getByRole('button', { name: 'Play' }).click();

  await expect(page.getByRole('log')).toContainText('voice not found', { timeout: 5_000 });
  await expect(page.getByLabel('Default voice')).toBeVisible();

  consoleCapture.assertClean();
});

test('rapid setting changes are coalesced before persistence', async ({ page }) => {
  const consoleCapture = await installFakeBroker(page, baseFakeState());
  await page.goto('/');

  const volume = page.getByLabel(/Volume/);
  await expect(volume).toBeVisible();
  for (let i = 0; i < 20; i += 1) {
    await volume.fill(String(Math.round((0.3 + i * 0.01) * 100) / 100));
  }
  await expect(page.getByText('49%')).toBeVisible();

  await expect
    .poll(async () => (await readBrokerCalls(page)).filter((call) => call.method === 'settings.set').length, {
      timeout: 5_000,
    })
    .toBeGreaterThanOrEqual(1);
  await page.waitForTimeout(600);
  const writes = (await readBrokerCalls(page)).filter((call) => call.method === 'settings.set');
  expect(writes.length).toBeLessThan(20);

  const broker = await readBrokerState(page);
  const tts = broker.settings['tts'] as Record<string, unknown>;
  expect(tts['volume']).toBe(0.49);

  consoleCapture.assertClean();
});

test('output refresh re-reads devices and keeps the selection', async ({ page }) => {
  const consoleCapture = await installFakeBroker(page, baseFakeState());
  await page.goto('/');

  const outputs = page.getByLabel('Audio output').first();
  await expect(outputs).toHaveValue('Speakers');
  await page.getByRole('button', { name: 'Refresh outputs' }).click();

  await expect
    .poll(async () => {
      const calls = await readBrokerCalls(page);
      return calls.filter(
        (call) =>
          call.method === 'options.get' &&
          String(call.params['source']).endsWith(':device') &&
          call.params['refresh'] === true,
      ).length;
    })
    .toBeGreaterThanOrEqual(1);
  await expect(outputs).toHaveValue('Speakers');

  consoleCapture.assertClean();
});
