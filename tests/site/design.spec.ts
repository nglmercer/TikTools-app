import { expect, test } from '@playwright/test';
import {
  demoTestId, demoUrl, docLocales, expectFirstChatMessage, frameTestId, guideLink, retiredLandingSelectors, selector, stageFrame, stageTestId, trackPageErrors, viewport,
} from './support/index.ts';
import { docCopy } from '../../docs/locales.ts';
import { experienceCopy } from '../../docs/experience/copy.ts';
import { showcaseSteps } from '../../src/web/demo/showcase/script.ts';

/** Width between the desktop and phone checks of the demo toolbar. */
const INTERMEDIATE_WIDTH = 1058;
/** Pixel tolerance for toolbar and shell edges. */
const LAYOUT_TOLERANCE_PX = 2;
/** Hero heading size: at least this on desktop, at most the second value on a phone. */
const MIN_HERO_FONT_PX = 32;
const MAX_MOBILE_HERO_FONT_PX = 56;

test('demo toolbar shares real controls, contains menus and fits desktop and mobile', async ({ page }) => {
  const errors = trackPageErrors(page);
  const more = page.getByTestId(demoTestId.more);
  const play = page.getByTestId(demoTestId.play);
  await page.goto(demoUrl({ scenario: 'connection' }));
  await expect(more).toBeEnabled();
  await expect(play).toHaveClass(/base-button/);
  await play.click();
  await expect(play).toHaveAttribute('aria-pressed', 'true');
  await play.click();
  await expect(play).toHaveAttribute('aria-pressed', 'false');
  await more.click();
  await page.getByRole('menuitem', { name: 'Component reference', exact: true }).click();
  await expect(page).toHaveURL(/#\/components\/buttons$/);
  await expect(page.locator('.component-catalog')).toBeVisible();
  await more.click();
  await page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' }).click();
  await more.click();
  await expect(page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' })).toHaveAttribute('aria-checked', 'true');
  await page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' }).click();
  for (const width of [viewport.desktop.width, INTERMEDIATE_WIDTH, viewport.mobile.width]) {
    await page.setViewportSize({ width, height: viewport.desktop.height });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const toolbar = await page.locator(selector.demoControls).boundingBox();
    const workspace = await page.locator('#app').boundingBox();
    expect(toolbar && workspace && Math.abs(toolbar.y + toolbar.height - workspace.y) < LAYOUT_TOLERANCE_PX).toBe(true);
    const shell = await page.locator(selector.appLayout).boundingBox();
    expect(shell && shell.y + shell.height <= viewport.desktop.height + LAYOUT_TOLERANCE_PX).toBe(true);
    await expect(page.getByRole('button', { name: /^Diagnostics/ })).toBeInViewport();
    await expect(page.getByTestId(demoTestId.reset)).toBeInViewport();
  }
  // The section comes from workspace navigation now; the toolbar has no scenario picker.
  await expect(page.getByRole('combobox', { name: 'Scenario', exact: true })).toHaveCount(0);
  await page.setViewportSize(viewport.desktop);
  await page.getByRole('link', { name: 'Points', exact: true }).click();
  await expect(page).toHaveURL(/#\/points$/);
  await expect(play).toBeVisible();
  expect(errors).toEqual([]);
  await page.setViewportSize(viewport.laptop);
  await page.screenshot({ path: '/tmp/tiktools-docs-design-demo-desktop.png' });
  await page.setViewportSize(viewport.mobile);
  await page.screenshot({ path: '/tmp/tiktools-docs-design-demo-mobile.png' });
});

test('the landing tour jumps between real views and docks the demo controls in the bar', async ({ page }) => {
  await page.goto('');
  const frame = page.locator(selector.stageFrame);
  await expect(frame).toHaveAttribute('src', /showcase=1/);
  // One landing step per showcase step.
  await expect(page.locator(selector.stageStep)).toHaveCount(showcaseSteps.length);
  const tourToggle = page.getByTestId(stageTestId.tourToggle);
  const pointsStep = page.getByTestId(stageTestId.step('points'));
  await expect(tourToggle).toHaveAttribute('aria-pressed', 'true');
  const source = await frame.getAttribute('src');
  await pointsStep.click();
  await expect(page.locator(selector.stageLesson)).toHaveText(experienceCopy.en.topics.points.title);
  await expect(pointsStep).toHaveAttribute('aria-current', 'step');
  await expect(stageFrame(page).locator('.points-leaderboard')).toBeVisible();
  await expect(page.locator(selector.stageGuideLink)).toHaveAttribute('href', guideLink('points'));
  // Jumping between steps must never reload the demo.
  expect(await frame.getAttribute('src')).toBe(source);
  await tourToggle.click();
  await expect(tourToggle).toHaveAttribute('aria-pressed', 'false');
  const stageBar = page.locator(selector.stageBar);
  await expect(stageBar.locator('.demo-frame__overlay a')).toBeVisible();
  await expect(stageBar.getByTestId(frameTestId.zoom)).toBeVisible();
  await expect(stageBar.locator('.compact-select__value')).toHaveText(/^\d+%$/);
});

test('mobile shows one full-name language selector at the top of the menu', async ({ page }) => {
  await page.setViewportSize(viewport.mobile);
  await page.goto('es/');
  await expect(page.locator('.site-language--bar')).toBeHidden();
  await page.getByRole('button', { name: docCopy.es.mobileMenu, exact: true }).click();
  const languageSelector = page.locator('.site-language--screen');
  await expect(languageSelector).toHaveCount(1);
  await expect(languageSelector).toBeVisible();
  await expect(languageSelector).toHaveText('Español');
});

test('the embedded demo follows the documentation theme without reloading', async ({ page }) => {
  await page.goto('');
  const frame = stageFrame(page);
  await expect(frame.locator(selector.appLayout)).toBeVisible();
  const src = await page.locator(selector.stageFrame).getAttribute('src');
  const themeToggle = page.locator('.VPNavBarAppearance button[role="switch"]');
  await themeToggle.click();
  await expect(page.locator('html')).toHaveClass(/dark/);
  await expect(frame.locator('html')).toHaveAttribute('data-theme', 'dark');
  await themeToggle.click();
  await expect(frame.locator('html')).toHaveAttribute('data-theme', 'light');
  expect(await page.locator(selector.stageFrame).getAttribute('src')).toBe(src);
});

test('the embedded demo hands vertical scroll back to the landing page', async ({ page }) => {
  await page.goto('es/');
  const frame = stageFrame(page);
  await expect(frame.locator(selector.appLayout)).toBeVisible();
  const box = await page.locator(selector.stageFrame).boundingBox();
  expect(box).not.toBeNull();
  // Aim at the visible top of the frame: its scaled canvas is taller than the viewport.
  await page.mouse.move(box!.x + box!.width / 2, box!.y + 80);
  // Chromium hands the delta to the article once the frame has nothing left to scroll.
  await page.mouse.wheel(0, 400);
  await page.mouse.wheel(0, 400);
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);
});

test('product landing pages show the real demo automatically and fit every viewport', async ({ page }) => {
  for (const route of ['', ...docLocales.map((locale) => `${locale}/`)]) {
    await page.goto(route);
    const title = page.locator('h1'); await expect(title).toHaveCount(1);
    expect(await title.evaluate((node) => parseFloat(getComputedStyle(node).fontSize))).toBeGreaterThanOrEqual(MIN_HERO_FONT_PX);
    await expect(page.locator('#choose-your-language')).toHaveCount(0);
    await expect(page.locator('.docs-home__eyebrow')).toHaveCSS('white-space', 'nowrap');
    await expect(page.locator('main a[href*="/user/"]').first()).toBeVisible();
    await expect(page.locator('main a[href*="/developer/"]').first()).toBeVisible();
    for (const retired of retiredLandingSelectors) {
      await expect(page.locator(retired)).toHaveCount(0);
    }
    await expect(page.locator('iframe')).toHaveCount(1);
    await expectFirstChatMessage(stageFrame(page));
    await page.setViewportSize(viewport.mobile);
    expect(await title.evaluate((node) => parseFloat(getComputedStyle(node).fontSize))).toBeLessThanOrEqual(MAX_MOBILE_HERO_FONT_PX);
    const layout = await page.evaluate(() => ({ width: document.documentElement.scrollWidth, viewport: innerWidth, overflow: [...document.querySelectorAll('main *')].filter((node) => node.getBoundingClientRect().right > innerWidth + 1).slice(0, 8).map((node) => ({ element: node.tagName, class: node.className, right: node.getBoundingClientRect().right })) }));
    expect(layout.width, JSON.stringify(layout.overflow)).toBeLessThanOrEqual(layout.viewport);
    await page.screenshot({ path: `/tmp/tiktools-docs-design-${route ? route.slice(0, 2) : 'root'}-mobile.png`, fullPage: true });
    await page.setViewportSize(viewport.desktop);
    await page.screenshot({ path: `/tmp/tiktools-docs-design-${route ? route.slice(0, 2) : 'root'}-desktop.png`, fullPage: true });
  }
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('');
  await expect(page.locator('html')).toHaveClass(/dark/);
  await page.screenshot({ path: '/tmp/tiktools-docs-design-root-dark.png', fullPage: true });
  await page.goto('es/');
  await expectFirstChatMessage(stageFrame(page));
  await page.screenshot({ path: '/tmp/tiktools-docs-design-es-dark.png', fullPage: true });
});
