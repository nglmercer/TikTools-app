import { test, expect } from '@playwright/test';
import { docCopy, docLocales } from '../../docs/locales.ts';
import { experienceCopy } from '../../docs/experience/copy.ts';
import snapshot from '../../docs/experience/releases.json' with { type: 'json' };

for (const locale of docLocales) {
  test(`${locale}: visual learning paths and focused examples stay localized and isolated`, async ({ page }) => {
    const copy = experienceCopy[locale];
    const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`${locale}/`);
    await expect(page.locator('.docs-topic')).toHaveCount(4);
    await expect(page.locator('.docs-topic').first()).toContainText(copy.topics.connection.title);
    await expect(page.locator('iframe')).toHaveCount(1);
    await expect(page.frameLocator('.product-stage iframe').locator('.live-feed')).toContainText('Hello TikTools!');
    await page.locator('.docs-topic').first().click();
    await expect(page).toHaveURL(new RegExp(`/${locale}/user/first-live.html$`));
    const panels = page.locator('.demo-frame');
    await expect(panels).toHaveCount(2);
    await panels.first().getByRole('button', { name: docCopy[locale].loadDemo, exact: true }).click();
    const connection = page.frameLocator('.demo-frame[data-panel="connection"] iframe');
    await expect(connection.locator('html')).toHaveAttribute('lang', locale);
    await expect(connection.locator('.connections-page__live')).toBeVisible();
    await expect(connection.locator('.connections-page__explorer')).toBeHidden();
    await expect(connection.locator('.app-layout__navigation')).toBeHidden();
    const handle = connection.locator('#live-handle');
    await handle.fill('isolated_creator');
    await panels.nth(1).getByRole('button', { name: docCopy[locale].loadDemo, exact: true }).click();
    const feed = page.frameLocator('.demo-frame[data-panel="feed"] iframe');
    await expect(feed.locator('.live-feed')).toContainText('Hello TikTools!');
    await panels.nth(1).getByTestId('frame-step').click();
    await expect(feed.getByTestId('demo-event-count')).toContainText('2 / 300');
    await panels.first().getByTestId('frame-reset').click();
    await expect(handle).not.toHaveValue('isolated_creator');
    await expect(feed.getByTestId('demo-event-count')).toContainText('2 / 300');
    const full = panels.first().getByRole('link', { name: new RegExp(docCopy[locale].fullDemo) });
    expect(new URL(await full.getAttribute('href') ?? '', page.url()).searchParams.has('embed')).toBe(false);
    if (locale === 'es') {
      await page.evaluate(() => window.scrollTo(0,0));
      await page.screenshot({ path:'/tmp/tiktools-redesign-es-guide-desktop.png', fullPage:true });
      await page.setViewportSize({ width:390, height:844 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await expect(page.locator('.VPSidebar')).not.toBeInViewport();
      await page.screenshot({ path:'/tmp/tiktools-redesign-es-guide-mobile.png', fullPage:true });
    }
    expect(errors).toEqual([]);
  });

  test(`${locale}: article zoom preserves a desktop viewport, working controls and existing state`, async ({ page }) => {
    await page.goto(`${locale}/user/live/feed.html`);
    const panel = page.locator('.demo-frame');
    await panel.getByRole('button', { name:docCopy[locale].loadDemo,exact:true }).click();
    const frame = (await (await panel.locator('iframe').elementHandle())!.contentFrame())!;
    await expect(frame.locator('#demo-controls')).toBeHidden();
    await expect(panel.getByTestId('frame-step')).toBeEnabled();
    expect(await frame.evaluate(() => innerWidth)).toBe(1280);
    await panel.getByTestId('frame-step').click();
    await expect(frame.locator('.live-feed')).toContainText('Hello TikTools!');
    const search = frame.locator('.live-feed input[type="search"]');
    const instance = new URL(frame.url()).searchParams.get('instance');
    for (const [scale,width] of [['0.5',640],['0.75',960],['1',1280]] as const) {
      await panel.getByTestId('frame-zoom').selectOption(scale);
      expect(await frame.evaluate(() => innerWidth)).toBe(1280);
      expect(Math.round((await panel.locator('iframe').boundingBox())!.width)).toBe(width);
      expect(new URL(frame.url()).searchParams.get('instance')).toBe(instance);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }
    await panel.getByTestId('frame-zoom').selectOption('0.5');
    await search.click(); await search.fill('Luna');
    await panel.getByTestId('frame-zoom').selectOption('fit');
    await expect(search).toHaveValue('Luna');
    const alignment = await frame.locator('.feed-page').evaluate(node => { const stream=node.querySelector('.feed-page__stream')!.getBoundingClientRect(); const inspector=node.querySelector('.feed-page__inspector')!.getBoundingClientRect(); return { top:Math.abs(stream.top-inspector.top), right:inspector.left >= stream.right }; });
    expect(alignment.top).toBeLessThan(2); expect(alignment.right).toBe(true);
    await panel.getByTestId('frame-reset').click();
    await expect(search).toHaveValue('');
    await expect(panel.getByTestId('frame-step')).toBeEnabled();
    expect(new URL(frame.url()).searchParams.get('instance')).toBe(instance);
    await panel.getByTestId('frame-play').click();
    await expect(panel.getByTestId('frame-play')).toHaveAttribute('aria-pressed','true');
    await panel.getByTestId('frame-play').click();
    await expect(panel.getByTestId('frame-play')).toHaveAttribute('aria-pressed','false');
    if (locale === 'en') {
      await panel.getByTestId('frame-step').click();
      await panel.scrollIntoViewIfNeeded();
      await panel.screenshot({ path:'/tmp/tiktools-scaled-feed-fit.png' });
      await panel.getByTestId('frame-zoom').selectOption('0.5');
      await panel.screenshot({ path:'/tmp/tiktools-scaled-feed-50.png' });
    }
  });

  test(`${locale}: product landing loads real controls directly and switches one stable workspace`, async ({ page,baseURL }) => {
    const errors: string[] = []; const outbound: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', request => { if (!request.url().startsWith(baseURL!) && !/^(data:|blob:)/.test(request.url())) outbound.push(request.url()); });
    await page.setViewportSize({ width:1440,height:960 });
    await page.goto(`${locale}/`);
    const stage = page.locator('.product-stage__window');
    await expect(page.locator('iframe')).toHaveCount(1);
    const frame = page.frameLocator('.product-stage iframe');
    await expect(frame.locator('.live-feed')).toContainText('Hello TikTools!');
    await expect(frame.getByTestId('demo-event-count')).toContainText('9 / 300');
    await expect(page.locator('.product-stage iframe')).toHaveAttribute('loading','eager');
    const height = (await stage.boundingBox())!.height;
    const source = await page.locator('.product-stage iframe').getAttribute('src');
    await page.locator('.docs-home__actions a').first().click();
    await expect(page).toHaveURL(/#playground$/);
    await expect(page.locator('.product-stage iframe')).toHaveAttribute('src',source!);
    for (const [view,selector] of [['connection','#live-handle'],['automation','.automations-page'],['overlays','.overlays-page'],['points','.points-page'],['feed','.live-feed']] as const) {
      await page.locator(`#stage-${view}-tab`).click();
      await expect(frame.locator('html')).toHaveAttribute('data-demo-panel',view);
      await expect(frame.locator(selector)).toBeVisible();
      await expect(page.locator('iframe')).toHaveCount(1);
      expect(Math.abs((await stage.boundingBox())!.height-height)).toBeLessThanOrEqual(1);
      await expect(page.locator(`#stage-${view}-tab`)).toHaveAttribute('aria-selected','true');
    }
    await page.locator('#stage-feed-tab').press('End');
    await expect(page.locator('#stage-points-tab')).toBeFocused();
    await expect(frame.locator('.points-page')).toBeVisible();
    const full = new URL((await page.locator('.product-stage__bar > a').getAttribute('href'))!,page.url());
    expect(full.searchParams.get('scenario')).toBe('points');
    expect(full.searchParams.has('embed')).toBe(false);
    expect(errors).toEqual([]); expect(outbound).toEqual([]);
  });

  test(`${locale}: connection examples fail accurately and recover without a session cookie`, async ({ page }) => {
    const copy = experienceCopy[locale].connectionStates;
    await page.setViewportSize({ width:600, height:560 });
    await page.goto(`demo/?scenario=connection&embed=connection&locale=${locale}`);
    await expect(page.getByTestId('demo-reset')).toBeEnabled();
    const form = page.locator('.connections-page__live');
    await form.locator('#live-handle').fill('demo_creator');
    for (const state of ['offline', 'auth'] as const) {
      await page.getByRole('combobox', { name: copy.label, exact: true }).click();
      await page.getByRole('option', { name: copy[state], exact: true }).click();
      await form.locator('.base-button--primary').click();
      await expect(page.locator('.demo-toolbar__recovery')).toHaveText(state === 'offline' ? copy.offlineHelp : copy.authHelp);
      await expect(form.locator('#live-handle')).toBeEnabled();
      await expect(page.getByTestId('demo-metrics')).toContainText('"ticks":0');
      await form.locator('.diagnostic-action').click();
      await expect(page.locator('.debug-page')).toBeVisible();
      await page.locator('[data-guide="diagnostics-back"]').click();
      await expect(form.locator('#live-handle')).toBeEnabled();
    }
    await page.getByRole('combobox', { name: copy.label, exact: true }).click();
    await page.getByRole('option', { name: copy.normal, exact: true }).click();
    await form.locator('.base-button--primary').click();
    await expect(form.locator('#live-handle')).toBeDisabled();
    await expect(form.locator('#live-cookie')).toHaveCount(0);
    await page.getByTestId('demo-reset').click();
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
    await page.setViewportSize({ width:390, height:844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (locale === 'es') { await expect(page.locator('.VPSidebar')).not.toBeInViewport(); await page.evaluate(() => window.scrollTo(0,0)); await page.screenshot({ path:'/tmp/tiktools-redesign-es-installation-mobile.png',fullPage:true }); }
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
    await page.setViewportSize({ width:390, height:844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (locale === 'es') await page.screenshot({ path:'/tmp/tiktools-redesign-es-releases-mobile.png', fullPage:true });
  });
}

test('all focused product pages fit an embedded mobile viewport and preserve their real controls', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  await page.setViewportSize({ width:360, height:560 });
  for (const [panel, selector] of [['automation', '.automations-page'], ['points', '.points-page'], ['overlays', '.overlays-page']] as const) {
    await page.goto(`demo/?scenario=${panel}&embed=${panel}&locale=en`);
    await expect(page.getByTestId('demo-step')).toBeEnabled();
    await expect(page.locator(selector)).toBeVisible();
    await expect(page.locator('.app-menu-bar')).toBeHidden();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.getByTestId('demo-step').click();
    await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  }
  expect(errors).toEqual([]);
});
