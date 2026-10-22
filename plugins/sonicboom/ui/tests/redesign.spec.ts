import { test, expect } from '@playwright/test';
import { baseFakeState, installFakeBroker } from './fixtures.ts';

test('dependent audience fields appear on demand and retain their values', async ({ page }) => {
  await installFakeBroker(page, baseFakeState()); await page.goto('/');
  await expect(page.getByRole('checkbox', { name: 'Team members', exact: true })).toBeHidden();
  await page.getByRole('checkbox', { name: 'All users', exact: true }).uncheck();
  const team = page.getByRole('checkbox', { name: 'Team members', exact: true });
  await expect(page.getByLabel('Minimum team level', { exact: true })).toBeHidden();
  await team.check(); await page.getByLabel('Minimum team level', { exact: true }).fill('12');
  await team.uncheck(); await expect(page.getByLabel('Minimum team level', { exact: true })).toBeHidden();
  await team.check(); await expect(page.getByLabel('Minimum team level', { exact: true })).toHaveValue('12');
  await page.getByRole('checkbox', { name: 'All users', exact: true }).check();
  await page.getByRole('checkbox', { name: 'All users', exact: true }).uncheck();
  await expect(team).toBeChecked(); await expect(page.getByLabel('Minimum team level', { exact: true })).toHaveValue('12');
});

test('command and cost fields are revealed only for their selected modes', async ({ page }) => {
  await installFakeBroker(page, baseFakeState()); await page.goto('/');
  await expect(page.locator('#tts-command')).toBeHidden(); await expect(page.locator('#tts-points-cost')).toBeHidden();
  await page.getByRole('radio', { name: 'Starts with custom command', exact: true }).check();
  await page.locator('#tts-command').fill('!speak');
  await page.getByRole('radio', { name: 'Any comment', exact: true }).check();
  await expect(page.locator('#tts-command')).toBeHidden();
  await page.getByRole('radio', { name: 'Starts with custom command', exact: true }).check();
  await expect(page.locator('#tts-command')).toHaveValue('!speak');
  await page.getByRole('radio', { name: 'Charge points per message', exact: true }).check();
  await expect(page.locator('#tts-points-cost')).toBeVisible();
});

test('checkbox hints support hover and focus, and optional groups expand', async ({ page }) => {
  await installFakeBroker(page, baseFakeState()); await page.goto('/');
  const toggle = page.getByRole('checkbox', { name: 'All users', exact: true });
  const hint = page.getByRole('tooltip').filter({ hasText: 'Everyone can use TTS.' });
  await toggle.locator('..').hover(); await expect(hint).toBeVisible();
  await page.mouse.move(0, 0); await toggle.focus(); await expect(hint).toBeVisible();
  await page.keyboard.press('Escape'); await expect(hint).toBeHidden();
  await expect(page.locator('#tts-speed')).toBeHidden();
  await page.getByText('Voice adjustments', { exact: true }).click();
  await expect(page.locator('#tts-speed')).toBeVisible(); await expect(page.locator('#tts-speed')).toBeDisabled();
  await expect(page.locator('#tts-emote-mode')).toBeHidden();
  await page.locator('summary').filter({ hasText: 'Text processing' }).click();
  await expect(page.locator('#tts-emote-mode')).toBeVisible();
  await page.getByRole('checkbox', { name: 'Remove TikTok emote codes', exact: true }).uncheck();
  await expect(page.locator('#tts-emote-mode')).toBeHidden();
});

test('mobile groups fit the viewport and SVG controls retain text names', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const state = baseFakeState(); state.playback = { currentId: 'audio-42', queueLength: 1, playing: true, paused: false, durationSecs: 12.5 };
  await installFakeBroker(page, state); await page.goto('/');
  await page.getByRole('checkbox', { name: 'Track audio for playback controls', exact: true }).check();
  const skip = page.getByRole('button', { name: 'Skip current audio', exact: true });
  await expect(skip).toBeEnabled(); await expect(skip.locator('svg')).toHaveAttribute('aria-hidden', 'true');
  expect(await page.locator('#app').evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
});
