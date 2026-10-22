import { expect, test } from '@playwright/test';
import { baseFakeState, installFakeBroker, readBrokerCalls, type SonicBoomFakeState } from './fixtures.ts';

test('playback tracking is opt-in, renders duration and controls the shared server queue', async ({ page }) => {
  const state = baseFakeState();
  state.playback = { currentId: 'track-one', queueLength: 2, playing: true, paused: false, durationSecs: 12.5 };
  const capture = await installFakeBroker(page, state);
  await page.goto('/');
  await expect(page.getByLabel('Track audio for playback controls')).not.toBeChecked();
  expect((await readBrokerCalls(page)).some(call => call.params.action === 'sonicboom.server.playback-status')).toBe(false);
  await page.getByLabel('Track audio for playback controls').check();
  await expect(page.locator('.tts-playback')).toContainText('12.5 s');
  await page.getByRole('button', { name: 'Skip current audio', exact: true }).click();
  await expect(page.locator('.tts-playback__metadata')).toHaveAttribute('title', 'next-track');
  await page.getByRole('button', { name: 'Pause audio', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Resume audio', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Resume audio', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Pause audio', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Stop and clear queue', exact: true }).click();
  await expect(page.locator('.tts-playback')).toContainText('No current audio');
  await expect(page.getByRole('button', { name: 'Skip current audio', exact: true })).toBeDisabled();
  await page.getByLabel('Track audio for playback controls').uncheck();
  await page.waitForTimeout(250);
  const before = (await readBrokerCalls(page)).filter(call => call.params.action === 'sonicboom.server.playback-status').length;
  await page.waitForTimeout(2200);
  expect((await readBrokerCalls(page)).filter(call => call.params.action === 'sonicboom.server.playback-status')).toHaveLength(before);
  capture.assertClean();
});

test('unknown duration stays unknown and authentication failures disable controls', async ({ page }) => {
  const state = baseFakeState();
  state.playback = { currentId: 'track-one', queueLength: 1, playing: true, paused: false };
  await installFakeBroker(page, state);
  await page.goto('/');
  await page.getByLabel('Track audio for playback controls').check();
  await expect(page.locator('.tts-playback__duration')).toHaveText('—');
  await expect(page.locator('.tts-playback__duration')).toHaveAttribute('title', /This server does not report audio duration/);
  await page.evaluate(() => { (window as unknown as { __SONICBOOM_STATE__: SonicBoomFakeState }).__SONICBOOM_STATE__.playback!.error = 'unauthorized'; });
  await expect(page.locator('.tts-playback [role="alert"]')).toContainText('Check the API token');
  await expect(page.getByRole('button', { name: 'Skip current audio', exact: true })).toBeDisabled();
});

test('failed skip is reported without claiming the track changed', async ({ page }) => {
  const state = baseFakeState();
  state.playback = { currentId: 'track-one', queueLength: 1, playing: true, paused: false };
  state.speakOutcome = { ok: false, summary: 'HTTP 503', error: 'Playback unavailable', logs: [] };
  await installFakeBroker(page, state);
  await page.goto('/'); await page.getByLabel('Track audio for playback controls').check();
  await expect(page.getByRole('button', { name: 'Skip current audio', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Skip current audio', exact: true }).click();
  await expect(page.locator('.tts-playback [role="alert"]')).toContainText('Playback unavailable');
  await expect(page.locator('.tts-playback__metadata')).toHaveAttribute('title', 'track-one');
});

test('audio bar stays compact with icon controls and optional details in titles', async ({ page }) => {
  const state = baseFakeState();
  state.playback = { currentId: 'private-queue-reference', queueLength: 2, playing: true, paused: false, durationSecs: 12.5 };
  await installFakeBroker(page, state); await page.goto('/');
  const card = page.locator('.tts-playback');
  expect((await card.boundingBox())!.height).toBeLessThan(90);
  await page.getByRole('checkbox', { name: 'Track audio for playback controls', exact: true }).check();
  await expect(card).toContainText('12.5 s');
  expect((await card.boundingBox())!.height).toBeLessThan(100);
  await expect(card).not.toContainText('private-queue-reference');
  await expect(card).not.toContainText('Controls affect the shared server queue');
  await expect(page.getByRole('button', { name: 'Skip current audio', exact: true })).toHaveText('');
  await expect(page.getByRole('button', { name: 'Skip current audio', exact: true })).toHaveAttribute('title', 'Skip current audio');
});
