import { test, expect, type Locator, type Page } from '@playwright/test';
import {
  demoLocales, demoTestId, demoUrl, eventCountText, expectFirstChatMessage, expectSettled, instanceId, selector, t, trackOutbound, trackPageErrors, viewport,
} from './support/index.ts';
import { DEMO_STORAGE_PREFIX, demoTiming } from '../../src/web/demo/contract.ts';
import { scenarios } from '../../src/web/demo/scenarios.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { componentFamilies } from '../../docs/scripts/component-families.ts';
import { experienceCopy } from '../../docs/experience/copy.ts';


const AUTOMATION_RESULT = 'Hello Luna';
const DEMO_CREATOR = 'demo_creator';
const APP_SIDEBAR = '#app-sidebar';
const APP_SPOTLIGHT = '#app-spotlight';
const DEMO_DOCK = '.demo-dock';
const COMPONENT_EXAMPLE = '.component-example';
const EXAMPLE_BUTTON = `${COMPONENT_EXAMPLE} button`;
/** One autoplay interval plus a margin: a paused demo must stop delivering across a full interval. */
const AUTOPLAY_PAUSE_MS = demoTiming.autoplayMs + 100;

/** Event counter text as the demo renders it, e.g. "1 / 300". */

/** The Feed navigation link, page-wide or scoped to a region such as the sidebar. */
const feedLink = (scope: Page | Locator): Locator => scope.getByRole('link', { name: t('navigation.feed'), exact: true });

test('table examples sort actual rows and tab examples retain independent drafts', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'settings', route: '/components/collections' }));
  const example = page.locator(COMPONENT_EXAMPLE);
  await example.getByRole('columnheader', { name: '#' }).click();
  await expect(example.locator('tbody tr').first()).toContainText('1');
  await example.getByRole('columnheader', { name: '#' }).click();
  await expect(example.locator('tbody tr').first()).toContainText('3');
  await page.goto(demoUrl({ scenario: 'settings', route: '/components/tabs' }));
  const tabs = page.locator(COMPONENT_EXAMPLE);
  await tabs.locator('textarea').fill('first draft');
  await tabs.getByRole('tab').nth(1).click();
  await tabs.locator('textarea').fill('second draft');
  await tabs.getByRole('tab').nth(0).click();
  await expect(tabs.locator('textarea')).toHaveValue('first draft');
  await tabs.getByRole('tab').nth(1).click();
  await expect(tabs.locator('textarea')).toHaveValue('second draft');
});

test('the feed panel title follows the live state and viewer count', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'feed', locale: 'es', instance: 'feed-title' }));
  const title = page.locator(`${selector.liveFeed} .base-panel__title`);
  await expect(title).toContainText('En vivo');
  await expect(title).toContainText('espectadores');
  await page.getByRole('button', { name: 'Desconectar', exact: true }).click();
  await expect(title).toContainText('Actividad en vivo');
});

test('a successful connection leaves the navigation without a special highlight', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', instance: 'feed-suggestion' }));
  // Playback connects through the simulated RPC without leaving the section.
  await page.getByTestId(demoTestId.play).click();
  await expect(page.getByTestId(demoTestId.playbackStatus)).toHaveAttribute('data-status', 'playing');
  const feed = feedLink(page.locator(APP_SIDEBAR));
  await expect(feed).not.toHaveClass(/app-navigation__link--suggested/);
  await expect(page.locator(APP_SPOTLIGHT)).toBeHidden();
  await feed.click();
  await expect(page).toHaveURL(/#\/feed$/);
});

test('non-feed scenarios start offline and connect on first playback', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'analytics', instance: 'analytics-status' }));
  const indicator = page.locator('.app-status-bar .connection-indicator');
  await expect(indicator).toContainText('Offline');
  await expect(page.locator('.app-status-bar')).toContainText('@demo_creator');
  await page.getByTestId(demoTestId.play).click();
  await expect(indicator).toContainText('Live');
});

