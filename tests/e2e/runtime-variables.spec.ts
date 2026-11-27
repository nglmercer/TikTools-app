const t = (key: string): string => testMessage('en', key);
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test, type Locator } from '@playwright/test';
import { countCalls, installFakeHost, readHostState, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { ROUTE } from '../shared/index.ts';

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
  await page.goto(ROUTE.automationVariables);
  await waitForFrontend(page);
  const table = page.getByRole('table', { name: t('globals.title') });
  const rows = table.getByRole('row');
  await expect(rows).toHaveCount(5);
  await expect(table.getByRole('columnheader')).toHaveText(['Key', 'Type', t('globals.value'), '']);
  const port = rows.nth(1);
  const secret = rows.nth(2);
  await expect(secret.locator('input[aria-label="Value"]')).toHaveAttribute('type', 'password');
  await expect(rows.nth(3).getByRole('combobox', { name: t('globals.value') })).toHaveValue('true');
  await port.getByRole('combobox', { name: 'Type' }).selectOption('number');
  await port.getByRole('textbox', { name: t('globals.value') }).fill('46665');
  await page.getByRole('button', { name: t('globals.save') }).click();
  await expect.poll(async () => (await readHostState(page)).typedGlobals?.commandPort).toEqual({ type: 'number', value: 46665 });
  await expect(page.getByRole('button', { name: t('globals.save') })).toBeDisabled();
  expect(await countCalls(page, 'sync.v2.stageVariables')).toBe(0);
  expect(await countCalls(page, 'sync.v2.now')).toBe(0);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('local-browser-test-private');
  await page.screenshot({ path: testInfo.outputPath('runtime-variables.png') });
  consoleCapture.assertClean();
});

/** Open one variable's details in the Changes dialog: each change is an article headed by its name. */
const openChange = (panel: Locator, name: string) => panel
  .locator('article').filter({ has: panel.page().getByText(name, { exact: true }) })
  .getByRole('button', { name: 'Show change details' }).click();

test('Variables Changes adopts a reviewed value and withdraws a secret without replacing its local copy', async ({ page }, testInfo) => {
  const state = emptyState();
  state.announcedMethods = ['cloud.status', 'globals.typed.list', 'globals.typed.replace', 'sync.status', 'sync.v2.selected', 'sync.v2.variables.review', 'sync.v2.variables.send', 'sync.v2.variables.fetch', 'sync.v2.variables.apply', 'sync.v2.variables.withdraw'];
  state.sync = { signedIn: true, enabled: true, entries: [], resolved: [] };
  state.typedGlobals = { ordinary: { type: 'secret', value: 'local-browser-private-value' } };
  state.variableComparisons = [
    { review: 'cloud-port-review', name: 'port', local: null, cloud: { type: 'number', value: 46665 }, cloudDeleted: false, localSecret: false, blockedLocal: false, pendingStatus: null, canSend: false, canApply: true, canWithdraw: true },
    { review: 'secret-withdraw-review', name: 'ordinary', local: null, cloud: null, cloudDeleted: false, localSecret: true, blockedLocal: false, pendingStatus: 'unconfirmed', canSend: false, canApply: false, canWithdraw: true },
  ];
  const capture = await installFakeHost(page, state);
  await page.goto(ROUTE.automationVariables);
  await waitForFrontend(page);
  await page.getByRole('button', { name: /Sync changes/ }).click();
  const panel = page.getByRole('dialog', { name: 'Changes · Variables', exact: true });
  await expect(panel).toBeVisible();
  await openChange(panel, 'port');
  await panel.getByRole('button', { name: t('syncChanges.applyCloud'), exact: true }).click();
  await expect.poll(() => countCalls(page, 'sync.v2.variables.apply')).toBe(0);
  await panel.getByRole('alertdialog').getByRole('button', { name: t('syncChanges.confirm'), exact: true }).click();
  await expect.poll(async () => (await readHostState(page)).typedGlobals?.port).toEqual({ type: 'number', value: 46665 });
  await openChange(panel, 'ordinary');
  await expect(panel.getByRole('button', { name: t('syncChanges.applyCloud'), exact: true })).toHaveCount(0);
  await expect(panel).not.toContainText('local-browser-private-value');
  await expect(panel.getByRole('checkbox')).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath('variables-changes-secret.png') });
  await panel.getByRole('button', { name: t('syncChanges.withdraw'), exact: true }).click();
  expect(await countCalls(page, 'sync.v2.variables.withdraw')).toBe(0);
  await panel.getByRole('alertdialog').getByRole('button', { name: t('syncChanges.confirm'), exact: true }).click();
  await expect.poll(() => countCalls(page, 'sync.v2.variables.withdraw')).toBe(1);
  expect((await readHostState(page)).typedGlobals?.ordinary).toEqual({ type: 'secret', value: 'local-browser-private-value' });
  expect(await countCalls(page, 'sync.v2.now')).toBe(0);
  expect(await countCalls(page, 'globals.typed.replace')).toBe(0);
  capture.assertClean();
});

test('editing another variable retains the exact native integer', async ({ page }) => {
  const state = emptyState();
  state.announcedMethods = ['globals.typed.list', 'globals.typed.replace'];
  state.typedGlobals = {
    counter: { type: 'number', value: JSON.parse('9223372036854775807') as number, numberText: '9223372036854775807' },
    label: { type: 'text', value: 'before' },
  };
  const capture = await installFakeHost(page, state);
  await page.goto(ROUTE.automationVariables);
  await waitForFrontend(page);
  const rows = page.getByRole('table', { name: t('globals.title') }).getByRole('row');
  await expect(rows.nth(1).getByRole('textbox', { name: t('globals.value') })).toHaveValue('9223372036854775807');
  await rows.nth(2).getByRole('textbox', { name: t('globals.value') }).fill('after');
  await page.getByRole('button', { name: t('globals.save') }).click();
  await expect.poll(async () => (await readHostState(page)).typedGlobals?.label?.value).toBe('after');
  expect((await readHostState(page)).typedGlobals?.counter).toMatchObject({ type: 'number', numberText: '9223372036854775807' });
  expect((await readHostState(page)).globals?.counter).toBe('9223372036854775807');
  await expect(rows.nth(1).getByRole('textbox', { name: t('globals.value') })).toHaveValue('9223372036854775807');
  expect(await countCalls(page, 'globals.set')).toBe(0);
  capture.assertClean();
});
