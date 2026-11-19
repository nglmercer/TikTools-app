import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
const te = (key: string): string => testMessage('es', key);
import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost, readHostState, waitForFrontend, writeHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { collaborationStatus } from '../../src/web/shared/lib/testing/collaboration.ts';
import type { E2EHostState } from './fixtures/tiktools-host.ts';

function ownerState(): E2EHostState {
  const state = emptyState();
  state.collaboration = { status: collaborationStatus(), invitations: [], requests: [], principals: [], sessions: [], audit: [],
    issued: { inviteId: '11111111-1111-4111-8111-111111111111', secret: 'b'.repeat(64), expiresAt: Date.now() + 600000 } };
  state.liveStatus = { connected: true, uniqueId: 'owner', roomId: 'test-room', connectionId: 'test-live', native: true };
  return state;
}

test('owner reviews access, shares local QR, changes grants and revokes one identity', async ({ page }, testInfo) => {
  const fixture = ownerState(); const privateSecret = fixture.collaboration!.issued.secret;
  const consoleCapture = await installFakeHost(page, fixture, { locale: 'en' });
  const network: string[] = []; page.on('request', (request) => network.push(request.url()));
  await page.goto('/#/settings?section=collaboration'); await waitForFrontend(page);
  const matrix = page.getByRole('table', { name: t('collaboration.accessTitle') });
  const inspector = page.getByRole('region', { name: t('collaboration.editing') });
  await expect(page.getByText(t('collaboration.running'), { exact: true })).toBeVisible();

  // Build a new invitation from the workflow preset, narrowed to exact IDs.
  await page.getByRole('group', { name: t('collaboration.presetsLabel'), exact: true }).getByRole('button', { name: t('collaboration.presetWorkflow'), exact: true }).click();
  await matrix.getByRole('button', { name: /New invitation.*Workflows/ }).click();
  await inspector.getByRole('radio', { name: t('collaboration.levels.view'), exact: true }).click();
  await inspector.getByRole('radio', { name: t('collaboration.onlyIds'), exact: true }).click();
  await inspector.getByRole('textbox', { name: `${t('collaboration.areas.workflows')} — ${t('collaboration.ids')}`, exact: true }).fill('welcome');
  await matrix.getByRole('button', { name: /New invitation.*Automations/ }).click();
  await inspector.getByRole('radio', { name: t('collaboration.levels.view'), exact: true }).click();
  await inspector.getByRole('radio', { name: t('collaboration.onlyIds'), exact: true }).click();
  await inspector.getByRole('textbox', { name: `${t('collaboration.areas.automations')} — ${t('collaboration.ids')}`, exact: true }).fill('event:welcome, action:welcome');
  await page.getByRole('button', { name: t('collaboration.create'), exact: true }).click();
  await expect(page.locator('#collaboration-link')).toHaveAttribute('type', 'password');
  await page.getByRole('button', { name: t('collaboration.qr'), exact: true }).click();
  const qr = page.getByRole('img', { name: t('collaboration.privateLink') }); await expect(qr).toBeVisible();
  await expect.poll(() => qr.evaluate((node) => {
    const canvas = node as HTMLCanvasElement;
    const pixels = canvas.getContext('2d')!.getImageData(0, 0, canvas.width, canvas.height).data;
    let ink = 0; for (let i = 0; i < pixels.length; i += 4) if (pixels[i]! < 80 && pixels[i + 3]! > 0) ink += 1;
    return ink;
  })).toBeGreaterThan(100);
  expect(network.some((url) => url.includes(privateSecret))).toBe(false);
  await page.getByRole('button', { name: t('collaboration.hideQr'), exact: true }).click();
  await expect(qr).toHaveCount(0);
  await page.getByRole('button', { name: t('collaboration.done'), exact: true }).click();

  const issued = await readHostState(page);
  const granted = issued.collaboration!.invitations[0]!.grant;
  expect(granted.resources.workflow).toEqual(['welcome']);
  expect(granted.resources.automation).toEqual(['event:welcome', 'action:welcome']);
  expect(granted.capabilities).toContain('automation.read');
  issued.collaboration!.requests = [{ requestId: '22222222-2222-4222-8222-222222222222', clientLabel: 'Moderator PC', clientKind: 'desktop', grant: granted, state: 'pending', expiresAt: Date.now() + 600000 }];
  issued.collaboration!.status.pendingRequests = 1;
  await writeHostState(page, issued);
  await page.getByRole('button', { name: t('collaboration.approve'), exact: true }).click();
  const dialog = page.getByRole('dialog'); await expect(dialog).toContainText('22222222-2222-4222-8222-222222222222');
  await expect(dialog).toContainText('welcome');
  expect((await readHostState(page)).calls.some((c) => c.method === 'collaboration.requests.approve')).toBe(false);
  await dialog.getByRole('button', { name: t('common.apply'), exact: true }).click(); await expect(dialog).not.toBeVisible();
  const approved = await readHostState(page); const c = approved.collaboration!;
  c.principals = [{ principalId: '33333333-3333-4333-8333-333333333333', clientLabel: 'Moderator PC', clientKind: 'desktop', grant: granted, grantRevision: '1', revoked: false }];
  c.sessions = ['44444444-4444-4444-8444-444444444444', '55555555-5555-4555-8555-555555555555'].map((id) => ({ sessionId: id, principalId: c.principals[0]!.principalId, clientKind: 'desktop', createdAt: Date.now(), lastActivityAt: Date.now(), expiresAt: Date.now() + 600000, revoked: false }));
  c.status.activeSessions = 2; c.status.gateway.connections = 2;
  c.status.gateway.sessionConnections = Object.fromEntries(c.sessions.map((s) => [s.sessionId, 1]));
  await writeHostState(page, approved);

  // Edit the identity's LIVE permission through its matrix cell; the write waits for confirmation.
  await matrix.getByRole('button', { name: /Moderator PC, LIVE/ }).click();
  await inspector.getByRole('radio', { name: t('collaboration.levels.none'), exact: true }).click();
  await page.getByRole('button', { name: t('collaboration.grantSave'), exact: true }).click();
  await expect(dialog).toContainText('workflows.read'); await expect(dialog).not.toContainText('live.read');
  await dialog.getByRole('button', { name: t('common.apply'), exact: true }).click(); await expect(inspector).toHaveCount(0);

  // Revoking the identity also revokes every one of its device sessions.
  await matrix.getByRole('button', { name: t('collaboration.revoke'), exact: true }).click();
  await expect(dialog).toContainText(t('collaboration.revokePrincipalMessage'));
  await dialog.getByRole('button', { name: t('common.apply'), exact: true }).click();
  const final = await readHostState(page);
  expect(final.collaboration!.principals.map((p) => p.revoked)).toEqual([true]);
  expect(final.collaboration!.sessions.map((s) => s.revoked)).toEqual([true, true]);
  expect(final.liveStatus.connected).toBe(true);
  expect(final.calls.filter((c) => c.method === 'collaboration.grants.set')[0]?.params['expectedRevision']).toBe('1');
  expect(final.calls.some((c) => ['live.disconnect', 'live.connect', 'sync.enable', 'sync.disable'].includes(c.method))).toBe(false);
  await assertNoUnhandledCalls(page); consoleCapture.assertClean();
  await page.locator('.settings-page__content').evaluate((node) => { node.scrollTop = 0; });
  await page.screenshot({ path: testInfo.outputPath('owner-collaboration-wide.png') });
});

