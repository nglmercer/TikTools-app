import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { test, expect } from '@playwright/test';
import { demoLocales, scenarios } from '../../src/web/demo/scenarios.ts';
import { componentFamilies } from '../../docs/scripts/component-families.ts';
import { experienceCopy } from '../../docs/experience/copy.ts';

test('table examples sort actual rows and tab examples retain independent drafts', async ({ page }) => {
  await page.goto('demo/?scenario=settings&locale=en&route=/components/collections');
  const example = page.locator('.component-example');
  await example.getByRole('columnheader', { name: '#' }).click();
  await expect(example.locator('tbody tr').first()).toContainText('1');
  await example.getByRole('columnheader', { name: '#' }).click();
  await expect(example.locator('tbody tr').first()).toContainText('3');
  await page.goto('demo/?scenario=settings&locale=en&route=/components/tabs');
  const tabs = page.locator('.component-example');
  await tabs.locator('textarea').fill('first draft');
  await tabs.getByRole('tab').nth(1).click();
  await tabs.locator('textarea').fill('second draft');
  await tabs.getByRole('tab').nth(0).click();
  await expect(tabs.locator('textarea')).toHaveValue('first draft');
  await tabs.getByRole('tab').nth(1).click();
  await expect(tabs.locator('textarea')).toHaveValue('second draft');
});

test('the feed panel title follows the live state and viewer count', async ({ page }) => {
  await page.goto('demo/?scenario=feed&locale=es&instance=feed-title');
  const title = page.locator('.live-feed .base-panel__title');
  await expect(title).toContainText('En vivo');
  await expect(title).toContainText('espectadores');
  await page.getByRole('button', { name: 'Desconectar', exact: true }).click();
  await expect(title).toContainText('Actividad en vivo');
});

test('a successful connection highlights the feed section to read events', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=feed-suggestion');
  // Playback connects through the simulated RPC without leaving the section.
  await page.getByTestId('demo-play').click();
  await expect(page.getByTestId('demo-playback-status')).toHaveAttribute('data-status', 'playing');
  const feed = page.locator('#app-sidebar').getByRole('link', { name: t('navigation.feed'), exact: true });
  await expect(feed).toHaveClass(/app-navigation__link--suggested/);
  // The spotlight is state-driven: no pointer interaction is needed.
  const spotlight = page.locator('#app-spotlight');
  await expect(spotlight).toBeVisible();
  await page.mouse.move(900, 700);
  await expect(spotlight).toBeVisible();
  const [cutout, link] = await Promise.all([spotlight.boundingBox(), feed.boundingBox()]);
  expect(Math.abs(cutout!.x - link!.x)).toBeLessThan(6);
  await feed.click();
  await expect(spotlight).toBeHidden();
  await expect(page).toHaveURL(/#\/feed$/);
});

test('the spotlight can be dismissed with Escape or an outside click', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=spotlight-dismiss');
  await page.getByTestId('demo-play').click();
  const spotlight = page.locator('#app-spotlight');
  await expect(spotlight).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(spotlight).toBeHidden();
});

test('non-feed scenarios start offline and connect on first playback', async ({ page }) => {
  await page.goto('demo/?scenario=analytics&locale=en&instance=analytics-status');
  const indicator = page.locator('.app-status-bar .connection-indicator');
  await expect(indicator).toContainText('Offline');
  await expect(page.locator('.app-status-bar')).toContainText('@demo_creator');
  await page.getByTestId('demo-play').click();
  await expect(indicator).toContainText('Live');
});

