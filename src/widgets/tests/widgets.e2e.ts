import { expect, test, type Page } from '@playwright/test';

type WidgetTestHook = {
  emitTestFollow: (overrides?: Record<string, unknown>) => void;
  emitTestGift: (overrides?: Record<string, unknown>) => void;
  emitTestGiftCombo: (count: number, overrides?: Record<string, unknown>) => void;
};

async function waitForTestHook(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as unknown as Record<string, unknown>)['__tiktoolsWidgetTest'] !== undefined,
      ),
    )
    .toBe(true);
}

test('follow widget keeps a transparent OBS stage', async ({ page }) => {
  await page.goto('/follow/');
  await waitForTestHook(page);
  const backgrounds = await page.evaluate(() => {
    const names: string[] = [];
    for (const element of [document.documentElement, document.body]) {
      names.push(getComputedStyle(element).backgroundColor);
    }
    return names;
  });
  expect(backgrounds).toEqual(['rgba(0, 0, 0, 0)', 'rgba(0, 0, 0, 0)']);
  await expect(page.locator('.follow-card')).toHaveCount(0);
});

test('follow alert appears with the viewer name, then disappears', async ({ page }) => {
  await page.goto('/follow/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestFollow();
  });
  const card = page.locator('.follow-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Viewer Name')).toBeVisible();
  await expect(card).toBeHidden({ timeout: 5000 });
});

test('duplicate and historical follows never render', async ({ page }) => {
  await page.goto('/follow/?duration=600');
  await waitForTestHook(page);
  // Same event id twice: the second delivery is dropped.
  await page.evaluate(() => {
    const hook = (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest;
    hook.emitTestFollow({ id: 'dup-1' });
    hook.emitTestFollow({ id: 'dup-1' });
  });
  const card = page.locator('.follow-card');
  await expect(card).toBeVisible();
  await expect(card).toBeHidden({ timeout: 5000 });
  // No second alert was queued by the duplicate.
  await expect(card).toHaveCount(0);
  // Historical events are ignored outright.
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestFollow({
      id: 'hist-1',
      isHistory: true,
    });
  });
  await page.waitForTimeout(700);
  await expect(card).toHaveCount(0);
});

test('gift combo updates one alert in place, then completes', async ({ page }) => {
  await page.goto('/gift/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestGiftCombo(5, {
      giftName: 'Rose',
      diamondCount: 1,
    });
  });
  const card = page.locator('.gift-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Rose')).toBeVisible();
  await expect(card.getByText('×5')).toBeVisible();
  await expect(card.getByText('5 diamonds')).toBeVisible();
  // Exactly one card for the whole streak, then it clears.
  await expect(card).toHaveCount(1);
  await expect(card).toBeHidden({ timeout: 5000 });
});

test('gift widget falls back when the artwork fails to load', async ({ page }) => {
  await page.goto('/gift/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestGift({
      giftName: 'Galaxy',
      giftIconUrl: 'http://127.0.0.1:9/missing.png',
    });
  });
  const card = page.locator('.gift-card');
  await expect(card).toBeVisible();
  await expect(card.locator('.gift-art svg')).toBeVisible();
  await expect(card.getByText('Galaxy')).toBeVisible();
});
