import { expect, test } from '@playwright/test';

import { readHostState, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState, sonicboomConnectedState } from './fixtures/states.ts';
import { SONICBOOM_ID } from './fixtures/plugins.ts';

/**
 * The Changes panel opens from a button in each section's toolbar and only shows that section's
 * elements. Automations compare against the snapshot this device last saw; Plugins read the v2 workspace.
 */

const event = (id: string, name: string) => ({ schemaVersion: 1, id, name, enabled: true, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'user', runMode: 'all', actionIds: [] });

test('the Events tab lists what changed since the last snapshot and asks before resetting', async ({ page }) => {
  const state = emptyState();
  state.automationSnapshot = { ...state.automationSnapshot, events: [event('greet', 'Greeting renamed'), event('new', 'Brand new')] };
  const snapshot = { greet: event('greet', 'Greeting'), gone: event('gone', 'Removed one') };
  const capture = await installFakeHost(page, state);
  await page.addInitScript((value) => localStorage.setItem('tiktools.sync.snapshot.automation-events', JSON.stringify(value)), snapshot);
  await page.goto('/#/automations');
  await waitForFrontend(page);

  const button = page.getByRole('button', { name: 'Sync changes, 3 pending' });
  await expect(button).toBeVisible();
  await button.click();
  const panel = page.getByRole('dialog', { name: 'Changes · Events' });
  await expect(panel.getByRole('listitem')).toHaveCount(3);
  // Automations have no Cloud transport yet, so only reset is offered.
  await expect(panel.getByRole('button', { name: 'Fetch changes from Cloud' })).toHaveCount(0);
  await expect(panel.getByRole('button', { name: 'Sync', exact: true })).toHaveCount(0);
  await panel.getByRole('button', { name: 'View Greeting renamed' }).click();
  await expect(panel.locator('.base-json-diff__line--added').first()).toContainText('Greeting renamed');
  await panel.getByRole('button', { name: 'Reset to the synced version' }).click();
  await expect(panel.getByRole('alertdialog')).toContainText('Greeting renamed');
  await panel.getByRole('button', { name: 'Cancel' }).click();
  await expect(panel.getByRole('alertdialog')).toHaveCount(0);
  expect((await readHostState(page)).calls.some((call) => call.method === 'automation.update')).toBe(false);
  capture.assertClean();
});

test('Plugins send the whole pass and resolve a conflict through the v2 contract', async ({ page }) => {
  const state = sonicboomConnectedState();
  state.announcedMethods = ['cloud.status', 'sync.status', 'sync.v2.selected', 'sync.v2.changes.list', 'sync.v2.now', 'sync.v2.conflict.review', 'sync.v2.conflict.resolve'];
  const value = (enabled: boolean, revision: number) => ({ state: 'present', revision, version: '1.0.0', enabled, value: { schemaVersion: 1, pluginId: SONICBOOM_ID, version: '1.0.0', enabled } });
  state.sync = {
    signedIn: true, enabled: true, lastSyncedAt: null, resolved: [],
    entries: [{ collection: 'plugin-selections', key: SONICBOOM_ID, remote: value(false, 4), pending: { status: 'conflict', base: value(true, 3), local: value(true, 3) }, hasNewer: false }],
  };
  const capture = await installFakeHost(page, state);
  await page.goto('/#/plugins');
  await waitForFrontend(page);

  await page.getByRole('button', { name: 'Sync changes, 1 pending' }).click();
  const panel = page.getByRole('dialog', { name: /Changes · Plugins/ });
  await expect(panel.locator('.sync-panel__group')).toContainText('Conflicts');
  await expect(panel.getByRole('button', { name: 'Sync', exact: true })).toBeVisible();
  // No selective send, no force and no history on this section.
  await expect(panel.getByRole('button', { name: 'Send the selected elements' })).toHaveCount(0);
  await expect(panel.getByRole('button', { name: 'More actions' })).toHaveCount(0);
  await panel.getByRole('button', { name: 'All Cloud' }).click();
  await expect.poll(async () => (await readHostState(page)).sync?.resolved).toEqual(['takeCloud']);
  await panel.getByRole('button', { name: 'Sync', exact: true }).click();
  await expect.poll(async () => (await readHostState(page)).calls.some((call) => call.method === 'sync.v2.now')).toBe(true);
  capture.assertClean();
});

test('the Plugins button explains why it is disabled when sync is off', async ({ page }) => {
  const state = sonicboomConnectedState();
  state.announcedMethods = ['cloud.status', 'sync.status', 'sync.v2.changes.list'];
  state.sync = { signedIn: true, enabled: false, entries: [], resolved: [] };
  const capture = await installFakeHost(page, state);
  await page.goto('/#/plugins');
  await waitForFrontend(page);
  const button = page.getByRole('button', { name: 'Sync changes' });
  await expect(button).toBeDisabled();
  await expect(button).toHaveAttribute('aria-description', 'Turn on Cloud sync in Settings, Account & Cloud.');
  capture.assertClean();
});