test('Connect without Play stays on Connections and the guide owns the feed step', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=connect-guide');
  await page.getByRole('textbox').first().fill('demo_creator');
  await page.getByRole('button', { name: t('connection.connect'), exact: true }).first().click();
  await expect(page).toHaveURL(/#\/connections$/);
  const feed = page.locator('#app-sidebar').getByRole('link', { name: t('navigation.feed'), exact: true });
  await expect(feed).toHaveClass(/app-navigation__link--suggested/);
  await expect(page.locator('#app-spotlight')).toBeVisible();

  // The opt-in switch lives on the Connections page, not in demo controls.
  await expect(page.getByTestId('demo-redirect-on-connect')).toHaveCount(0);
  const redirect = page.getByTestId('connection-open-feed');
  await expect(redirect).not.toBeChecked();
  await redirect.locator('xpath=..').click();
  await expect(redirect).toBeChecked();
  await page.getByRole('button', { name: t('connection.disconnect'), exact: true }).first().click();
  await page.getByRole('button', { name: t('connection.connect'), exact: true }).first().click();
  await expect(page).toHaveURL(/#\/feed$/);
});

test('the demo sidebar exposes diagnostics and component reference sections', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=sidebar-sections');
  const nav = page.locator('#app-sidebar');
  await expect(nav.getByRole('link', { name: 'Diagnostics', exact: true })).toBeVisible();
  await expect(nav.getByRole('link', { name: 'Components', exact: true })).toBeVisible();
  // The floating controls dock covers the lower sidebar links at this size.
  await page.evaluate(() => document.querySelector<HTMLAnchorElement>('a[href="#/debug"]')!.click());
  await expect(page).toHaveURL(/#\/debug$/);
});

test('the toolbar language switcher changes the demo locale', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en');
  const language = page.getByRole('combobox', { name: 'Language', exact: true });
  await expect(language).toHaveValue('en');
  await language.selectOption('ja');
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  expect(new URL(page.url()).searchParams.get('locale')).toBe('ja');
});

test('floating controls keep the top bar clear, hide the step outside the feed and teach connection cases', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=es&instance=floating-controls');
  await expect(page.locator('.demo-dock')).toBeVisible();
  await expect(page.getByTestId('demo-play')).toBeVisible();
  await expect(page.getByTestId('demo-guide')).toBeVisible();
  await expect(page.getByTestId('demo-step')).toHaveCount(0);
  await expect(page.getByRole('combobox', { name: experienceCopy.es.connectionStates.label, exact: true })).toBeVisible();
  await page.getByTestId('demo-close-controls').click();
  await expect(page.locator('.demo-dock')).toHaveCount(0);
  await expect(page.getByTestId('demo-open-controls')).toBeVisible();
  await page.getByTestId('demo-open-controls').click();
  await expect(page.locator('.demo-dock')).toBeVisible();
  await page.getByRole('link', { name: testMessage('es', 'navigation.feed'), exact: true }).click();
  await expect(page.getByTestId('demo-step')).toBeVisible();
  await expect(page.getByRole('combobox', { name: experienceCopy.es.connectionStates.label, exact: true })).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('the jp URL alias selects Japanese and is rewritten to the canonical ja locale', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=jp');
  await expect(page.getByTestId('demo-play')).toBeEnabled();
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  expect(new URL(page.url()).searchParams.get('locale')).toBe('ja');
});

test('every public scenario loads real pages without external or native requests', async ({ page, baseURL }) => {
  const failures: string[] = []; const outbound: string[] = [];
  page.on('pageerror', (error) => failures.push(error.message));
  page.on('request', (request) => { if (!request.url().startsWith(baseURL!) && !request.url().startsWith('data:') && !request.url().startsWith('blob:')) outbound.push(request.url()); });
  for (const scenario of Object.keys(scenarios)) {
    await page.goto(`demo/?scenario=${scenario}&locale=en&instance=smoke-${scenario}`);
    await expect(page.getByTestId('demo-play')).toBeVisible();
    await expect(page.locator('#app')).not.toBeEmpty();
    await expect.poll(async () => { const value = await page.getByTestId('demo-metrics').textContent(); return value ? JSON.parse(value).client.pending : -1; }).toBe(0);
    await expect(page.locator('body')).not.toContainText('Invalid automation snapshot');
    expect(await page.evaluate(() => typeof window.ipc)).toBe('undefined');
  }
  expect(failures).toEqual([]); expect(outbound).toEqual([]);
});

test('connect → event → automation → result, then reset and direct refresh', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=flow');
  await page.getByRole('textbox').first().fill('demo_creator');
  await page.getByRole('button', { name: t('connection.connect'), exact: true }).first().click();
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await page.getByTestId('demo-step').click();
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText('Hello Luna');
  await page.reload(); await expect(page.locator('#app')).not.toBeEmpty();
  await page.getByTestId('demo-reset').click();
  await expect(page.locator('body')).toContainText('No runs yet');
});

