import { expect, test, type Page } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { ROUTE } from '../shared/index.ts';

const labels = { feed: testMessage('en', 'navigation.feed'), points: testMessage('en', 'navigation.points') };
const navLink = (page: Page, name: string) => page.getByRole('navigation', { name: 'Main Navigation' }).getByRole('link', { name, exact: true });

/** Records the class of every element that starts a CSS transition, so a test can see which animations ran. */
const recordTransitions = () => {
  const started: string[] = [];
  Reflect.set(window, '__transitionsStarted', started);
  document.addEventListener('transitionstart', (event) => {
    if (event.target instanceof Element) started.push(`${event.propertyName}:${event.target.className}`);
  }, true);
};

const transitionsStarted = (page: Page) => page.evaluate(() => {
  const started: unknown = Reflect.get(window, '__transitionsStarted');
  return Array.isArray(started) ? started.map(String) : [];
});

test.describe('with motion', () => {
  test.use({ reducedMotion: 'no-preference' });

  test('a screen change fades and settles the new screen, from below going down the sidebar and from above going up', async ({ page }) => {
    await installFakeHost(page, emptyState());
    await page.addInitScript(recordTransitions);
    await page.goto(ROUTE.root);
    await waitForFrontend(page);
    const content = page.locator('.app-layout__content');

    await test.step('going down the sidebar reads as forward', async () => {
      await navLink(page, labels.points).click();
      await expect(content).toHaveAttribute('data-direction', 'forward');
      await expect(page.locator('.points-page')).toBeVisible();
    });

    await test.step('going back up reads as back', async () => {
      await navLink(page, labels.feed).click();
      await expect(content).toHaveAttribute('data-direction', 'back');
      await expect(page.locator('.feed-page')).toBeVisible();
    });

    await test.step('the screens animated opacity through the screen transition classes', async () => {
      const started = await transitionsStarted(page);
      expect(started.some((entry) => entry.startsWith('opacity:') && entry.includes('screen-enter-active'))).toBe(true);
      expect(started.some((entry) => entry.startsWith('opacity:') && entry.includes('screen-leave-active'))).toBe(true);
    });
    await assertNoUnhandledCalls(page);
  });

  test('the accent line under the selected sub-tab slides to the new tab', async ({ page }) => {
    await installFakeHost(page, emptyState());
    await page.goto(`${ROUTE.automations}`);
    await waitForFrontend(page);
    const indicator = page.locator('.base-tabs__indicator').first();
    await expect(indicator).toBeAttached();
    const before = await indicator.evaluate((node) => node.style.transform);

    await page.locator('.base-tabs__tab[aria-selected="false"]').first().click();
    await expect.poll(() => indicator.evaluate((node) => node.style.transform)).not.toBe(before);
    await expect(indicator).toHaveCSS('transition-property', /transform/);
  });
});

test.describe('with reduced motion', () => {
  test.use({ reducedMotion: 'reduce' });

  test('screens change without travelling or fading', async ({ page }) => {
    await installFakeHost(page, emptyState());
    await page.goto(ROUTE.root);
    await waitForFrontend(page);
    const root = page.locator('html');
    await expect(root).toHaveCSS('--motion-shift', '0px');
    await expect(root).toHaveCSS('--duration-normal', '0ms');
    await navLink(page, labels.points).click();
    await expect(page.locator('.points-page')).toBeVisible();
    await expect(page.locator('.screen-enter-active, .screen-leave-active')).toHaveCount(0);
  });
});
