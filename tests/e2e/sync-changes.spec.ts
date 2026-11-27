import { expect, test } from '@playwright/test';

import { readHostState, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState, sonicboomConnectedState } from './fixtures/states.ts';
import { SONICBOOM_ID } from './fixtures/plugins.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { EVENT_KIND, ROUTE, RPC } from '../shared/index.ts';
import { SEL } from './support/selectors.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);

/** Expands the review card of one element so its inline diff and actions render. */
const expandCard = async (panel: import('@playwright/test').Locator, name: string) => {
  await panel.locator('.change-review__card', { hasText: name }).first().getByRole('button', { name: t('changeReview.expand') }).click();
};

/**
 * The Changes panel opens from a button in each section's toolbar and only shows that section's
 * elements. Automations compare against this device's reference copy (never Cloud, nothing is sent); Plugins read
 * the v2 workspace, and plugin settings are listed by key name only.
 */

const event = (id: string, name: string) => ({
  schemaVersion: 1,
  id,
  name,
  enabled: true,
  trigger: EVENT_KIND.tiktokChat,
  filters: [],
  cooldownMs: 0,
  cooldownScope: 'user',
  runMode: 'all',
  actionIds: [],
});

test('the Events tab lists what changed since the last snapshot and asks before resetting', async ({ page }) => {
  const state = emptyState();
  state.automationSnapshot = { ...state.automationSnapshot, events: [event('greet', 'Greeting renamed'), event('new', 'Brand new')] };
  const snapshot = { greet: event('greet', 'Greeting'), gone: event('gone', 'Removed one') };
  const capture = await installFakeHost(page, state);
  await page.addInitScript((value) => localStorage.setItem('tiktools.sync.snapshot.automation-events', JSON.stringify(value)), snapshot);
  await page.goto(ROUTE.automations);
  await waitForFrontend(page);

  const button = page.getByRole('button', { name: t('syncChanges.buttonCount', { count: 3 }) });
  await expect(button).toBeVisible();
  await button.click();
  const panel = page.getByRole('dialog', { name: `${t('syncChanges.title')} · Events` });
  await expect(panel.locator(SEL.changeReviewCard)).toHaveCount(3);
  // Automations have no Cloud transport yet, so only reset and mark as reviewed are offered.
  await expect(panel.locator('.sync-panel__status')).toContainText(t('syncChanges.automationsNotice'));
  await expect(panel.getByRole('button', { name: t('syncChanges.fetch') })).toHaveCount(0);
  await expect(panel.getByRole('button', { name: t('syncChanges.sync'), exact: true })).toHaveCount(0);
  await expandCard(panel, 'Greeting renamed');
  await expect(panel.locator('.change-review__fields').first()).toContainText('Greeting renamed');
  await panel.getByRole('button', { name: t('syncChanges.reset') }).click();
  await expect(panel.getByRole('alertdialog')).toContainText('Greeting renamed');
  await panel.getByRole('button', { name: t('common.cancel') }).click();
  await expect(panel.getByRole('alertdialog')).toHaveCount(0);
  expect((await readHostState(page)).calls.some((call) => call.method === RPC.automationUpdate)).toBe(false);

  // Marking the section as reviewed asks first, then makes the current records the reference copy.
  await panel.getByRole('button', { name: t('syncChanges.markReviewed') }).click();
  await expect(panel.getByRole('alertdialog')).toContainText(t('syncChanges.confirmMarkReviewed'));
  await panel.getByRole('button', { name: t('syncChanges.confirm') }).click();
  await expect(panel.locator(SEL.changeReviewCard)).toHaveCount(0);
  await expect(panel.getByRole('button', { name: t('syncChanges.markReviewed') })).toBeDisabled();
  const reference = await page.evaluate(() => JSON.parse(localStorage.getItem('tiktools.sync.snapshot.automation-events') ?? '{}'));
  expect(Object.keys(reference).sort()).toEqual(['greet', 'new']);
  expect((await readHostState(page)).calls.some((call) => call.method === RPC.automationUpdate)).toBe(false);
  capture.assertClean();
});

