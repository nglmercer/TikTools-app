import { expect, test } from '@playwright/test';
import { countCalls, installFakeHost, readHostState, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('Variables preserves explicit types and keeps local secrets masked', async ({ page }, testInfo) => {
  const state = emptyState();
  state.announcedMethods = ['globals.typed.list', 'globals.typed.replace'];
  state.typedGlobals = {
    commandPort: { type: 'text', value: '8080' },
    credentials: { type: 'secret', value: 'local-browser-test-private' },
    enabled: { type: 'boolean', value: true },
    names: { type: 'list', value: ['uno', 'dos'] },
  };
  const consoleCapture = await installFakeHost(page, state);
  await page.goto('/#/automations?view=variables');
  await waitForFrontend(page);
  const table = page.getByRole('table', { name: 'Runtime globals' });
  const rows = table.getByRole('row');
  await expect(rows).toHaveCount(5);
  await expect(table.getByRole('columnheader')).toHaveText(['Key', 'Type', 'Value', '']);
  const port = rows.nth(1);
  const secret = rows.nth(2);
  await expect(secret.locator('input[aria-label="Value"]')).toHaveAttribute('type', 'password');
  await expect(rows.nth(3).getByRole('combobox', { name: 'Value' })).toHaveValue('true');
  await port.getByRole('combobox', { name: 'Type' }).selectOption('number');
  await port.getByRole('textbox', { name: 'Value' }).fill('46665');
  await page.getByRole('button', { name: 'Save globals' }).click();
  await expect.poll(async () => (await readHostState(page)).typedGlobals?.commandPort).toEqual({ type: 'number', value: 46665 });
  await expect(page.getByRole('button', { name: 'Save globals' })).toBeDisabled();
  expect(await countCalls(page, 'sync.v2.stageVariables')).toBe(0);
  expect(await countCalls(page, 'sync.v2.now')).toBe(0);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('local-browser-test-private');
  await page.screenshot({ path: testInfo.outputPath('runtime-variables.png') });
  consoleCapture.assertClean();
});