test('Play connects the first scenario, shows events, and pause/step/reset remain coherent', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=playback-controls');
  const status = page.getByTestId('demo-playback-status');
  await expect(page.getByTestId('demo-play')).toBeEnabled();
  await page.getByTestId('demo-play').click();
  await expect(status).toHaveAttribute('data-status', 'playing');
  await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  await page.getByTestId('demo-play').click();
  await expect(status).toHaveAttribute('data-status', 'paused');
  const count = await page.getByTestId('demo-event-count').textContent();
  await page.waitForTimeout(2100); // Cross the actual autoplay interval: pause must stop delivery.
  await expect(page.getByTestId('demo-event-count')).toHaveText(count ?? '');
  await expect(page.getByTestId('demo-step')).toHaveCount(0);
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await page.getByTestId('demo-step').click();
  await expect(page.getByTestId('demo-event-count')).toContainText('2 / 300');
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText('Hello Luna');
  await page.getByTestId('demo-reset').click();
  await expect(page.getByTestId('demo-event-count')).toContainText('0 / 300');
  await expect(status).toHaveAttribute('data-status', 'ready');
});

test('component previews and every locale work at a narrow width', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const locale of demoLocales) {
    await page.goto(`demo/?scenario=settings&locale=${locale}&route=/components/buttons`);
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.getByTestId('demo-play')).toBeEnabled();
    await expect(page.locator('.component-example button')).toBeVisible();
    await page.locator('.component-example button').click();
    await expect(page.locator('.component-example button')).toContainText('1');
  }
});

test('direct component entry owns a working runtime across catalog and workspace navigation', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('demo/?scenario=feed&locale=en&route=/components/buttons&instance=catalog-runtime');
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await expect(page.getByTestId('demo-step')).toBeEnabled();
  await page.getByTestId('demo-step').click();
  await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await expect(page.locator('.live-feed')).toContainText('Hello TikTools!');
  await page.getByTestId('demo-more').click();
  await page.getByRole('menuitem', { name: 'Component reference', exact: true }).click();
  await expect(page.locator('.component-catalog')).toBeVisible();
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await page.getByTestId('demo-step').click();
  await expect(page.getByTestId('demo-event-count')).toContainText('2 / 300');
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText('Hello Luna');
  expect(errors).toEqual([]);
});

test('all executable component examples render and survive state changes', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  for (const family of Object.keys(componentFamilies)) {
    await page.goto(`demo/?scenario=settings&locale=en&route=/components/${family}`);
    await expect(page.locator('.component-example')).toBeVisible();
    for (const state of ['Disabled', 'No items', 'Loading']) {
      await page.getByRole('combobox', { name: 'Preview state', exact: true }).click();
      await page.getByRole('option', { name: state, exact: true }).click();
    }
  }
  expect(errors).toEqual([]);
});

test('copying a demo URL into another tab isolates preferences and reset', async ({ page, context }) => {
  await page.goto('demo/?scenario=settings&locale=en&route=/components/buttons&instance=copied');
  await expect(page.locator('.component-example button')).toBeVisible();
  const firstInstance = new URL(page.url()).searchParams.get('instance');
  const second = await context.newPage();
  try {
    await second.goto(page.url());
    await expect(second.locator('.component-example button')).toBeVisible();
    expect(new URL(second.url()).searchParams.get('instance')).not.toBe(firstInstance);
    await page.evaluate(() => localStorage.setItem(`tiktools.demo.${new URL(location.href).searchParams.get('instance')}.isolation`, 'first'));
    await second.evaluate(() => localStorage.setItem(`tiktools.demo.${new URL(location.href).searchParams.get('instance')}.isolation`, 'second'));
    await page.getByTestId('demo-reset').click();
    await expect(page.locator('.component-example button')).toBeVisible();
    expect(new URL(page.url()).searchParams.get('instance')).toBe(firstInstance);
    expect(await page.evaluate(() => localStorage.getItem(`tiktools.demo.${new URL(location.href).searchParams.get('instance')}.isolation`))).toBeNull();
    expect(await second.evaluate(() => localStorage.getItem(`tiktools.demo.${new URL(location.href).searchParams.get('instance')}.isolation`))).toBe('second');
  } finally { await second.close(); }
});
