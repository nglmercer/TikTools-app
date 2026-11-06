import { expect, test, type Page } from '@playwright/test';

import {
  makeTestChatEnvelope,
  makeTestFollowEnvelope,
  makeTestGiftCombo,
  makeTestShareEnvelope,
  makeTestSubscribeEnvelope,
} from '../shared/test-events.ts';
import { gatewayAvailable, readStagedSettings, startGateway, type RunningGateway } from './gateway-harness.ts';

test.skip(!gatewayAvailable, 'needs the plugins-example-tiktools checkout (set TIKTOOLS_PLUGINS_REPO)');

type WidgetTestHook = {
  connectionStatus: () => string;
};

let gateway: RunningGateway;

test.beforeAll(async () => {
  gateway = await startGateway();
});

test.afterAll(async () => {
  await gateway.stop();
});

async function waitForHook(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as unknown as Record<string, unknown>)['__tiktoolsWidgetTest'] !== undefined,
      ),
    )
    .toBe(true);
}

async function waitForStatus(page: Page, status: string): Promise<void> {
  await waitForHook(page);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as {
              __tiktoolsWidgetTest: WidgetTestHook;
            }
          ).__tiktoolsWidgetTest.connectionStatus(),
      ),
    )
    .toBe(status);
}

test('gateway serves widget bundles from the staged layout', async ({ request }) => {
  for (const kind of ['follow', 'gift', 'chat', 'share', 'subscribe']) {
    const response = await request.get(`${gateway.baseUrl}/widgets/${kind}/`);
    expect(response.ok()).toBe(true);
    expect(response.headers()['content-type']).toContain('text/html');
  }
});

test('gateway reads credentials from the per-plugin settings file', async () => {
  const stored = (await readStagedSettings(gateway.dataDir)) as Record<string, unknown>;
  // The harness wrote these tokens; the running gateway must have adopted
  // them (not generated its own from a different file) for auth below.
  expect(stored['token']).toBe(gateway.token);
  expect(stored['widgetToken']).toBe(gateway.widgetToken);
});

test('follow alert arrives over the real widget transport', async ({ page }) => {
  // Served by the gateway itself: connecting back proves the real
  // loopback Origin passes the origin allowlist.
  await page.goto(`${gateway.baseUrl}/widgets/follow/?duration=1200#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  expect(page.url()).not.toContain('?token=');
  gateway.sendEvent(makeTestFollowEnvelope());
  const card = page.locator('.follow-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Viewer Name')).toBeVisible();
  await expect(card).toBeHidden({ timeout: 8000 });
});

test('share alert arrives over the real widget transport', async ({ page }) => {
  await page.goto(`${gateway.baseUrl}/widgets/share/?duration=1500#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  gateway.sendEvent(makeTestShareEnvelope({ user: { nickname: 'live-sharer' } }));
  const card = page.locator('.share-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('live-sharer')).toBeVisible();
  await expect(card.getByText('shared the LIVE!')).toBeVisible();
});

test('subscribe alert arrives over the real widget transport', async ({ page }) => {
  await page.goto(`${gateway.baseUrl}/widgets/subscribe/?duration=1500#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  gateway.sendEvent(makeTestSubscribeEnvelope({ user: { nickname: 'live-subscriber' } }));
  const card = page.locator('.subscribe-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('live-subscriber')).toBeVisible();
  await expect(card.getByText('just subscribed!')).toBeVisible();
});

test('chat messages arrive over the real widget transport', async ({ page }) => {
  await page.goto(`${gateway.baseUrl}/widgets/chat/#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  gateway.sendEvent(makeTestChatEnvelope({ comment: 'transport hello' }));
  await expect(page.locator('.chat-list').getByText('transport hello')).toBeVisible();
  await expect(page.locator('.chat-message')).toHaveCount(1);
});

test('gift combo aggregates over the real widget transport', async ({ page }) => {
  await page.goto(`${gateway.baseUrl}/widgets/gift/?duration=1200#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  for (const envelope of makeTestGiftCombo(3, { giftName: 'Rose', diamondCount: 1 })) {
    gateway.sendEvent(envelope);
  }
  const card = page.locator('.gift-card');
  await expect(card).toBeVisible();
  await expect(card.getByText('Rose')).toBeVisible();
  await expect(card.getByText('×3')).toBeVisible();
  await expect(card).toBeHidden({ timeout: 8000 });
});

test('wrong widget credential never renders', async ({ page }) => {
  await page.goto(`${gateway.baseUrl}/widgets/follow/#token=ttw_wrong_credential`);
  await waitForStatus(page, 'auth-failed');
  gateway.sendEvent(makeTestFollowEnvelope());
  await page.waitForTimeout(1000);
  await expect(page.locator('.follow-card')).toHaveCount(0);
});


test('opaque chat and standalone emotes retain metadata through the real gateway', async ({ page }) => {
  await page.route('https://cdn.example.test/**', (route) => route.fulfill({ status: 200, contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aDQAAAABJRU5ErkJggg==', 'base64') }));
  await page.goto(`${gateway.baseUrl}/widgets/chat/#token=${gateway.widgetToken}`);
  await waitForStatus(page, 'connected');
  const envelope = makeTestChatEnvelope({ comment: 'Hola 👩🏽‍💻 世界', msgId: '9007199254740993' });
  const payload = envelope.data as { event: { data: Record<string, unknown> } };
  payload.event.data.emotes = [{ id: '987654', imageUrl: 'https://cdn.example.test/emote.png', packageId: 'creator' }];
  gateway.sendEvent(envelope);
  await expect(page.locator('.chat-message')).toContainText('Hola 👩🏽‍💻 世界');
  await expect(page.locator('.chat-message img.chat-emote')).toHaveCount(1);
  const standalone = structuredClone(envelope);
  const standaloneEvent = (standalone.data as { event: { id: string; type: string; data: Record<string, unknown> } }).event;
  standaloneEvent.id = 'standalone';
  standaloneEvent.type = 'tiktok.emote_chat';
  // A separate canonical type may legitimately use the same upstream id.
  delete standaloneEvent.data.comment;
  gateway.sendEvent(standalone);
  await expect(page.locator('.chat-message')).toHaveCount(2);
  await expect(page.locator('.chat-message img.chat-emote')).toHaveCount(2);
});
