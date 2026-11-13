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

test('component previews and every locale work at a narrow width', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const locale of ['en', 'es', 'pt', 'fr']) {
    await page.goto(`demo/?scenario=settings&locale=${locale}&route=/components/buttons`);
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.locator('.component-example button')).toBeVisible();
    await page.locator('.component-example button').click();
    await expect(page.locator('.component-example button')).toContainText('1');
  }
});

test('all executable component examples render and survive state changes', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  for (const family of Object.keys(componentFamilies)) {
    await page.goto(`demo/?scenario=settings&locale=en&route=/components/${family}`);
    await expect(page.locator('.component-example')).toBeVisible();
    for (const state of ['Disabled', 'No items', 'Loading']) {
      await page.getByRole('combobox', { name: 'Enabled', exact: true }).click();
      await page.getByRole('option', { name: state, exact: true }).click();
    }
  }
  expect(errors).toEqual([]);
});