test('Spanish setup remains usable in a narrow window and never enables itself on mount', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const fixture = ownerState(); const s = fixture.collaboration!.status;
  s.configuration.enabled = false; s.configuration.advertisedOrigin = null; s.configuration.tlsCertificate = null; s.configuration.tlsPrivateKey = null;
  s.configurationRevision = '0'; s.gateway.running = false; s.gateway.configurationRevision = null;
  const consoleCapture = await installFakeHost(page, fixture, { locale: 'es' });
  await page.goto('/#/settings?section=collaboration'); await waitForFrontend(page);
  await expect(page.getByText(te('collaboration.stopped'), { exact: true })).toBeVisible();
  expect((await readHostState(page)).calls.some((call) => call.method === 'collaboration.configure')).toBe(false);
  await page.getByRole('button', { name: te('collaboration.connectionSettings'), exact: true }).first().click();
  await page.getByRole('switch', { name: te('collaboration.enabled'), exact: true }).check();
  await page.getByLabel(te('collaboration.bind'), { exact: true }).fill('0.0.0.0:8788');
  await page.getByLabel(te('collaboration.origin'), { exact: true }).fill('https://pc-a.example:8788');
  await page.getByLabel(te('collaboration.certificate'), { exact: true }).fill('/host/cert.pem');
  await page.getByLabel(te('collaboration.privateKey'), { exact: true }).fill('/host/key.pem');
  await page.getByRole('button', { name: te('collaboration.save'), exact: true }).click();
  await page.getByRole('button', { name: te('collaboration.backToAccess'), exact: true }).click();
  await expect(page.getByText(te('collaboration.running'), { exact: true })).toBeVisible();
  const configured = await readHostState(page);
  expect(configured.calls.find((call) => call.method === 'collaboration.configure')?.params['expectedRevision']).toBe('0');
  await page.getByRole('button', { name: te('collaboration.connectionSettings'), exact: true }).first().click();
  await page.getByRole('switch', { name: te('collaboration.enabled'), exact: true }).uncheck();
  await page.getByRole('button', { name: te('collaboration.save'), exact: true }).click();
  const dialog = page.getByRole('dialog'); await expect(dialog).toContainText(te('collaboration.disableMessage'));
  await dialog.getByRole('button', { name: te('common.apply'), exact: true }).click();
  await page.getByRole('button', { name: te('collaboration.backToAccess'), exact: true }).click();
  await expect(page.getByText(te('collaboration.stopped'), { exact: true })).toBeVisible();
  const disabled = await readHostState(page);
  expect(disabled.collaboration!.status.configuration.enabled).toBe(false); expect(disabled.liveStatus.connected).toBe(true);
  await assertNoUnhandledCalls(page); consoleCapture.assertClean();
  await page.locator('.settings-page').evaluate((node) => { node.scrollTop = 0; });
  await page.screenshot({ path: testInfo.outputPath('owner-collaboration-mobile.png') });
});
