import { expect, test, type Page } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { resolve, join, sep } from 'node:path';
import {
  demoTestId,
  demoUrl,
  docLocales,
  expectFirstChatMessage,
  frameTestId,
  qaArtifacts,
  qaScreenshot,
  qaWrite,
  readyPanelFrame,
  selector,
  trackPageErrors,
  viewport,
} from './support/index.ts';
import { readArticles, metadataErrors } from '../../docs/scripts/content.ts';
import { docCopy, localizedDocumentPath, type DocLocale } from '../../docs/locales.ts';
import { siteConfig } from '../../scripts/lib/site-config.ts';

/** Longer than the mobile navigation's exit animation, so QA captures never catch a transition. */
const NAV_SETTLE_MS = 550;

const root = resolve(import.meta.dirname, '../..');
const site = siteConfig();
const articles = readArticles();
const htmlPath = (path: string) => path.replace(/\.md$/, '.html');
const canonicalPath = (path: string) => path.replace(/index\.md$/, '').replace(/\.md$/, '.html');
async function switchSiteLocale(page: Page, current: DocLocale, target: DocLocale): Promise<void> {
  const trigger = page.getByRole('button', { name: docCopy[current].language, exact: true });
  await expect(trigger).toBeVisible();
  await expect(trigger).toHaveText(docCopy[current].label);
  await trigger.click();
  await page.getByRole('menuitemcheckbox', { name: docCopy[target].label, exact: true }).click();
}

async function localNetworkOnly(page: Page, baseURL: string) {
  const origin = new URL(baseURL).origin;
  const external: string[] = [];
  const sockets: string[] = [];
  page.on('websocket', (socket) => sockets.push(socket.url()));
  await page.route('**/*', async (route) => {
    const url = route.request().url();
    if (/^(data:|blob:)/.test(url) || new URL(url).origin === origin) await route.continue();
    else {
      external.push(url);
      await route.abort();
    }
  });
  return { external, sockets };
}

