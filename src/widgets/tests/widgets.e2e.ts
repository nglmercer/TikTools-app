import { expect, test, type Page } from '@playwright/test';

type WidgetTestHook = {
  emitTestFollow: (overrides?: Record<string, unknown>) => void;
  emitTestGift: (overrides?: Record<string, unknown>) => void;
  emitTestGiftCombo: (count: number, overrides?: Record<string, unknown>) => void;
  emitTestChat: (overrides?: Record<string, unknown>) => void;
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

test('gift alerts replace one card at a time during transitions', async ({ page }) => {
  await page.goto('/gift/?duration=900');
  await waitForTestHook(page);
  const card = page.locator('.gift-card');

  await page.evaluate(() => {
    const hook = (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest;
    hook.emitTestGift({ id: 'first-gift', giftName: 'Rose', diamondCount: 1 });
  });
  await expect(card.getByText('Rose')).toBeVisible();

  await page.evaluate(() => {
    const hook = (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest;
    hook.emitTestGift({ id: 'second-gift', giftName: 'Galaxy', diamondCount: 1 });
  });

  const maxCards = await page.evaluate(async () => {
    let max = 0;
    const deadline = performance.now() + 1000;
    while (performance.now() < deadline) {
      max = Math.max(max, document.querySelectorAll('.gift-card').length);
      await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    }
    return max;
  });

  expect(maxCards).toBe(1);
  await expect(card.getByText('Galaxy')).toBeVisible({ timeout: 5000 });
});

test('share alert appears with the viewer name, then disappears', async ({ page }) => {
  await page.goto('/share/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestShare();
  });
  const card = page.locator('.share-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Viewer Name')).toBeVisible();
  await expect(card.getByText('shared the LIVE!')).toBeVisible();
  await expect(card).toBeHidden({ timeout: 5000 });
});

test('subscribe alert appears with the viewer name, then disappears', async ({ page }) => {
  await page.goto('/subscribe/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestSubscribe();
  });
  const card = page.locator('.subscribe-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Viewer Name')).toBeVisible();
  await expect(card.getByText('subscribed to the LIVE!')).toBeVisible();
  await expect(card).toBeHidden({ timeout: 5000 });
});

test('subscribe widget ignores plain joins', async ({ page }) => {
  await page.goto('/subscribe/?duration=800');
  await waitForTestHook(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest.emitTestSubscribe({
      id: 'join-plain',
      action: 1,
    });
  });
  await page.waitForTimeout(700);
  await expect(page.locator('.subscribe-card')).toHaveCount(0);
});

test('chat overlay lists messages in order and honors the limit', async ({ page }) => {
  await page.goto('/chat/?limit=2');
  await waitForTestHook(page);
  await page.evaluate(() => {
    const hook = (window as unknown as { __tiktoolsWidgetTest: WidgetTestHook }).__tiktoolsWidgetTest;
    hook.emitTestChat({ comment: 'first message' });
    hook.emitTestChat({ comment: 'second message' });
    hook.emitTestChat({ comment: 'third message' });
  });
  const list = page.locator('.chat-list');
  await expect(list.getByText('third message')).toBeVisible();
  await expect(page.locator('.chat-message')).toHaveCount(2);
  await expect(list.getByText('first message')).toHaveCount(0);
  await expect(list.getByText('Viewer Name').first()).toBeVisible();
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
test('OBS widgets apply the saved design snapshot from the URL fragment', async ({ page }) => {
  const design = encodeURIComponent(JSON.stringify({ background: '#112233', textColor: '#abcdef', accent: '#ff00ff', radius: 32,
    text: { name: 'Hello {{name}}', title: '', streakTitle: '', message: '<b>Thanks {{name}}</b>' },
  }));
  for (const kind of ['follow', 'gift', 'chat', 'share', 'subscribe']) {
    await page.goto(`/${kind}/#demo=${kind}&design=${design}`);
    const card = page.locator(kind === 'chat' ? '.chat-message' : `.${kind}-card`).first();
    await expect(card).toBeVisible();
    await expect(card).toHaveCSS('background-color', 'rgb(17, 34, 51)');
    await expect(card).toHaveCSS('border-radius', '32px');
    await expect(card.locator(`.${kind}-name`)).toHaveText('Hello Viewer Name');
    await expect(card.locator(`.${kind}-kicker`)).toHaveCount(0);
    await expect(card).toContainText('<b>Thanks Viewer Name</b>');
    await expect(card.locator('b')).toHaveCount(0);
  }
});
