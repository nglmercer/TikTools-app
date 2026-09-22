import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('rapid replay and tab changes never place alert cards side by side', async ({ page }) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  await expect(page.locator('.follow-card')).toBeVisible();
  // Observe every animation frame, including the intermediate transition
  // states that a final visibility assertion would miss.
  await page.evaluate(() => {
    const violations: string[] = [];
    const target = window as unknown as { widgetTransitionViolations: string[]; stopWidgetMonitor: () => void };
    target.widgetTransitionViolations = violations;
    let frame = 0;
    const sample = () => {
      const host = document.querySelector('.widget-host');
      const cards = host?.querySelectorAll('[role="alert"]') ?? [];
      if (cards.length > 1) violations.push('multiple cards');
      const bounds = host?.getBoundingClientRect();
      if (bounds) {
        for (const card of Array.from(cards)) {
          const rect = card.getBoundingClientRect();
          if (Math.abs(rect.x + rect.width / 2 - bounds.x - bounds.width / 2) > 2) {
            violations.push('card moved off center');
          }
        }
      }
      frame = requestAnimationFrame(sample);
    };
    sample();
    target.stopWidgetMonitor = () => cancelAnimationFrame(frame);
  });
  for (const label of ['Followers', 'Gifts', 'Shared', 'Subscriptions', 'Followers']) {
    await page.getByRole('tab', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host [role="alert"]')).toBeVisible();
    for (let index = 0; index < 3; index++) {
      await page.getByRole('button', { name: 'Replay preview' }).click();
      await page.waitForTimeout(150);
    }
  }
  await page.waitForTimeout(700);
  const violations = await page.evaluate(() => {
    const target = window as unknown as { widgetTransitionViolations: string[]; stopWidgetMonitor: () => void };
    target.stopWidgetMonitor();
    return target.widgetTransitionViolations;
  });
  expect(violations).toEqual([]);
  await expect(page.locator('.follow-card')).toHaveCount(1);
});

test('all widget previews render locally without a gateway and replay', async ({ page }, testInfo) => {
  const consoleCapture = await installFakeHost(page, emptyState(), { theme: 'dark' });
  const gatewayRequests: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes(':17452')) gatewayRequests.push(request.url());
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Copy URL', exact: true })).toBeDisabled();
  for (const [label, kind, selector] of [
    ['Followers', 'follow', '.follow-card'],
    ['Gifts', 'gift', '.gift-card'],
    ['Chat', 'chat', '.chat-message'],
    ['Shared', 'share', '.share-card'],
    ['Subscriptions', 'subscribe', '.subscribe-card'],
  ]) {
    await page.getByRole('tab', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host')).toHaveAttribute('data-widget', kind!);
    await expect(page.locator(selector!).first()).toBeVisible();
    await page.getByRole('button', { name: 'Replay preview' }).click();
    await expect(page.locator(selector!).first()).toBeVisible();
    await expect(page.locator('.widget-host')).toHaveCount(1);
  }
  // The sample must remain visible beyond the normal subscription duration.
  await page.waitForTimeout(5500);
  await expect(page.locator('.subscribe-card')).toBeVisible();
  await expect(page.locator('.widgets-preview-frame iframe')).toHaveCount(0);
  expect(gatewayRequests).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('widgets-preview.png') });
  await page.setViewportSize({ width: 520, height: 800 });
  const preview = await page.locator('.widgets-preview-frame').boundingBox();
  const card = await page.locator('.subscribe-card').boundingBox();
  expect(card!.width).toBeLessThanOrEqual(preview!.width);
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