test('Connect without Play stays on Connections until the redirect is enabled', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', instance: 'connect-guide' }));
  await page.getByRole('textbox').first().fill(DEMO_CREATOR);
  await page.getByRole('button', { name: t('connection.connect'), exact: true }).first().click();
  await expect(page).toHaveURL(/#\/connections$/);
  const feed = feedLink(page.locator(APP_SIDEBAR));
  await expect(feed).not.toHaveClass(/app-navigation__link--suggested/);
  await expect(page.locator(APP_SPOTLIGHT)).toBeHidden();

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
  await page.goto(demoUrl({ scenario: 'connection', instance: 'sidebar-sections' }));
  const nav = page.locator(APP_SIDEBAR);
  await expect(nav.getByRole('link', { name: 'Diagnostics', exact: true })).toBeVisible();
  await expect(nav.getByRole('link', { name: 'Components', exact: true })).toBeVisible();
  // The floating controls dock covers the lower sidebar links at this size.
  await page.evaluate(() => document.querySelector<HTMLAnchorElement>('a[href="#/debug"]')!.click());
  await expect(page).toHaveURL(/#\/debug$/);
});

test('the toolbar language switcher changes the demo locale', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection' }));
  const language = page.getByRole('combobox', { name: 'Language', exact: true });
  await expect(language).toHaveValue('en');
  await language.selectOption('ja');
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  expect(new URL(page.url()).searchParams.get('locale')).toBe('ja');
});

test('floating controls keep the top bar clear, hide the step outside the feed and teach connection cases', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', locale: 'es', instance: 'floating-controls' }));
  await expect(page.locator(DEMO_DOCK)).toBeVisible();
  await expect(page.getByTestId(demoTestId.play)).toBeVisible();
  await expect(page.getByTestId(demoTestId.guide)).toBeVisible();
  await expect(page.getByTestId(demoTestId.step)).toHaveCount(0);
  await expect(page.getByRole('combobox', { name: experienceCopy.es.connectionStates.label, exact: true })).toBeVisible();
  await page.getByTestId(demoTestId.closeControls).click();
  await expect(page.locator(DEMO_DOCK)).toHaveCount(0);
  await expect(page.getByTestId(demoTestId.openControls)).toBeVisible();
  await page.getByTestId(demoTestId.openControls).click();
  await expect(page.locator(DEMO_DOCK)).toBeVisible();
  await page.getByRole('link', { name: testMessage('es', 'navigation.feed'), exact: true }).click();
  await expect(page.getByTestId(demoTestId.step)).toBeVisible();
  await expect(page.getByRole('combobox', { name: experienceCopy.es.connectionStates.label, exact: true })).toHaveCount(0);
  await page.setViewportSize(viewport.mobile);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('the jp URL alias selects Japanese and is rewritten to the canonical ja locale', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', locale: 'jp' }));
  await expect(page.getByTestId(demoTestId.play)).toBeEnabled();
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  expect(new URL(page.url()).searchParams.get('locale')).toBe('ja');
});

for (const scenario of Object.keys(scenarios)) {
  test(`every public scenario loads real pages without external or native requests: ${scenario}`, async ({ page, baseURL }) => {
    const failures = trackPageErrors(page);
    const outbound = trackOutbound(page, baseURL!);
    await page.goto(demoUrl({ scenario, instance: instanceId('smoke', scenario) }));
    await expect(page.getByTestId(demoTestId.play)).toBeVisible();
    await expect(page.locator('#app')).not.toBeEmpty();
    await expectSettled(page);
    await expect(page.locator('body')).not.toContainText('Invalid automation snapshot');
    expect(await page.evaluate(() => typeof window.ipc)).toBe('undefined');
    expect(failures).toEqual([]); expect(outbound).toEqual([]);
  });
}

test('connect → event → automation → result, then reset and direct refresh', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', instance: 'flow' }));
  await page.getByRole('textbox').first().fill(DEMO_CREATOR);
  await page.getByRole('button', { name: t('connection.connect'), exact: true }).first().click();
  await feedLink(page).click();
  await page.getByTestId(demoTestId.step).click();
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText(AUTOMATION_RESULT);
  await page.reload(); await expect(page.locator('#app')).not.toBeEmpty();
  await page.getByTestId(demoTestId.reset).click();
  await expect(page.locator('body')).toContainText('No runs yet');
});

test('Play connects the first scenario, shows events, and pause/step/reset remain coherent', async ({ page }) => {
  // Install before navigation so the page's autoplay timer runs on the fake clock.
  await page.clock.install();
  await page.goto(demoUrl({ scenario: 'connection', instance: 'playback-controls' }));
  const status = page.getByTestId(demoTestId.playbackStatus);
  await expect(page.getByTestId(demoTestId.play)).toBeEnabled();
  await page.getByTestId(demoTestId.play).click();
  await expect(status).toHaveAttribute('data-status', 'playing');
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(1));
  await page.getByTestId(demoTestId.play).click();
  await expect(status).toHaveAttribute('data-status', 'paused');
  const count = await page.getByTestId(demoTestId.eventCount).textContent();
  await page.clock.runFor(AUTOPLAY_PAUSE_MS); // Cross the autoplay interval on the fake clock: pause must stop delivery.
  await expect(page.getByTestId(demoTestId.eventCount)).toHaveText(count ?? '');
  await expect(page.getByTestId(demoTestId.step)).toHaveCount(0);
  await feedLink(page).click();
  await page.getByTestId(demoTestId.step).click();
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(2));
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText(AUTOMATION_RESULT);
  await page.getByTestId(demoTestId.reset).click();
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(0));
  await expect(status).toHaveAttribute('data-status', 'ready');
});

