import { test, expect } from '@playwright/test';

test('the showcase walks the real views on its own, with a spotlight and a tooltip', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('demo/?showcase=1&locale=en&instance=showcase-auto');
  await expect(page.locator('html')).toHaveAttribute('data-demo-showcase', 'true');
  await expect(page.locator('.showcase[data-on="true"] .showcase__tip')).toHaveText('Events arrive in real time');
  await expect(page.locator('.live-feed')).toContainText('Hello TikTools!');
  // The inspector profile is opened by the registered guide action, not by synthetic clicks.
  await expect(page.locator('.showcase__tip')).toHaveText('Open any viewer’s profile', { timeout: 15_000 });
  await expect(page.locator('[data-guide="feed-inspector-profile"]')).toBeVisible();
  await expect(page.locator('#demo-controls')).toBeHidden();
  expect(errors).toEqual([]);
});

test('a recording run plays every step once and flags the end', async ({ page }) => {
  test.setTimeout(90_000);
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('demo/?showcase=1&record=1&framed=1&locale=en&instance=showcase-record');
  const visited = new Set<string>();
  await expect.poll(async () => {
    const text = await page.locator('.showcase__tip').textContent().catch(() => null);
    if (text) visited.add(text);
    return page.locator('html').getAttribute('data-showcase-done');
  }, { timeout: 60_000, intervals: [250] }).toBe('true');
  expect([...visited]).toEqual(expect.arrayContaining(['Events arrive in real time', 'Open any viewer’s profile', 'Turn events into actions', 'Preview it and add it to OBS', 'Points update with every event']));
  expect(errors).toEqual([]);
});

test('reduced motion shows the first step still and waits for the viewer', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('demo/?showcase=1&locale=en&instance=showcase-still');
  await expect(page.locator('.showcase[data-still="true"][data-on="true"]')).toBeVisible();
  await expect(page.locator('.live-feed')).toContainText('Hello TikTools!');
  await page.waitForTimeout(2500);
  await expect(page.locator('.showcase__tip')).toHaveText('Events arrive in real time');
  await expect(page.locator('html')).not.toHaveAttribute('data-showcase-done', 'true');
});

test('showcase options cannot be mixed with a tour or a focused panel', async ({ page }) => {
  await page.goto('demo/?showcase=1&embed=feed&locale=en');
  await expect(page.getByRole('alert')).toHaveText('The showcase runs on the feed scenario without a tour, panel or preview.');
});
