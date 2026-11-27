import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test, type Page } from '@playwright/test';

import { installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { EVENT_KIND, RPC } from '../shared/index.ts';

const t = (key: string): string => testMessage('en', key);

/**
 * Ranking and competition RPCs are answered by a small in-page store layered
 * over the shared fake host, so create → list → detail round-trips for real.
 */
const RANKINGS_HOST = `(() => {
  const base = window.ipc.postMessage;
  const store = { rankings: [], competitions: [], calls: [] };
  window.__RANKINGS_E2E__ = store;
  const metrics = [['tiktok.like', 'like.count'], ['tiktok.gift', 'gift.diamonds'], ['competition.vote', 'event.count']]
    .map(([source, metric]) => ({ source, metric, description: metric, unit: 'x' }));
  const snapshot = (id) => ({ rankingId: id, schemaVersion: 1, creatorId: 'creator', roomId: 'room', revision: 1, integrityStatus: 'unknown', paused: false,
    entries: [{ key: '1', label: 'Ana', rank: 1, score: 30 }], totalScore: 30, totalParticipants: 1, instanceId: '0123456789abcdef0123456789abcdef' });
  const handlers = {
    // Seeds are read lazily: a test may register them after this script.
    'rankings.list': () => ({ rankings: [...(window.__RANKINGS_SEED__ || []), ...store.rankings] }),
    'rankings.metrics': () => ({ metrics }),
    'rankings.create': (params) => {
      store.rankings.push(params);
      return { ok: true };
    },
    'rankings.snapshot': (params) => snapshot(params.id),
    'competitions.list': () => ({ competitions: store.competitions }),
    'competitions.create': (params) => {
      store.competitions.push(params);
      return { ok: true };
    },
    'competitions.state': () => ({ lifecycle: 'draft', generation: 0, deadlineMs: null, remainingMs: null }),
    'competitions.results': () => ({ results: [] }),
    'automation.list': () => ({ actions: [] }),
    'widgets.copyObsUrl': () => ({ ok: true }),
  };
  window.ipc.postMessage = (raw) => {
    const message = JSON.parse(raw);
    const handler = message && message.jsonrpc === '2.0' && handlers[message.method];
    if (!handler) return base(raw);
    store.calls.push({ method: message.method, params: message.params || {} });
    const response = JSON.stringify({
      type: 'rpc-response',
      response: { jsonrpc: '2.0', id: message.id, result: handler(message.params || {}) },
    });
    queueMicrotask(() => window.__webview_on_message__(response));
  };
})();`;

type Store = {
  calls: Array<{
    method: string;
    params: Record<string, unknown>;
  }>;
};

declare global {
  interface Window {
    /** The in-page ranking store installed by `RANKINGS_HOST`. */
    __RANKINGS_E2E__: Store;
    /** Rankings a test seeds before the app lists them. */
    __RANKINGS_SEED__: unknown[];
  }
}

const calls = (page: Page, method: string) =>
  page.evaluate(
    (name) =>
      window.__RANKINGS_E2E__.calls
        .filter((call) => call.method === name)
        .map((call) => call.params),
    method,
  );

test.beforeEach(async ({ page }) => {
  const state = emptyState();
  state.liveStatus = { connected: true, uniqueId: 'creator', roomId: 'room', connectionId: 'c', native: true };
  await installFakeHost(page, state);
  await page.addInitScript(RANKINGS_HOST);
  await page.goto('/#/rankings');
  await waitForFrontend(page);
});

test('creates a gift ranking prefilled from the live room and copies its OBS URL', async ({ page }) => {
  await page.getByRole('button', { name: t('rankingsPage.list.new') }).click();
  await expect(page.getByLabel(t('rankingsPage.form.creator'))).toHaveValue('creator');
  await page.getByLabel(t('rankingsPage.form.id')).fill('top-gifts');
  await page.getByRole('button', { name: '+ Gift value (diamonds)' }).click();
  await page.getByRole('radio', { name: 'Daily' }).click();
  await page.getByRole('button', { name: t('rankingsPage.form.create') }).click();

  await expect(page.getByText(t('rankingsPage.form.created'))).toBeVisible();
  const [created] = await calls(page, RPC.rankingsCreate);
  expect(created).toMatchObject({
    id: 'top-gifts',
    creatorId: 'creator',
    roomId: 'room',
    window: { type: 'daily' },
    sources: [{ source: EVENT_KIND.tiktokGift, metric: 'gift.diamonds' }],
  });
  await expect(page.getByText('Ana — 30')).toBeVisible();

  await page.getByRole('button', { name: t('rankingsPage.detail.copyUrl') }).click();
  expect(await calls(page, 'widgets.copyObsUrl')).toEqual([expect.objectContaining({ widget: 'ranking', rankingId: 'top-gifts' })]);
});

test('creates a leaderboard challenge without options', async ({ page }) => {
  await page.addInitScript(() => {
    window.__RANKINGS_SEED__ = [{
      schemaVersion: 1, id: 'top-gifts', creatorId: 'creator', roomId: 'room', groupBy: 'user', window: { type: 'session' },
      sources: [{ source: 'tiktok.gift', metric: 'gift.diamonds', weight: 1, enabled: true }], limit: 10, capacity: 100000,
    }];
  });
  await page.reload();
  await waitForFrontend(page);
  await page.getByRole('tab', { name: 'Competitions' }).click();
  await page.getByRole('button', { name: t('competitionsPanel.list.new') }).click();
  await page.getByRole('combobox', { name: 'Type' }).selectOption('leaderboard_challenge');
  await page.getByLabel('Name').fill('Top gifter');
  await page.getByLabel(t('competitionsPanel.form.id')).fill('challenge');
  await page.getByRole('combobox', { name: 'Ranking', exact: true }).selectOption('top-gifts');
  await page.getByLabel(t('competitionsPanel.form.targetScore')).fill('500');
  await expect(page.getByText('Options', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: t('competitionsPanel.form.create') }).click();

  await expect(page.getByText(t('competitionsPanel.form.created'))).toBeVisible();
  const [created] = await calls(page, RPC.competitionsCreate);
  expect(created).toMatchObject({
    id: 'challenge',
    kind: 'leaderboard_challenge',
    rankingId: 'top-gifts',
    options: [],
    targetScore: 500,
    selectionMode: 'highest_score',
  });
});