for (const locale of demoLocales) {
  test(`component previews and every locale work at a narrow width: ${locale}`, async ({ page }) => {
    await page.setViewportSize(viewport.mobile);
    await page.goto(demoUrl({ scenario: 'settings', locale, route: '/components/buttons', instance: instanceId('preview', locale) }));
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.getByTestId(demoTestId.play)).toBeEnabled();
    await expect(page.locator(EXAMPLE_BUTTON)).toBeVisible();
    await page.locator(EXAMPLE_BUTTON).click();
    await expect(page.locator(EXAMPLE_BUTTON)).toContainText('1');
  });
}

test('direct component entry owns a working runtime across catalog and workspace navigation', async ({ page }) => {
  const errors = trackPageErrors(page);
  await page.goto(demoUrl({ scenario: 'feed', route: '/components/buttons', instance: 'catalog-runtime' }));
  await feedLink(page).click();
  await expect(page.getByTestId(demoTestId.step)).toBeEnabled();
  await page.getByTestId(demoTestId.step).click();
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(1));
  await feedLink(page).click();
  await expectFirstChatMessage(page);
  await page.getByTestId(demoTestId.more).click();
  await page.getByRole('menuitem', { name: 'Component reference', exact: true }).click();
  await expect(page.locator('.component-catalog')).toBeVisible();
  await feedLink(page).click();
  await page.getByTestId(demoTestId.step).click();
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(eventCountText(2));
  await page.getByRole('link', { name: 'Automations', exact: true }).click();
  await expect(page.locator('body')).toContainText(AUTOMATION_RESULT);
  expect(errors).toEqual([]);
});

for (const family of Object.keys(componentFamilies)) {
  test(`all executable component examples render and survive state changes: ${family}`, async ({ page }) => {
    const errors = trackPageErrors(page);
    await page.goto(demoUrl({ scenario: 'settings', route: `/components/${family}`, instance: instanceId('components', family) }));
    await expect(page.locator(COMPONENT_EXAMPLE)).toBeVisible();
    for (const state of ['Disabled', 'No items', 'Loading']) {
      await page.getByRole('combobox', { name: 'Preview state', exact: true }).click();
      await page.getByRole('option', { name: state, exact: true }).click();
    }
    expect(errors).toEqual([]);
  });
}

test('copying a demo URL into another tab isolates preferences and reset', async ({ page, context }) => {
  await page.goto(demoUrl({ scenario: 'settings', route: '/components/buttons', instance: 'copied' }));
  await expect(page.locator(EXAMPLE_BUTTON)).toBeVisible();
  const firstInstance = new URL(page.url()).searchParams.get('instance');
  const second = await context.newPage();
  try {
    await second.goto(page.url());
    await expect(second.locator(EXAMPLE_BUTTON)).toBeVisible();
    expect(new URL(second.url()).searchParams.get('instance')).not.toBe(firstInstance);
    await page.evaluate((prefix) => localStorage.setItem(`${prefix}${new URL(location.href).searchParams.get('instance')}.isolation`, 'first'), DEMO_STORAGE_PREFIX);
    await second.evaluate((prefix) => localStorage.setItem(`${prefix}${new URL(location.href).searchParams.get('instance')}.isolation`, 'second'), DEMO_STORAGE_PREFIX);
    await page.getByTestId(demoTestId.reset).click();
    await expect(page.locator(EXAMPLE_BUTTON)).toBeVisible();
    expect(new URL(page.url()).searchParams.get('instance')).toBe(firstInstance);
    expect(await page.evaluate((prefix) => localStorage.getItem(`${prefix}${new URL(location.href).searchParams.get('instance')}.isolation`), DEMO_STORAGE_PREFIX)).toBeNull();
    expect(await second.evaluate((prefix) => localStorage.getItem(`${prefix}${new URL(location.href).searchParams.get('instance')}.isolation`), DEMO_STORAGE_PREFIX)).toBe('second');
  } finally { await second.close(); }
});