test('all six Markdown languages have reviewed equivalents, current hashes and shared heading anchors', async () => {
  const all = await articles;
  expect(metadataErrors(all)).toEqual([]);
  const english = all.filter((article) => article.relativePath.startsWith('en/'));
  expect(english.length).toBeGreaterThan(0);
  for (const locale of docLocales.filter((locale) => locale !== 'en')) {
    const translations = all.filter((article) => article.relativePath.startsWith(`${locale}/`));
    expect(translations).toHaveLength(english.length);
    for (const source of english) {
      const translated = translations.find((article) => article.relativePath === source.relativePath.replace(/^en\//, `${locale}/`));
      expect(translated, source.relativePath).toBeTruthy();
      const revision = createHash('sha256').update(source.body.trim()).digest('hex');
      for (const article of [source, translated!]) {
        expect(article.frontmatter.docId).toBe(source.frontmatter.docId);
        expect(article.frontmatter.translationKey).toBe(source.frontmatter.translationKey);
        expect(article.frontmatter.sourceRevision).toBe(revision);
        expect(article.frontmatter.translationRevision).toBe(createHash('sha256').update(article.body.trim()).digest('hex'));
        expect(article.headings.filter((heading) => heading.level === 'h1')).toHaveLength(1);
      }
      expect(translated!.body).not.toBe(source.body);
      expect(translated!.headings).toEqual(source.headings);
    }
  }
});

test('explicit article navigation ordering is finite and numeric when provided', async () => {
  const invalid = (await articles)
    .filter((article) => article.frontmatter.order !== undefined && !Number.isFinite(article.frontmatter.order))
    .map((article) => article.relativePath);
  expect(invalid, 'Invalid explicit order metadata; omitted values use the navigation default').toEqual([]);
});

test('all localized built pages expose matching lang, canonical, hreflang and sitemap entries', async ({ request }) => {
  const sitemap = await request.get('sitemap.xml');
  expect(sitemap.ok()).toBe(true);
  const xml = await sitemap.text();
  const locations = new Set([...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => match[1]!.replace(/\/index\.html$/, '/')));
  for (const article of (await articles).filter((entry) => localizedDocumentPath(entry.relativePath) !== undefined)) {
    const response = await request.get(htmlPath(article.relativePath));
    expect(response.status(), article.relativePath).toBe(200);
    const html = await response.text();
    const locale = String(article.frontmatter.locale);
    const tail = localizedDocumentPath(canonicalPath(article.relativePath))!.tail;
    expect(html, article.relativePath).toMatch(new RegExp(`<html[^>]+lang="${locale}"`));
    expect(html).toContain(`<link rel="canonical" href="${new URL(canonicalPath(article.relativePath), site.url).href}">`);
    for (const language of docLocales) {
      expect(html).toContain(`<link rel="alternate" hreflang="${language}" href="${new URL(`${language}/${tail}`, site.url).href}">`);
    }
    expect(html).toContain(`<link rel="alternate" hreflang="x-default" href="${new URL(`en/${tail}`, site.url).href}">`);
    expect(locations.has(new URL(canonicalPath(article.relativePath), site.url).href), article.relativePath).toBe(true);
    expect(html).not.toContain('tests/e2e');
  }
});

test('locale switching preserves the equivalent article and stable heading in all six languages', async ({ page }) => {
  await page.setViewportSize(viewport.desktop);
  await page.goto('en/user/preferences/backups.html#restore-and-verify');
  let current: typeof docLocales[number] = 'en';
  for (const locale of [...docLocales.filter((locale) => locale !== 'en'), 'en'] as const) {
    await switchSiteLocale(page, current, locale);
    await expect(page).toHaveURL(new RegExp(`/${locale}/user/preferences/backups\\.html#restore-and-verify$`));
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.locator('#restore-and-verify')).toBeVisible();
    current = locale;
  }
});

test('one language selector is available on the unlocalized root and keeps the reader on the section', async ({ page }) => {
  await page.goto('');
  const rootSelector = page.getByRole('button', { name: docCopy.en.language, exact: true });
  await expect(rootSelector).toBeVisible();
  await expect(rootSelector).toHaveText(docCopy.en.label);
  await expect(page.getByRole('navigation', { name: docCopy.en.language })).toHaveCount(0);

  await page.goto('es/#playground');
  await switchSiteLocale(page, 'es', 'fr');
  await expect(page).toHaveURL(/\/fr\/#playground$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr');
  await expect(page.locator('#playground')).toBeVisible();
});

for (const locale of docLocales) {
  test(`${locale}: local search has translated controls and locale-specific results`, async ({ page }) => {
    await page.goto(`${locale}/user/preferences/backups.html`);
    await page.locator('.VPNavBarSearch button').click();
    const input = page.locator('#localsearch-input');
    await expect(input).toHaveAttribute('placeholder', docCopy[locale].search);
    await input.fill(({ en: 'Back up', es: 'Respaldar', pt: 'backup', fr: 'sauvegarde', zh: '备份', ja: 'バックアップ' })[locale]);
    const results = page.locator('.VPLocalSearchBox a.result');
    await expect(results.first()).toBeVisible();
    const urls = await results.evaluateAll((links) => links.map((link) => (link instanceof HTMLAnchorElement ? link.href : '')));
    expect(urls.length).toBeGreaterThan(0);
    expect(urls.every((url) => new URL(url).pathname.startsWith(`${site.base}${locale}/`))).toBe(true);
    const backups = (await articles).find((entry) => entry.relativePath === `${locale}/user/preferences/backups.md`);
    const backupsTitle = backups?.frontmatter.title;
    if (typeof backupsTitle !== 'string') throw new Error(`the ${locale} backups article has no title`);
    await expect(results.filter({ hasText: backupsTitle }).first()).toBeVisible();
    await input.fill('zzzznoresultstiktoolsreview');
    await expect(page.locator('.VPLocalSearchBox')).toContainText(docCopy[locale].noResults);
    await page.keyboard.press('Escape');
    await expect(input).not.toBeVisible();
  });

  test(`${locale}: mobile navigation reaches user and developer documentation`, async ({ page }) => {
    await page.setViewportSize(viewport.mobile);
    await page.goto(`${locale}/user/live/feed.html`);
    const menu = page.getByRole('button', { name: docCopy[locale].navigation, exact: true });
    await menu.click();
    await expect(menu).toHaveAttribute('aria-expanded', 'true');
    const sidebar = page.locator('.VPSidebar');
    await expect(sidebar).toBeVisible();
    const backupSelector = `a[href="${site.base}${locale}/user/preferences/backups.html"]`;
    const backupLink = sidebar.locator(backupSelector);
    if (!await backupLink.isVisible()) {
      const section = sidebar.locator('.VPSidebarItem.level-0').filter({ has: page.locator(backupSelector) });
      await section.locator(':scope > .item').click();
    }
    await backupLink.click();
    await expect(page).toHaveURL(new RegExp(`/${locale}/user/preferences/backups\\.html$`));
    await expect(menu).toHaveAttribute('aria-expanded', 'false');
    await page.getByRole('button', { name: docCopy[locale].mobileMenu, exact: true }).click();
    await page.locator('.VPNavScreenMenuGroup').getByRole('button', { name: docCopy[locale].audience, exact: true }).click();
    await page.locator('.VPNavScreen').getByRole('link', { name: docCopy[locale].developer, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/${locale}/developer/$`));
    await expect(page.locator('.VPNavScreen')).not.toBeVisible();
    await expect(page.locator('main h1')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (locale === 'es') {
      if (qaArtifacts) {
        // QA captures intentionally wait beyond the navigation exit animation.
        await page.waitForTimeout(NAV_SETTLE_MS);
        await qaScreenshot(page, 'tiktools-docs-review-docs-mobile.png');
      }
      await menu.click();
      await expect(menu).toHaveAttribute('aria-expanded', 'true');
      if (qaArtifacts) {
        await page.waitForTimeout(NAV_SETTLE_MS);
        await qaScreenshot(page, 'tiktools-docs-review-docs-mobile-navopen.png');
      }
    }
  });
}

test('docs load demo iframes automatically without native IPC or external requests', async ({ page, baseURL }) => {
  const network = await localNetworkOnly(page, baseURL!);
  const requests: string[] = [];
  const errors = trackPageErrors(page);
  page.on('request', (request) => requests.push(request.url()));
  await page.setViewportSize(viewport.desktop);
  await page.goto('es/user/live/feed.html');
  await expect(page.locator('iframe')).toHaveCount(1);
  expect(requests.some((url) => new URL(url).pathname.startsWith(`${site.base}demo/`))).toBe(true);
  const frame = await readyPanelFrame(page);
  await expect(frame.locator('html')).toHaveAttribute('lang', 'es');
  expect(new URL(frame.url()).origin).toBe(new URL(baseURL!).origin);
  expect(await frame.evaluate(() => Object.hasOwn(window, 'ipc'))).toBe(false);
  expect(await frame.evaluate(() => Object.hasOwn(window, '__TIKTOOLS_E2E_STATE__'))).toBe(false);
  await expect(page.locator('iframe')).toHaveAttribute('sandbox', 'allow-scripts allow-same-origin');
  await expect(page.getByRole('button', { name: 'Cargar demo interactiva', exact: true })).toHaveCount(0);
  await qaScreenshot(page, 'tiktools-docs-review-docs-desktop.png');
  expect(network.external).toEqual([]);
  expect(network.sockets).toEqual([]);
  expect(errors).toEqual([]);
});

test('two same-origin demo frames isolate preferences and reset preserves app plus other frame storage', async ({ page, baseURL }) => {
  const network = await localNetworkOnly(page, baseURL!);
  await test.step('open the feed and seed the real app storage', async () => {
    await page.goto('en/user/live/feed.html');
    await page.evaluate(() => {
      localStorage.setItem('tiktok-live-locale', 'fr');
      localStorage.setItem('tiktok-live-username', 'real-app-sentinel');
      localStorage.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, feed: { searchQuery: 'real-app-query' } }));
      sessionStorage.setItem('real-app-session-sentinel', 'keep-session');
    });
  });
  const first = await readyPanelFrame(page);
  const firstId = new URL(first.url()).searchParams.get('instance')!;
  await test.step('open a second demo frame on the same origin', async () => {
    const secondURL = new URL(first.url());
    secondURL.searchParams.set('instance', `${firstId.slice(0, 50)}-b`);
    await page.evaluate((url) => {
      const frame = document.createElement('iframe');
      frame.id = 'isolated-second-demo';
      frame.title = 'Second isolated demo';
      frame.src = url;
      frame.style.cssText = 'display:block;width:100%;height:650px';
      document.body.append(frame);
    }, secondURL.href);
  });
  const second = (await (await page.locator('#isolated-second-demo').elementHandle())!.contentFrame())!;
  await expect(second.locator(selector.appLayout)).toBeVisible();
  const secondId = new URL(second.url()).searchParams.get('instance')!;
  const secondReloadURL = second.url();
  expect(secondId).not.toBe(firstId);
  const firstSearch = first.locator(`${selector.liveFeed} input[type="search"]`);
  const secondSearch = second.locator(`${selector.liveFeed} input[type="search"]`);
  await test.step('fill each frame search, reset only the first frame, and compare stored values', async () => {
    await expect(firstSearch).toHaveValue('');
    await expect(secondSearch).toHaveValue('');
    await firstSearch.fill('only-first-frame');
    await secondSearch.fill('only-second-frame');
    await expect.poll(() => page.evaluate((id) => localStorage.getItem(`tiktools.demo.${id}.tiktok-live-view-state`), firstId)).toContain('only-first-frame');
    await expect.poll(() => page.evaluate((id) => localStorage.getItem(`tiktools.demo.${id}.tiktok-live-view-state`), secondId)).toContain('only-second-frame');
    await page.evaluate((ids) => {
      ids.forEach((id, index) => {
        const value = index === 0 ? 'first' : 'second';
        localStorage.setItem(`tiktools.demo.${id}.review-sentinel`, value);
        sessionStorage.setItem(`tiktools.demo.${id}.review-sentinel`, value);
      });
    }, [firstId, secondId]);
    await page.locator('.demo-frame').getByTestId(frameTestId.reset).click();
    await expect(page.locator('.demo-frame').getByTestId(frameTestId.reset)).toBeEnabled();
    await expect(firstSearch).toHaveValue('');
    await expect(secondSearch).toHaveValue('only-second-frame');
    const stored = await page.evaluate(([a, b]) => ({
      appLocale: localStorage.getItem('tiktok-live-locale'), appCreator: localStorage.getItem('tiktok-live-username'),
      appView: localStorage.getItem('tiktok-live-view-state'), appSession: sessionStorage.getItem('real-app-session-sentinel'),
      firstLocal: localStorage.getItem(`tiktools.demo.${a}.review-sentinel`), firstSession: sessionStorage.getItem(`tiktools.demo.${a}.review-sentinel`),
      secondLocal: localStorage.getItem(`tiktools.demo.${b}.review-sentinel`), secondSession: sessionStorage.getItem(`tiktools.demo.${b}.review-sentinel`),
    }), [firstId, secondId]);
    expect(stored).toEqual({
      appLocale: 'fr',
      appCreator: 'real-app-sentinel',
      appView: JSON.stringify({ version: 1, feed: { searchQuery: 'real-app-query' } }),
      appSession: 'keep-session',
      firstLocal: null,
      firstSession: null,
      secondLocal: 'second',
      secondSession: 'second',
    });
  });
  await test.step('reload the second frame and check the network stayed local', async () => {
    await second.goto(secondReloadURL);
    await expect(secondSearch).toHaveValue('only-second-frame');
    expect(network.external).toEqual([]);
    expect(network.sockets).toEqual([]);
  });
});

test('standalone demo remains usable at desktop and mobile sizes', async ({ page, baseURL }, testInfo) => {
  const network = await localNetworkOnly(page, baseURL!);
  await page.goto(demoUrl({ scenario: 'feed', locale: 'es', instance: 'documentation-screenshots' }));
  await expect(page.locator(selector.liveFeed)).toBeVisible();
  await expect.poll(async () => JSON.parse((await page.getByTestId(demoTestId.metrics).textContent()) || '{}').client?.pending).toBe(0);
  // The before snapshot only feeds the review evidence.
  const before = qaArtifacts ? JSON.parse((await page.getByTestId(demoTestId.metrics).textContent())!) : undefined;
  await page.getByTestId(demoTestId.step).click();
  await expect.poll(async () => JSON.parse((await page.getByTestId(demoTestId.metrics).textContent()) || '{}').host?.ticks).toBe(1);
  if (qaArtifacts) {
    const evidence = {
      url: page.url(),
      selector: selector.liveFeed,
      matchingPanels: await page.locator(selector.liveFeed).count(),
      visible: await page.locator(selector.liveFeed).isVisible(),
      before,
      after: JSON.parse((await page.getByTestId(demoTestId.metrics).textContent())!),
      feedText: await page.locator(selector.liveFeed).innerText(),
    };
    const report = JSON.stringify(evidence, null, 2);
    await qaWrite('tiktools-docs-review-feed-evidence.json', `${report}\n`);
    await testInfo.attach('demo-feed-step', { body: report, contentType: 'application/json' });
  }
  await expectFirstChatMessage(page);
  await qaScreenshot(page, 'tiktools-docs-review-demo-desktop.png');
  await page.setViewportSize(viewport.mobile);
  await page.getByTestId(demoTestId.step).click();
  await expect(page.getByTestId(demoTestId.reset)).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await qaScreenshot(page, 'tiktools-docs-review-demo-mobile.png');
  expect(network.external).toEqual([]);
  expect(network.sockets).toEqual([]);
});

test('existing frontend without native IPC fails explicitly and never falls back to the demo', async ({ page, baseURL }) => {
  const errors = trackPageErrors(page);
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));
  const origin = new URL(baseURL!).origin;
  const entryURL = new URL('native-entry-review.html', baseURL!).href;
  // Serve the existing real frontend bytes inside this test, without changing
  // dist/site, injecting a fake host or adding any test globals to a bundle.
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url());
    if (url.href === entryURL) return route.fulfill({ path: join(root, 'dist/web/index.html'), contentType: 'text/html' });
    if (url.origin === origin && url.pathname.startsWith('/assets/')) {
      const file = resolve(root, 'dist/web', `.${url.pathname}`);
      expect(file.startsWith(resolve(root, 'dist/web/assets') + sep)).toBe(true);
      const extension = file.split('.').at(-1);
      return route.fulfill({ path: file, contentType: extension === 'js' ? 'text/javascript' : extension === 'css' ? 'text/css' : 'image/svg+xml' });
    }
    return route.abort();
  });
  await page.goto(entryURL);
  await expect.poll(() => errors.some((error) => /native IPC bridge is unavailable/.test(error))).toBe(true);
  expect(await page.evaluate(() => Object.hasOwn(window, 'ipc'))).toBe(false);
  expect(await page.evaluate(() => Object.hasOwn(window, '__TIKTOOLS_E2E_STATE__'))).toBe(false);
  await expect(page.getByTestId(demoTestId.reset)).toHaveCount(0);
  expect(requests.some((url) => new URL(url).pathname.startsWith(`${site.base}demo/`))).toBe(false);
});

test('public JavaScript bundles contain no E2E fixture host or mutable test-state global', async () => {
  async function scripts(directory: string): Promise<string[]> {
    return (await Promise.all((await readdir(directory, { withFileTypes: true })).map((entry) => entry.isDirectory()
      ? scripts(join(directory, entry.name))
      : entry.name.endsWith('.js') ? [join(directory, entry.name)] : []))).flat();
  }
  const files = await scripts(join(root, 'dist/site'));
  expect(files.length).toBeGreaterThan(0);
  for (const file of files) {
    const source = await readFile(file, 'utf8');
    expect(source, file).not.toContain('__TIKTOOLS_E2E_STATE__');
    expect(source, file).not.toMatch(/tests[/\\]e2e[/\\]fixtures/);
  }
});