test('Plugins send the whole pass and resolve a conflict through the v2 contract', async ({ page }) => {
  const state = sonicboomConnectedState();
  await test.step('seed a connected sync state with a conflict and a queued settings change', async () => {
    state.announcedMethods = [
      RPC.cloudStatus,
      RPC.syncStatus,
      'sync.v2.section',
      'sync.v2.selected',
      'sync.v2.changes.list',
      'sync.v2.now',
      'sync.v2.conflict.review',
      'sync.v2.conflict.resolve',
    ];
    const value = (enabled: boolean, revision: number) => ({
      state: 'present',
      revision,
      version: '1.0.0',
      enabled,
      value: { schemaVersion: 1, pluginId: SONICBOOM_ID, version: '1.0.0', enabled },
    });
    const hidden = (revision: number | null) => ({ state: 'present', revision, version: null, enabled: null, value: null });
    state.sync = {
      signedIn: true, enabled: true, lastSyncedAt: null, resolved: [],
      entries: [
        {
          collection: 'plugin-selections',
          key: SONICBOOM_ID,
          remote: value(false, 4),
          pending: { status: 'conflict', base: value(true, 3), local: value(true, 3), changedKeys: null },
          hasNewer: false,
        },
        // The host never sends settings values: only the names of the keys that changed.
        {
          collection: 'plugin-settings',
          key: 'other.plugin',
          remote: hidden(2),
          pending: { status: 'queued', base: hidden(2), local: hidden(null), changedKeys: ['apiKey', 'volume'] },
          hasNewer: false,
        },
      ],
    };
  });
  const capture = await installFakeHost(page, state);
  await test.step('open the Plugins sync changes panel', async () => {
    await page.goto(ROUTE.plugins);
    await waitForFrontend(page);

    await page.getByRole('button', { name: t('syncChanges.buttonCount', { count: 2 }) }).click();
  });
  const panel = page.getByRole('dialog', { name: new RegExp(`${t('syncChanges.title')} · Plugins`) });
  await test.step('check the local settings card and the conflicting plugin card', async () => {
    await expect(panel.locator('.sync-review__side--local')).toHaveCount(1);
    await expandCard(panel, 'other.plugin');
    await expect(panel.locator('.sync-panel__keys')).toContainText('Changed settings: apiKey, volume');
    await expect(panel.locator(SEL.baseJsonDiff)).toHaveCount(0);
    await expandCard(panel, 'SonicBoom Server');
    await expect(panel.locator('.sync-review__side--conflict')).toHaveCount(1);
    await expect(panel.getByRole('button', { name: t('syncChanges.sync'), exact: true })).toBeVisible();
  });
  await test.step('resolve the conflict by taking the cloud copy', async () => {
    // No selective send, no force and no history on this section.
    await expect(panel.getByRole('button', { name: t('syncChanges.push') })).toHaveCount(0);
    await expect(panel.getByRole('button', { name: t('syncChanges.more') })).toHaveCount(0);
    await panel.getByRole('button', { name: t('syncChanges.allCloud') }).click();
    await expect.poll(async () => (await readHostState(page)).sync?.resolved).toEqual(['takeCloud']);
  });
  await test.step('sync the section and confirm the send reached the host', async () => {
    await panel.getByRole('button', { name: t('syncChanges.sync'), exact: true }).click();
    await expect.poll(async () => (await readHostState(page)).calls.some((call) => call.method === RPC.syncV2Section)).toBe(true);
    capture.assertClean();
  });
});

test('the Plugins button explains why it is disabled when sync is off', async ({ page }) => {
  const state = sonicboomConnectedState();
  state.announcedMethods = [RPC.cloudStatus, RPC.syncStatus, 'sync.v2.section', 'sync.v2.changes.list'];
  state.sync = { signedIn: true, enabled: false, entries: [], resolved: [] };
  const capture = await installFakeHost(page, state);
  await page.goto(ROUTE.plugins);
  await waitForFrontend(page);
  const button = page.getByRole('button', { name: t('syncChanges.button') });
  await expect(button).toBeEnabled();
  await button.click();
  const panel = page.getByRole('dialog', { name: new RegExp(t('syncChanges.title')) });
  await expect(panel).toContainText(/Prepare Cloud sync/);
  capture.assertClean();
});
