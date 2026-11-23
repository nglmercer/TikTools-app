const t = (key: string): string => testMessage('en', key);
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { test as base, expect, type Locator, type Page } from '@playwright/test';

// Exercise the production Vue controls through the public demo transport. No
// RPC injection, hidden host mutation, native bridge or substituted responses.
const test = base.extend<{ publicDemoOnly: void }>({
  publicDemoOnly: [async ({ page, baseURL }, use) => {
    const failures: string[] = [];
    const outbound: string[] = [];
    page.on('pageerror', (error) => failures.push(error.message));
    await page.route('**/*', async (route) => {
      const url = route.request().url();
      if (url.startsWith('data:') || url.startsWith('blob:') || url.startsWith(baseURL!)) await route.continue();
      else { outbound.push(url); await route.abort('blockedbyclient'); }
    });
    await page.routeWebSocket('**/*', (socket) => {
      outbound.push(socket.url());
      socket.close();
    });
    await use();
    expect(outbound, 'Demo interactions must never contact a native service or an external origin').toEqual([]);
    expect(failures, 'Real Vue views must not throw during interaction').toEqual([]);
  }, { auto: true }],
});

async function settled(page: Page): Promise<void> {
  await expect.poll(async () => {
    const text = await page.getByTestId('demo-metrics').textContent();
    return text ? (JSON.parse(text) as { client: { pending: number } }).client.pending : -1;
  }, { message: 'Every demo RPC must settle' }).toBe(0);
}

async function openDemo(page: Page, scenario: string, instance: string): Promise<void> {
  await page.goto(`demo/?scenario=${scenario}&locale=en&instance=domain-${instance}`);
  await expect(page.getByTestId('demo-play')).toBeVisible();
  expect(await page.evaluate(() => typeof (window as Window & { ipc?: unknown }).ipc)).toBe('undefined');
  await settled(page);
}

async function audioDetails(page: Page): Promise<Locator> {
  await page.locator('.plugin-catalog__open').filter({ hasText: 'Demo Audio' }).click();
  const details = page.locator('.plugin-details');
  await expect(details.getByRole('heading', { name: 'Demo Audio', exact: true })).toBeVisible();
  return details;
}

async function overlayLibrary(page: Page): Promise<Locator> {
  await page.getByRole('button', { name: 'Cloud', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: t('overlayLibrary.title'), exact: true });
  await expect(dialog).toBeVisible();
  return dialog;
}

function greetingRow(page: Page): Locator {
  return page.getByRole('table', { name: t('globals.title'), exact: true }).getByRole('row', { name: /^greeting / });
}

