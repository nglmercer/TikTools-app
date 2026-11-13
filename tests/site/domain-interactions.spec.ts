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
  await expect(page.getByTestId('demo-step')).toBeVisible();
  await expect(page.getByText('Native effects, TikTok, OBS and real accounts are simulated.', { exact: true })).toBeVisible();
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
  const dialog = page.getByRole('dialog', { name: 'Cloud overlays', exact: true });
  await expect(dialog).toBeVisible();
  return dialog;
}

function greetingRow(page: Page): Locator {
  return page.getByRole('table', { name: 'Runtime globals', exact: true }).getByRole('row', { name: /^greeting / });
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

test('plugin settings discard, save and reopen preserve the saved values', async ({ page }) => {
  await openDemo(page, 'plugins', 'plugin-settings');
  const details = await audioDetails(page);
  await details.getByRole('tab', { name: 'Settings', exact: true }).click();
  const settings = details.locator('.plugin-settings');
  const volume = settings.getByRole('spinbutton', { name: 'volume', exact: true });
  const enabled = settings.getByRole('checkbox', { name: 'enabled', exact: true });
  const save = settings.getByRole('button', { name: 'Save', exact: true });
  await expect(volume).toHaveValue('50');
  await expect(enabled).toBeChecked();
  await expect(save).toBeDisabled();

  await volume.fill('65');
  await volume.press('Enter');
  await expect(save).toBeEnabled();
  await settings.getByRole('button', { name: 'Discard changes', exact: true }).click();
  await expect(volume).toHaveValue('50');
  await expect(save).toBeDisabled();

  await volume.fill('72');
  await volume.press('Enter');
  await enabled.uncheck();
  await save.click();
  await expect(settings.getByRole('status')).toHaveText('Settings saved.');
  await expect(save).toBeDisabled();
  await details.getByRole('button', { name: 'Back', exact: true }).click();
  const reopened = await audioDetails(page);
  await reopened.getByRole('tab', { name: 'Settings', exact: true }).click();
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
  await details.getByRole('button', { name: 'Remove from installed', exact: true }).click();
  await expect(details.getByRole('button', { name: 'Install', exact: true })).toBeVisible();
  await expect(details.getByRole('switch', { name: 'Enabled', exact: true })).toHaveCount(0);
  await details.getByRole('button', { name: 'Back', exact: true }).click();
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
  await library.getByRole('searchbox', { name: 'Search overlays…', exact: true }).fill('Chat');
  const card = library.locator('.overlay-library__card').filter({ hasText: 'Chat overlay' });
  await expect(card).toHaveCount(1);
  await card.click();
  await expect(card).toHaveAttribute('aria-pressed', 'true');
  await library.getByRole('button', { name: 'Open in editor', exact: true }).click();
  await expect(library).not.toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Browser source URL', exact: true })).toHaveValue(/\/widgets\/chat\//);
  await expect(page.getByRole('button', { name: 'Replay preview', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Replay preview', exact: true }).click();
  await expect(page.locator('.overlay-preview')).toBeVisible();
  await settled(page);
});

test('Cloud overlay creation, revision and library reopen retain the edited document', async ({ page }) => {
  await openDemo(page, 'overlays', 'overlay-create');
  const library = await overlayLibrary(page);
  await library.getByRole('button', { name: 'Cloud editor', exact: true }).click();
  const editor = library.locator('.cloud-overlays');
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Fictional audience card');
  await editor.getByRole('button', { name: 'Add text', exact: true }).click();
  await editor.getByRole('textbox', { name: 'Text', exact: true }).fill('Hello fictional viewers');
  const width = editor.getByRole('textbox', { name: 'Canvas width', exact: true });
  await width.fill('100');
  await expect(editor.getByRole('button', { name: 'Create overlay', exact: true })).toBeDisabled();
  await width.fill('1920');
  await editor.getByRole('button', { name: 'Create overlay', exact: true }).click();
  await expect(editor).toContainText(/Saved as .+, revision 1\./);

  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Fictional audience card revised');
  await editor.getByRole('textbox', { name: 'Text', exact: true }).fill('Welcome fictional viewers');
  await editor.getByRole('button', { name: 'Save new revision', exact: true }).click();
  await expect(editor).toContainText(/Saved as .+, revision 2\./);
  await library.getByRole('button', { name: 'My overlays', exact: true }).click();
  const card = library.locator('.overlay-library__card').filter({ hasText: 'Fictional audience card revised' });
  await expect(card).toContainText('Revision 2');
  await card.click();
  await library.getByRole('button', { name: 'Open in editor', exact: true }).click();
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue('Fictional audience card revised');
  await expect(editor.getByRole('textbox', { name: 'Text', exact: true })).toHaveValue('Welcome fictional viewers');
  await expect(editor).toContainText(/revision 2\./);
  await settled(page);
});

test('collaboration approval and identity revocation require confirmation and refresh the matrix', async ({ page }) => {
  await openDemo(page, 'collaboration', 'collaboration-approve');
  const matrix = page.getByRole('table', { name: 'Who can access this host', exact: true });
  const reviewer = matrix.getByRole('row').filter({ hasText: 'Demo reviewer' });
  await expect(reviewer, 'The seeded pending request must remain usable at the current date').toBeVisible();
  await reviewer.getByRole('button', { name: 'Approve device', exact: true }).click();
  const approval = page.getByRole('dialog', { name: 'Approve this device?', exact: true });
  await expect(approval).toContainText('demo-request');
  await approval.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(reviewer.getByRole('button', { name: 'Approve device', exact: true })).toBeVisible();
  await reviewer.getByRole('button', { name: 'Approve device', exact: true }).click();
  await approval.getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(approval).not.toBeVisible();
  await expect(reviewer.getByRole('button', { name: 'Approve device', exact: true })).toHaveCount(0);
  await reviewer.getByRole('button', { name: 'Revoke', exact: true }).click();
  const revoke = page.getByRole('dialog', { name: 'Revoke access?', exact: true });
  await expect(revoke).toContainText('demo-reviewer');
  await revoke.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(reviewer).toBeVisible();
  await reviewer.getByRole('button', { name: 'Revoke', exact: true }).click();
  await revoke.getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(reviewer).toHaveCount(0);
  await expect(matrix.getByRole('button', { name: 'Create private invitation', exact: true })).toBeEnabled();
  await settled(page);
});

test('private invitation creation and revocation close the transient masked-link dialog', async ({ page }) => {
  await openDemo(page, 'collaboration', 'collaboration-invite');
  await page.getByRole('button', { name: 'Workflow reviewer', exact: true }).click();
  await page.getByRole('button', { name: 'Create private invitation', exact: true }).click();
  const invitation = page.getByRole('dialog', { name: 'Invitation ready', exact: true });
  await expect(invitation, 'A newly issued fictional invitation must not already be expired').toBeVisible();
  await expect(invitation.locator('#collaboration-link')).toHaveAttribute('type', 'password');
  await expect(invitation.locator('#collaboration-link')).toHaveAttribute('readonly', '');
  await invitation.getByRole('button', { name: 'Revoke invitation', exact: true }).click();
  const revoke = page.getByRole('dialog', { name: 'Revoke access?', exact: true });
  await revoke.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(invitation).toBeVisible();
  await invitation.getByRole('button', { name: 'Revoke invitation', exact: true }).click();
  await revoke.getByRole('button', { name: 'Apply', exact: true }).click();
  await expect(invitation).not.toBeVisible();
  await expect(revoke).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Create private invitation', exact: true })).toBeEnabled();
  await settled(page);
});

test('Variables fence unsaved edits, then resolve to the reviewed Cloud value', async ({ page }) => {
  await openDemo(page, 'cloud', 'variables-cloud');
  const greeting = greetingRow(page);
  await expect(greeting.getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Hello');
  await greeting.getByRole('textbox', { name: 'Value', exact: true }).fill('Unsaved fictional greeting');
  const changes = page.getByRole('button', { name: /^Sync changes(?:, \d+ pending)?$/ });
  await expect(changes).toBeDisabled();
  await expect(changes).toHaveAttribute('title', 'Save or discard your Variables edits first.');
  await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
  const dialog = await variableChanges(page);
  await expect(dialog.locator('.sync-panel__conflict')).toContainText('Choose the version to use.');
  await dialog.getByRole('button', { name: 'Use Cloud on this device', exact: true }).click();
  await expect(dialog).toContainText('Everything is synced');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(greeting.getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Welcome');
  await expect(page.getByRole('button', { name: 'Save globals', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Sync changes', exact: true })).toBeEnabled();
  await settled(page);
});

test('Variables local resolution clears only the comparison and preserves the saved value', async ({ page }) => {
  await openDemo(page, 'cloud', 'variables-local');
  const dialog = await variableChanges(page);
  await expect(dialog.getByRole('button', { name: /Force send|Force fetch|Reset everything/ })).toHaveCount(0);
  await expect(dialog.getByRole('switch', { name: 'Auto', exact: true })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Send saved value', exact: true }).click();
  await expect(dialog).toContainText('Everything is synced');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(greetingRow(page).getByRole('textbox', { name: 'Value', exact: true })).toHaveValue('Hello');
  await page.getByRole('button', { name: 'Sync changes', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Changes · Variables', exact: true })).toContainText('Everything is synced');
  await settled(page);
});

test('updater preferences work locally while native checking and installation remain explicitly unavailable', async ({ page }) => {
  await openDemo(page, 'settings', 'settings-updater');
  const updater = page.locator('.app-updater');
  await expect(updater).toContainText('Installed version: 4.2.2');
  await expect(updater.getByRole('button', { name: 'Check for updates', exact: true })).toBeDisabled();
  await expect(updater.getByRole('button', { name: /^(Download update|Install and restart)$/ })).toHaveCount(0);
  await expect(updater).toContainText('Automatic installation is unavailable. Download a release manually:');
  await expect(updater.getByRole('link')).toHaveAttribute('href', 'https://github.com/nglmercer/TikTools-app/releases');

  await updater.getByRole('combobox', { name: 'Update channel', exact: true }).click();
  await page.getByRole('option', { name: 'Beta', exact: true }).click();
  await updater.getByRole('switch', { name: 'Check automatically', exact: true }).uncheck();
  await updater.getByRole('switch', { name: 'Notify me about updates', exact: true }).uncheck();
  await page.getByRole('tab', { name: 'Account & Cloud', exact: true }).click();
  await page.getByRole('tab', { name: 'General', exact: true }).click();
  await expect(updater.getByRole('combobox', { name: 'Update channel', exact: true })).toContainText('Beta');
  await expect(updater.getByRole('switch', { name: 'Check automatically', exact: true })).not.toBeChecked();
  await expect(updater.getByRole('switch', { name: 'Notify me about updates', exact: true })).not.toBeChecked();
  await expect(updater.getByRole('button', { name: 'Check for updates', exact: true })).toBeDisabled();
  await expect(updater).not.toContainText('TikTools is up to date.');
  await settled(page);
});
