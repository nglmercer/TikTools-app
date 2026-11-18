const t = (key: string): string => testMessage('en', key);
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test } from '@playwright/test';

import { assertNoUnhandledCalls, emitTopic, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('feed rows stay badge-free and open details in the shared panel', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);

  await emitTopic(page, 'live.ui-event', {
    event: {
      kind: 'gift',
      author: 'flor',
      nickname: 'Flor',
      text: 'sent a gift',
      msgId: 'aside-gift-1',
      level: 12,
      pointsDelta: 1.5,
      giftDetails: { name: 'Rose', count: 1, diamonds: 1 },
    },
  });

  const row = page.locator('.event-row').first();
  await expect(row).toBeVisible();
  // Level and points are details, not row decoration.
  await expect(row.locator('.tt-badge-level')).toHaveCount(0);
  await expect(row.locator('.tt-points-badge')).toHaveCount(0);
  // The timestamp is always visible, even when points were earned.
  await expect(row.locator('.event-row__time')).toBeVisible();

  await row.locator('.event-row__content').click();
  await expect(page.locator('.event-details')).toBeVisible();
  await expect(page.locator('.event-details')).toContainText('Level');
  await expect(page.locator('.event-details')).toContainText(t('feed.details.points'));
  await expect(page.locator('.event-details')).toContainText('Rose');

  // Escape closes the panel and returns focus to the row's control.
  await page.keyboard.press('Escape');
  await expect(page.locator('.event-details')).toHaveCount(0);

  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('the avatar opens the user profile in the same panel', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);

  await emitTopic(page, 'live.ui-event', {
    event: { kind: 'chat', author: 'alice', nickname: 'Alice', text: 'hola', msgId: 'aside-chat-1' },
  });

  await page.locator('.event-row').first().locator('.event-row__author').click();
  await expect(page.locator('.feed-inspector .viewer-profile')).toBeVisible();
  await expect(page.locator('.feed-inspector')).toContainText('@alice');

  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