async function variableChanges(page: Page): Promise<Locator> {
  await page.getByRole('button', { name: 'Sync changes, 1 pending', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Changes · Variables', exact: true });
  await expect(dialog).toBeVisible();
  // The Cloud scenario is already signed in, opted in and has a comparison.
  // A missing sync.v2.selected contract must not strand it in setup.
  await expect(dialog.getByRole('button', { name: 'View greeting', exact: true })).toBeVisible();
  await dialog.getByRole('button', { name: 'View greeting', exact: true }).click();
  return dialog;
}

for (const copy of [
  { locale: 'zh', actions: '操作', create: '新建操作…', picker: '新建操作', title: '写入日志', message: '消息' },
  { locale: 'ja', actions: 'アクション', create: '新規アクション…', picker: '新規アクション', title: 'メッセージをログに記録', message: 'メッセージ' },
]) {
  test(`${copy.locale}: native action metadata is translated in the real picker and editor`, async ({ page }) => {
    await page.goto(`demo/?scenario=automation&locale=${copy.locale}&instance=domain-native-${copy.locale}`);
    await expect(page.getByTestId('demo-play')).toBeVisible();
    await page.getByRole('tab', { name: new RegExp(`^${copy.actions} \\(`) }).click();
    await page.getByRole('button', { name: copy.create, exact: true }).click();
    const picker = page.getByRole('dialog', { name: copy.picker, exact: true });
    await picker.locator('.type-picker__card').filter({ hasText: copy.title }).click();
    await expect(picker).not.toBeVisible();
    const editor = page.locator('.action-editor');
    await expect(editor).toContainText(copy.title);
    await expect(editor.getByRole('textbox', { name: copy.message, exact: true })).toHaveValue('Hello {{ user.nickname }}');
    await settled(page);
  });
}

test('plugin settings discard, save and reopen preserve the saved values', async ({ page }) => {
  await openDemo(page, 'plugins', 'plugin-settings');
  const details = await audioDetails(page);
  await details.getByRole('tab', { name: t('plugins.details.tabs.settings'), exact: true }).click();
  const settings = details.locator('.plugin-settings');
  const volume = settings.getByRole('spinbutton', { name: 'volume', exact: true });
  const enabled = settings.getByRole('checkbox', { name: 'enabled', exact: true });
  const save = settings.getByRole('button', { name: t('plugins.settings.save'), exact: true });
  await expect(volume).toHaveValue('50');
  await expect(enabled).toBeChecked();
  await expect(save).toBeDisabled();

  await volume.fill('65');
  await volume.press('Enter');
  await expect(save).toBeEnabled();
  await settings.getByRole('button', { name: t('plugins.settings.discard'), exact: true }).click();
  await expect(volume).toHaveValue('50');
  await expect(save).toBeDisabled();

  await volume.fill('72');
  await volume.press('Enter');
  await enabled.uncheck();
  await save.click();
  await expect(settings.getByRole('status')).toHaveText(t('plugins.settings.saved'));
  await expect(save).toBeDisabled();
  await details.getByRole('button', { name: t('common.back'), exact: true }).click();
  const reopened = await audioDetails(page);
  await reopened.getByRole('tab', { name: t('plugins.details.tabs.settings'), exact: true }).click();
  await expect(reopened.getByRole('spinbutton', { name: 'volume', exact: true })).toHaveValue('72');
  await expect(reopened.getByRole('checkbox', { name: 'enabled', exact: true })).not.toBeChecked();
  await settled(page);
});

test('plugin enablement and reversible installation round-trip through catalog and details', async ({ page }) => {
  await openDemo(page, 'plugins', 'plugin-install');
  const catalogSwitch = page.getByRole('switch', { name: 'Enable Demo Audio', exact: true });
  await expect(catalogSwitch).toBeChecked();
  await catalogSwitch.uncheck();
  await expect(catalogSwitch).not.toBeChecked();
  const details = await audioDetails(page);
  await expect(details.getByRole('switch', { name: 'Enabled', exact: true })).not.toBeChecked();
  await details.getByRole('switch', { name: 'Enabled', exact: true }).check();
  await details.getByRole('button', { name: t('plugins.menu.remove'), exact: true }).click();
  await expect(details.getByRole('button', { name: 'Install', exact: true })).toBeVisible();
  await expect(details.getByRole('switch', { name: 'Enabled', exact: true })).toHaveCount(0);
  await details.getByRole('button', { name: t('common.back'), exact: true }).click();
  await page.getByRole('tab', { name: /^Installed/ }).click();
  await expect(page.locator('.plugin-catalog__open').filter({ hasText: 'Demo Audio' })).toHaveCount(0);
  await page.getByRole('tab', { name: 'Explore', exact: true }).click();
  await page.getByRole('button', { name: 'Install Demo Audio', exact: true }).click();
  await page.getByRole('tab', { name: /^Installed/ }).click();
  await expect(page.locator('.plugin-catalog__open').filter({ hasText: 'Demo Audio' })).toBeVisible();
  await expect(page.getByRole('switch', { name: 'Enable Demo Audio', exact: true })).toBeChecked();
  await settled(page);
});

test('built-in overlay library selection changes the real preview without contacting OBS', async ({ page }) => {
  await openDemo(page, 'overlays', 'overlay-library');
  const library = await overlayLibrary(page);
  await library.getByRole('searchbox', { name: t('overlayLibrary.search'), exact: true }).fill('Chat');
  const card = library.locator('.overlay-library__card').filter({ hasText: t('overlays.kinds.chat.name') });
  await expect(card).toHaveCount(1);
  await card.click();
  await expect(card).toHaveAttribute('aria-pressed', 'true');
  await library.getByRole('button', { name: t('overlayLibrary.open'), exact: true }).click();
  await expect(library).not.toBeVisible();
  await expect(page.getByRole('textbox', { name: t('overlays.source.title'), exact: true })).toHaveValue(/\/widgets\/chat\//);
  await expect(page.getByRole('button', { name: t('overlays.preview.replay'), exact: true })).toBeEnabled();
  await page.getByRole('button', { name: t('overlays.preview.replay'), exact: true }).click();
  await expect(page.locator('.overlay-preview')).toBeVisible();
  await settled(page);
});

test('Cloud overlay creation, revision and library reopen retain the edited document', async ({ page }) => {
  await openDemo(page, 'overlays', 'overlay-create');
  const library = await overlayLibrary(page);
  await library.getByRole('button', { name: t('overlayLibrary.edit'), exact: true }).click();
  const editor = library.locator('.cloud-overlays');
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Fictional audience card');
  await editor.getByRole('button', { name: t('cloudOverlays.addText'), exact: true }).click();
  await editor.getByRole('textbox', { name: 'Text', exact: true }).fill('Hello fictional viewers');
  const width = editor.getByRole('textbox', { name: t('cloudOverlays.canvasWidth'), exact: true });
  await width.fill('100');
  await expect(editor.getByRole('button', { name: t('cloudOverlays.create'), exact: true })).toBeDisabled();
  await width.fill('1920');
  await editor.getByRole('button', { name: t('cloudOverlays.create'), exact: true }).click();
  await expect(editor).toContainText(/Saved as .+, revision 1\./);

  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Fictional audience card revised');
  await editor.getByRole('textbox', { name: 'Text', exact: true }).fill('Welcome fictional viewers');
  await editor.getByRole('button', { name: t('cloudOverlays.saveRevision'), exact: true }).click();
  await expect(editor).toContainText(/Saved as .+, revision 2\./);
  await library.getByRole('button', { name: t('overlayLibrary.mine'), exact: true }).click();
  const card = library.locator('.overlay-library__card').filter({ hasText: 'Fictional audience card revised' });
  await expect(card).toContainText('Revision 2');
  await card.click();
  await library.getByRole('button', { name: t('overlayLibrary.open'), exact: true }).click();
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue('Fictional audience card revised');
  await expect(editor.getByRole('textbox', { name: 'Text', exact: true })).toHaveValue('Welcome fictional viewers');
  await expect(editor).toContainText(/revision 2\./);
  await settled(page);
});

test('Variables fence unsaved edits, then resolve to the reviewed Cloud value', async ({ page }) => {
  await openDemo(page, 'cloud', 'variables-cloud');
  const greeting = greetingRow(page);
  await expect(greeting.getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Hello');
  await greeting.getByRole('textbox', { name: 'Value', exact: true }).fill('Unsaved fictional greeting');
  const changes = page.getByRole('button', { name: /^Sync changes(?:, \d+ pending)?$/ });
  await expect(changes).toBeDisabled();
  await expect(changes).toHaveAttribute('title', t('syncChanges.variablesSaveFirst'));
  await page.getByRole('button', { name: t('plugins.settings.discard'), exact: true }).click();
  const dialog = await variableChanges(page);
  await expect(dialog.locator('.sync-panel__conflict')).toContainText(t('syncChanges.conflictBoth'));
  await dialog.getByRole('button', { name: t('syncChanges.applyCloud'), exact: true }).click();
  await expect(dialog).toContainText(t('syncChanges.empty'));
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(greeting.getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Welcome');
  await expect(page.getByRole('button', { name: t('globals.save'), exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: t('syncChanges.button'), exact: true })).toBeEnabled();
  await settled(page);
});

test('Variables local resolution clears only the comparison and preserves the saved value', async ({ page }) => {
  await openDemo(page, 'cloud', 'variables-local');
  const dialog = await variableChanges(page);
  await expect(dialog.getByRole('button', { name: /Force send|Force fetch|Reset everything/ })).toHaveCount(0);
  await expect(dialog.getByRole('switch', { name: 'Auto', exact: true })).toHaveCount(0);
  await dialog.getByRole('button', { name: t('syncChanges.sendSavedValue'), exact: true }).click();
  await expect(dialog).toContainText(t('syncChanges.empty'));
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(greetingRow(page).getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Hello');
  await page.getByRole('button', { name: t('syncChanges.button'), exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Changes · Variables', exact: true })).toContainText(t('syncChanges.empty'));
  await settled(page);
});

test('updater preferences work locally while native checking and installation remain explicitly unavailable', async ({ page }) => {
  await openDemo(page, 'settings', 'settings-updater');
  const updater = page.locator('.app-updater');
  await expect(updater).toContainText('Installed version: 4.2.2');
  await expect(updater.getByRole('button', { name: t('appUpdater.check'), exact: true })).toBeDisabled();
  await expect(updater.getByRole('button', { name: /^(Download update|Install and restart)$/ })).toHaveCount(0);
  await expect(updater).toContainText(t('appUpdater.manual'));
  await expect(updater.getByRole('link')).toHaveAttribute('href', 'https://github.com/nglmercer/TikTools-app/releases');

  await updater.getByRole('combobox', { name: t('appUpdater.channel'), exact: true }).click();
  await page.getByRole('option', { name: 'Beta', exact: true }).click();
  await updater.getByRole('switch', { name: t('appUpdater.automaticChecks'), exact: true }).uncheck();
  await updater.getByRole('switch', { name: t('appUpdater.notifications'), exact: true }).uncheck();
  await page.getByRole('tab', { name: t('settingsLayout.labels.account'), exact: true }).click();
  await page.getByRole('tab', { name: 'General', exact: true }).click();
  await expect(updater.getByRole('combobox', { name: t('appUpdater.channel'), exact: true })).toContainText('Beta');
  await expect(updater.getByRole('switch', { name: t('appUpdater.automaticChecks'), exact: true })).not.toBeChecked();
  await expect(updater.getByRole('switch', { name: t('appUpdater.notifications'), exact: true })).not.toBeChecked();
  await expect(updater.getByRole('button', { name: t('appUpdater.check'), exact: true })).toBeDisabled();
  await expect(updater).not.toContainText(t('appUpdater.upToDate'));
  await settled(page);
});
