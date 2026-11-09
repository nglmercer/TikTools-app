import { test, expect } from '@playwright/test';
import type { ControllerStatus, PointsPage } from '../../src/web/features/controller/model/controller.ts';

for (const locale of ['en', 'es'] as const) {
  test(`controller reviews a points adjustment with a bounded layout (${locale})`, async ({ page }) => {
    await page.setViewportSize({ width: locale === 'es' ? 390 : 1100, height: 900 });
    const origin = 'https://moderator-owner-with-a-long-public-hostname.example';
    const status: ControllerStatus = { epoch: '1', viewRevision: '1', pointsRevision: '1', pointsLoading: false, phase: 'ready', profile: 'desk', origin, errorCode: null,
      hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host-A', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['live.read', 'points.read', 'points.adjust', 'commands.read'], features: { pointsProjection: true, pointsAdjust: true } },
      commandJournal: { binding: { profileId: 'desk', origin, hostId: 'host-A', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
    const points: PointsPage = { epoch: '1', pointsRevision: '1', loading: false, errorCode: null, currencyName: 'Gems', rows: [], selectedId: null, selected: null, selectedVersion: null };
    const errors: string[] = []; page.on('pageerror', () => errors.push('Controller page failed'));
    await page.addInitScript(({ status, points, locale }) => {
      window.__tiktools_controller__ = {}; localStorage.setItem('tiktok-live-locale', locale);
      const state = { status, points, calls: [] as Array<{ method: string; params: Record<string, unknown> }>, unhandled: [] as string[] };
      (window as unknown as { __POINTS_FIXTURE__: typeof state }).__POINTS_FIXTURE__ = state;
      window.ipc = { postMessage(raw) {
        const request = JSON.parse(raw) as { type?: string; id: string; method: string; params?: Record<string, unknown> };
        if (request.type === 'frontend-ready') return;
        const params = request.params ?? {}; state.calls.push({ method: request.method, params });
        let result: unknown;
        if (request.method === 'controller.status') result = state.status;
        else if (request.method === 'controller.profiles') result = [];
        else if (request.method === 'controller.workflows') result = { epoch: '1', viewRevision: state.status.viewRevision, rows: [], next: null };
        else if (request.method === 'controller.live.status') result = { epoch:'1', connectionId:'connection', connected:true, uniqueId:'creator', native:true };
        else if (request.method === 'controller.live.feed') result = { epoch:'1', connectionId:'connection', oldestCursor:'1', latestCursor:'1', nextCursor:'1', gap:false, truncated:false,
          events: params['after'] === '0' ? [{ cursor:'1', kind:'chat', author:'alice', authorLookupId:'alice', nickname:'Alice', text:'Hello!' }] : [] };
        else if (request.method === 'controller.points') result = state.points;
        else if (request.method === 'controller.points.select') {
          state.points.selectedId = String(params['uniqueId']);
          state.points.selectedVersion = { kind: 'viewer', scope: 'host', id: state.points.selectedId, revision: '0' };
          state.points.pointsRevision = state.status.pointsRevision = String(Number(state.status.pointsRevision) + 1); result = {};
        } else if (request.method === 'controller.points.command') {
          const intent = { commandId: crypto.randomUUID(), operation: 'adjust-points' as const, resourceId: String(params['uniqueId']), delta: Number(params['delta']), currencyName: String(params['currencyName']), hostInstanceId: 'instance', createdAt: Date.now() };
          state.status.commandJournal!.recent.unshift({ intent, outcome: 'succeeded', revision: '1', errorCode: null, recordedAt: Date.now(), pointsResult: { delta: 3.75, totalPoints: 3.75, level: 1 } });
          state.status.viewRevision = '2'; result = {};
        } else { state.unhandled.push(request.method); }
        const response = JSON.stringify({ type: 'rpc-response', response: { jsonrpc: '2.0', id: request.id, result } });
        queueMicrotask(() => window.__webview_on_message__?.(response));
      } };
    }, { status, points, locale });
    await page.goto('/');
    const text = locale === 'es' ? { search: 'Buscar visor por ID', find: 'Consultar visor', feed: 'Consultar puntos de @alice', amount: 'Cantidad a sumar o restar (Gems)', adjust: 'Ajustar puntos', cancel: 'Cancelar', create: 'El ajuste creará uno.', history: 'Historial reciente de comandos', requested: 'Cantidad solicitada: 3 Gems', effective: 'Ajuste aplicado: 3.75 Gems' } :
      { search: 'Find viewer by ID', find: 'Look up viewer', feed: 'Look up points for @alice', amount: 'Amount to add or subtract (Gems)', adjust: 'Adjust points', cancel: 'Cancel', create: 'The adjustment will create one.', history: 'Recent command history', requested: 'Requested amount: 3 Gems', effective: 'Applied adjustment: 3.75 Gems' };
    await page.getByRole('button', { name: text.feed, exact: true }).click();
    await expect(page.getByRole('textbox', { name: text.search, exact: true })).toHaveValue('alice');
    expect(await page.evaluate(() => (window as unknown as { __POINTS_FIXTURE__: { calls: Array<{ method: string }> } }).__POINTS_FIXTURE__.calls.some((call) => call.method === 'controller.points.command'))).toBe(false);
    await page.getByRole('textbox', { name: text.search, exact: true }).fill('@new-viewer');
    await page.getByRole('button', { name: text.find, exact: true }).click();
    await expect(page.getByText(text.create, { exact: false })).toBeVisible();
    await page.getByRole('spinbutton', { name: text.amount, exact: true }).fill('3');
    await page.getByRole('button', { name: text.adjust, exact: true }).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: `/home/meme/.cache/tiktools-test-runs/controller-points-${locale}.png` });
    await page.getByText(text.history, { exact: true }).click();
    await expect(page.getByText(text.requested, { exact: true })).toBeVisible();
    await expect(page.getByText(text.effective, { exact: false })).toBeVisible();
    const trace = await page.evaluate(() => (window as unknown as { __POINTS_FIXTURE__: { calls: Array<{ method: string; params: Record<string, unknown> }>; unhandled: string[] } }).__POINTS_FIXTURE__);
    expect(errors).toEqual([]); expect(trace.unhandled).toEqual([]);
    expect(trace.calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward')).toBe(true);
    expect(trace.calls.filter((c) => c.method === 'controller.points.command').map((c) => c.params)).toEqual([{ epoch: '1', connectionId: 'connection', uniqueId: 'new-viewer', delta: 3, currencyName: 'Gems' }]);
  });
}
