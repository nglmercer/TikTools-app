import { expect, test } from '@playwright/test';
import {
  demoTestId, demoUrl, docLocales, eventCountText, expectFirstChatMessage, FIRST_CHAT_MESSAGE, frameTestId, guideLink,
  guideTargetSelector, panelFrameFor, qaArtifacts, qaScreenshot, retiredLandingSelectors, selector, stageFrame, stageTestId,
  trackOutbound, trackPageErrors, viewport,
} from './support/index.ts';
import { docCopy } from '../../docs/locales.ts';
import { experienceCopy } from '../../docs/experience/copy.ts';
import { showcaseSteps } from '../../src/web/demo/showcase/script.ts';
import snapshot from '../../docs/experience/releases.json' with { type: 'json' };

/** Connection examples are checked in a narrow embed. */
const CONNECTION_EMBED_VIEWPORT = { width: 600, height: 560 };
/** Focused product pages are checked in a small embedded phone. */
const FOCUSED_PAGE_VIEWPORT = { width: 360, height: 560 };
for (const locale of docLocales) {
  test(`${locale}: visual learning paths and focused examples stay localized and isolated`, async ({ page }) => {
    const copy = experienceCopy[locale];
    const errors = trackPageErrors(page);
    await page.goto(`${locale}/`);
    await expect(page.locator('.docs-topic')).toHaveCount(4);
    await expect(page.locator('.docs-topic').first()).toContainText(copy.topics.connection.title);
    await expect(page.locator('iframe')).toHaveCount(1);
    await expectFirstChatMessage(stageFrame(page));
    await page.locator('.docs-topic').first().click();
    await expect(page).toHaveURL(guideLink('connection', `/${locale}`));
    const panels = page.locator('.demo-frame');
    await expect(panels).toHaveCount(2);
    const connection = page.frameLocator(panelFrameFor('connection'));
    await expect(connection.locator('html')).toHaveAttribute('lang', locale);
    await expect(connection.locator('.connections-page__live')).toBeVisible();
    await expect(connection.locator('.connections-page__explorer')).toBeHidden();
    await expect(connection.locator('.app-layout__navigation')).toBeHidden();
    const handle = connection.locator('#live-handle');
    await handle.fill('isolated_creator');
    const feed = page.frameLocator(panelFrameFor('feed'));
    await expectFirstChatMessage(feed);
    await panels.nth(1).getByTestId(frameTestId.step).click();
    await expect(feed.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(2));
    await panels.first().getByTestId(frameTestId.reset).click();
    await expect(handle).not.toHaveValue('isolated_creator');
    await expect(feed.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(2));
    const full = panels.first().getByRole('link', { name: new RegExp(docCopy[locale].fullDemo) });
    expect(new URL(await full.getAttribute('href') ?? '', page.url()).searchParams.has('embed')).toBe(false);
    if (locale === 'es') {
      await page.evaluate(() => window.scrollTo(0,0));
      await qaScreenshot(page, 'tiktools-redesign-es-guide-desktop.png', { fullPage: true });
      await page.setViewportSize(viewport.mobile);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await expect(page.locator('.VPSidebar')).not.toBeInViewport();
      await qaScreenshot(page, 'tiktools-redesign-es-guide-mobile.png', { fullPage: true });
    }
    expect(errors).toEqual([]);
  });

  // Fixtures are identical in every locale, so this check runs once, in English.
  if (locale === 'en') test(`${locale}: article zoom preserves a desktop viewport, working controls and existing state`, async ({ page }) => {
    await test.step('open the feed demo page', async () => {
      await page.goto(`${locale}/user/live/feed.html`);
    });
    const panel = page.locator('.demo-frame');
    const frame = (await (await panel.locator('iframe').elementHandle())!.contentFrame())!;
    await test.step('check the demo is isolated at laptop width and play its first step', async () => {
      await expect(frame.locator(selector.demoControls)).toBeHidden();
      await expect(panel.getByTestId(frameTestId.step)).toBeEnabled();
      expect(await frame.evaluate(() => innerWidth)).toBe(viewport.laptop.width);
      await panel.getByTestId(frameTestId.step).click();
      await expect(frame.locator(selector.liveFeed)).toContainText(FIRST_CHAT_MESSAGE);
    });
    const search = frame.locator(`${selector.liveFeed} input[type="search"]`);
    const instance = new URL(frame.url()).searchParams.get('instance');
    await test.step('zoom through each scale, then search and check the fit layout', async () => {
      for (const [scale,width] of [['0.5',640],['0.75',960],['1',1280]] as const) {
        await panel.getByTestId(frameTestId.zoom).selectOption(scale);
        expect(await frame.evaluate(() => innerWidth)).toBe(viewport.laptop.width);
        expect(Math.round((await panel.locator('iframe').boundingBox())!.width)).toBe(width);
        expect(new URL(frame.url()).searchParams.get('instance')).toBe(instance);
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      }
      await panel.getByTestId(frameTestId.zoom).selectOption('0.5');
      await search.click();
      await search.fill('Luna');
      await panel.getByTestId(frameTestId.zoom).selectOption('fit');
      await expect(search).toHaveValue('Luna');
      const alignment = await frame.locator('.feed-page').evaluate(node => {
        const stream=node.querySelector('.feed-page__stream')!.getBoundingClientRect();
        const inspector=node.querySelector('.feed-page__inspector')!.getBoundingClientRect();
        return { top:Math.abs(stream.top-inspector.top), right:inspector.left >= stream.right };
      });
      expect(alignment.top).toBeLessThan(2);
      expect(alignment.right).toBe(true);
    });
    await test.step('reset the demo and toggle autoplay', async () => {
      await panel.getByTestId(frameTestId.reset).click();
      await expect(search).toHaveValue('');
      await expect(panel.getByTestId(frameTestId.step)).toBeEnabled();
      expect(new URL(frame.url()).searchParams.get('instance')).toBe(instance);
      await panel.getByTestId(frameTestId.play).click();
      await expect(panel.getByTestId(frameTestId.play)).toHaveAttribute('aria-pressed','true');
      await panel.getByTestId(frameTestId.play).click();
      await expect(panel.getByTestId(frameTestId.play)).toHaveAttribute('aria-pressed','false');
    });
    await test.step('capture review screenshots when QA artifacts are enabled', async () => {
      // Review-only captures: no assertion follows, so the whole block is skipped outside QA runs.
      if (qaArtifacts) {
        await panel.getByTestId(frameTestId.step).click();
        await panel.scrollIntoViewIfNeeded();
        await qaScreenshot(panel, 'tiktools-scaled-feed-fit.png');
        await panel.getByTestId(frameTestId.zoom).selectOption('0.5');
        await qaScreenshot(panel, 'tiktools-scaled-feed-50.png');
      }
    });
  });

  test(`${locale}: product landing shows one quiet demo and keeps the guides reachable`, async ({ page, baseURL }) => {
    const errors = trackPageErrors(page);
    const outbound = trackOutbound(page, baseURL!);
    await page.setViewportSize(viewport.desktop);
    await page.goto(`${locale}/`);
    for (const retired of retiredLandingSelectors) {
      await expect(page.locator(retired)).toHaveCount(0);
    }
    await expect(page.locator('iframe')).toHaveCount(1);
    const frame = stageFrame(page);
    await expectFirstChatMessage(frame);
    await expect(page.locator(selector.stageStep)).toHaveCount(showcaseSteps.length);
    // Events arrive by themselves; the first gift carries the same-origin Rose image.
    const rose = frame.locator('img.event-card__gift').first();
    await rose.scrollIntoViewIfNeeded();
    await expect(rose).toHaveAttribute('alt', 'Rose');
    await expect.poll(() => rose.evaluate(node => (node as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
    expect(new URL((await rose.getAttribute('src'))!,page.url()).origin).toBe(new URL(baseURL!).origin);
    await expect(page.locator(selector.stageGuideLink)).toHaveAttribute('href', guideLink('feed', `/${locale}`));
    await page.getByTestId(stageTestId.step('overlays')).click();
    await expect(frame.locator(guideTargetSelector('overlay-preview'))).toBeVisible();
    await expect(page.locator(selector.stageGuideLink)).toHaveAttribute('href', guideLink('overlays', `/${locale}`));
    const source = await page.locator(selector.stageFrame).getAttribute('src');
    await page.locator('.docs-home__actions a').first().click();
    await expect(page).toHaveURL(/#playground$/);
    await expect(page.locator(selector.stageFrame)).toHaveAttribute('src',source!);
    expect(errors).toEqual([]);
    expect(outbound).toEqual([]);
  });

  test(`${locale}: connection examples fail accurately and recover without a session cookie`, async ({ page }) => {
    const copy = experienceCopy[locale].connectionStates;
    await page.setViewportSize(CONNECTION_EMBED_VIEWPORT);
    await page.goto(demoUrl({ scenario: 'connection', embed: 'connection', locale }));
    await expect(page.getByTestId(demoTestId.reset)).toBeEnabled();
    const form = page.locator('.connections-page__live');
    await form.locator('#live-handle').fill('demo_creator');
    for (const state of ['offline', 'auth'] as const) {
      await page.getByRole('combobox', { name: copy.label, exact: true }).click();
      await page.getByRole('option', { name: copy[state], exact: true }).click();
      await form.locator('.base-button--primary').click();
      // The dock no longer repeats the help text; the failure explains itself on hover.
      await expect(page.locator('#app-spotlight')).toBeVisible();
      await form.locator('.diagnostic-action').hover();
      await expect(page.locator('#app-tooltip')).toHaveText(state === 'offline' ? copy.offlineHelp : copy.authHelp);
      await expect(form.locator('#live-handle')).toBeEnabled();
      await expect(page.getByTestId(demoTestId.metrics)).toContainText('"ticks":0');
      await form.locator('.diagnostic-action').click();
      await expect(page.locator('#app-spotlight')).toBeHidden();
      await expect(page.locator('.debug-page')).toBeVisible();
      await page.locator(guideTargetSelector('diagnostics-back')).click();
      await expect(form.locator('#live-handle')).toBeEnabled();
    }
    await page.getByRole('combobox', { name: copy.label, exact: true }).click();
    await page.getByRole('option', { name: copy.normal, exact: true }).click();
    await form.locator('.base-button--primary').click();
    await expect(form.locator('#live-handle')).toBeDisabled();
    await expect(form.locator('#live-cookie')).toHaveCount(0);
    await page.getByTestId(demoTestId.reset).click();
    await expect(form.locator('#live-handle')).toBeEnabled();
    await expect(page.locator('.demo-toolbar__recovery')).toHaveCount(0);
  });

  test(`${locale}: installation tabs support keyboard navigation and retain direct platform anchors`, async ({ page }) => {
    await page.goto(`${locale}/user/installation.html`);
    expect(await page.locator('.doc-requirements li').count()).toBeGreaterThanOrEqual(3);
    const tabs = page.locator('.doc-tabs');
    const windows = tabs.getByRole('tab', { name:'Windows', exact:true });
    const linux = tabs.getByRole('tab', { name:'Linux', exact:true });
    await expect(windows).toHaveAttribute('aria-selected','true');
    await expect(tabs.getByRole('tabpanel')).toContainText('installer.exe');
    await linux.click();
    await expect(tabs.getByRole('tabpanel')).toContainText('AppImage');
    await linux.press('Home');
    await expect(windows).toBeFocused();
    await expect(windows).toHaveAttribute('aria-selected','true');
    await windows.press('End');
    await expect(linux).toBeFocused();
    await linux.press('ArrowRight');
    await expect(windows).toBeFocused();
    await page.goto(`${locale}/user/installation.html#linux`);
    await expect(page.locator('h2#linux')).toBeInViewport();
    await page.setViewportSize(viewport.mobile);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (locale === 'es') {
      await expect(page.locator('.VPSidebar')).not.toBeInViewport();
      if (qaArtifacts) {
        await page.evaluate(() => window.scrollTo(0,0));
        await qaScreenshot(page, 'tiktools-redesign-es-installation-mobile.png', { fullPage: true });
      }
    }
  });

  test(`${locale}: release cards use published assets and filter by operating system`, async ({ page }) => {
    await page.goto(`${locale}/releases/`);
    await expect(page.locator('.release-card')).toHaveCount(snapshot.releases.length);
    const first = page.locator('.release-card').first();
    await expect(first).toContainText(snapshot.releases[0]!.tag);
    for (const system of ['Windows', 'Linux']) {
      await page.getByRole('button', { name: system, exact: true }).click();
      const links = page.locator('.release-card__assets a');
      expect(await links.count()).toBeGreaterThan(0);
      for (const href of await links.evaluateAll((nodes) => nodes.map((node) => (node as HTMLAnchorElement).href))) {
        expect(href.toLowerCase()).toContain(system.toLowerCase());
        expect(snapshot.releases.flatMap((release) => release.assets).some((asset) => asset.url === href)).toBe(true);
      }
    }
    await page.setViewportSize(viewport.mobile);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (locale === 'es') await qaScreenshot(page, 'tiktools-redesign-es-releases-mobile.png', { fullPage: true });
  });
}

test('all focused product pages fit an embedded mobile viewport and preserve their real controls', async ({ page }) => {
  const errors = trackPageErrors(page);
  await page.setViewportSize(FOCUSED_PAGE_VIEWPORT);
  for (const [panel, root] of [['automation', '.automations-page'], ['points', '.points-page'], ['overlays', '.overlays-page']] as const) {
    await page.goto(demoUrl({ scenario: panel, embed: panel }));
    await expect(page.getByTestId(demoTestId.play)).toBeEnabled();
    await expect(page.locator(root)).toBeVisible();
    await expect(page.locator('.app-menu-bar')).toBeHidden();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.getByTestId(demoTestId.play).click();
    await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(1));
  }
  expect(errors).toEqual([]);
});
