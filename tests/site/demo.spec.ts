import { test, expect } from '@playwright/test';
import { scenarios } from '../../src/web/demo/scenarios.ts';
import { componentFamilies } from '../../docs/scripts/component-families.ts';

test('every public scenario loads real pages without external or native requests', async ({ page, baseURL }) => {
  const failures: string[] = []; const outbound: string[] = [];
  page.on('pageerror', (error) => failures.push(error.message));
  page.on('request', (request) => { if (!request.url().startsWith(baseURL!) && !request.url().startsWith('data:') && !request.url().startsWith('blob:')) outbound.push(request.url()); });
  for (const scenario of Object.keys(scenarios)) {
    await page.goto(`demo/?scenario=${scenario}&locale=en&instance=smoke-${scenario}`);
    await expect(page.getByTestId('demo-step')).toBeVisible();
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
  await page.getByRole('button', { name: 'Connect', exact: true }).first().click();
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
  const count = await page.getByTestId('demo-event-count').innerText();
  await page.waitForTimeout(2100); // Cross the actual autoplay interval: pause must stop delivery.
  await expect(page.getByTestId('demo-event-count')).toHaveText(count);
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
  for (const locale of ['en', 'es', 'pt', 'fr']) {
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
  await expect(page.getByTestId('demo-step')).toBeEnabled();
  await page.getByTestId('demo-step').click();
  await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  await page.getByRole('link', { name: 'Live feed', exact: true }).click();
  await expect(page.locator('.live-feed')).toContainText('Hello TikTools!');
  await page.getByTestId('demo-more').click();
  await page.getByRole('menuitem', { name: 'Component reference', exact: true }).click();
  await expect(page.locator('.component-catalog')).toBeVisible();
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
