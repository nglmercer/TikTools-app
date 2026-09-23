import { expect, test, type Page } from '@playwright/test';

type WidgetTestHook = {
  emitTestFollow: (overrides?: Record<string, unknown>) => void;
  emitTestGift: (overrides?: Record<string, unknown>) => void;
  emitTestShare: (overrides?: Record<string, unknown>) => void;
  emitTestSubscribe: (overrides?: Record<string, unknown>) => void;
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

/** Card box after the enter transition has settled (no transform skew). */
async function settledBox(page: Page, selector: string): Promise<{ width: number; height: number }> {
  const card = page.locator(selector);
  await expect(card).toBeVisible();
  await page.waitForTimeout(600);
  const box = await card.boundingBox();
  expect(box).not.toBeNull();
  return { width: box?.width ?? -1, height: box?.height ?? -1 };
}

test('follow card keeps stable geometry across show/hide and name lengths', async ({ page }) => {
  await page.goto('/follow/?duration=900');
  await waitForTestHook(page);
  const emit = (overrides: Record<string, unknown>): Promise<void> =>
    page.evaluate((o) => {
      (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestFollow(o);
    }, overrides);
  const card = page.locator('.follow-card');

  await emit({ user: { nickname: 'Al', uniqueId: 'al' } });
  const narrow = await settledBox(page, '.follow-card');
  await expect(card).toBeHidden({ timeout: 5000 });

  await emit({
    user: { nickname: 'A Much Longer Display Name Here', uniqueId: 'very_long_unique_id_here' },
  });
  const wide = await settledBox(page, '.follow-card');

  // Consecutive alerts neither resize nor re-center the card.
  expect(wide.width).toBe(narrow.width);
  expect(wide.height).toBe(narrow.height);
  // Long names clip inside the card instead of growing it.
  const nameBox = await card.locator('.follow-name').boundingBox();
  expect(nameBox?.width).toBeLessThanOrEqual(wide.width);
});

for (const kind of ['share', 'subscribe'] as const) {
  test(`${kind} card keeps stable geometry across show/hide and name lengths`, async ({ page }) => {
    await page.goto(`/${kind}/?duration=900`);
    await waitForTestHook(page);
    const emit = kind === 'share' ? 'emitTestShare' : 'emitTestSubscribe';
    const send = (overrides: Record<string, unknown>): Promise<void> =>
      page.evaluate(
        ([method, o]) => {
          (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest[method](o);
        },
        [emit, overrides] as const,
      );
    const selector = `.${kind}-card`;
    const card = page.locator(selector);
    await send({ user: { nickname: 'Al', uniqueId: 'al' } });
    const narrow = await settledBox(page, selector);
    await expect(card).toBeHidden({ timeout: 5000 });
    await send({
      user: { nickname: 'A Much Longer Display Name Here', uniqueId: 'very_long_unique_id_here' },
    });
    const wide = await settledBox(page, selector);
    expect(wide.width).toBe(narrow.width);
    expect(wide.height).toBe(narrow.height);
  });
}

test('gift card keeps stable geometry while the streak count ticks', async ({ page }) => {
  await page.goto('/gift/?duration=900&comboTimeout=60000');
  await waitForTestHook(page);
  const card = page.locator('.gift-card');
  const boxes: Array<{ width: number; height: number }> = [];
  for (let step = 1; step <= 6; step += 1) {
    await page.evaluate((count) => {
      (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestGift({
        groupId: 'geo-stable',
        giftName: 'Rose',
        diamondCount: 1,
        repeatCount: count,
        comboCount: count,
        streakable: true,
        repeatEnd: false,
      });
    }, step);
    await expect(card.getByText(`×${step}`)).toBeVisible();
    if (step === 1) await page.waitForTimeout(600);
    const box = await card.boundingBox();
    expect(box).not.toBeNull();
    boxes.push({ width: box?.width ?? -1, height: box?.height ?? -1 });
  }
  for (const box of boxes) {
    expect(box.width).toBe(boxes[0]?.width);
    expect(box.height).toBe(boxes[0]?.height);
  }
});
