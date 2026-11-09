import { test, expect } from '@playwright/test';
import type { ControllerStatus } from '../../src/web/features/controller/model/controller.ts';

for (const locale of ['en', 'es'] as const) {
  test(`private proposal consent fits the controller and clears after access loss (${locale})`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: locale === 'es' ? 390 : 1100, height: 900 });
    const status: ControllerStatus = { proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://owner-with-a-long-public-hostname.example', errorCode: null,
      hello: { connectionId: 'connection', hostId: 'host-A', principalId: 'author', sessionId: 'device', hostInstanceId: 'instance', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } },
      commandJournal: { binding: { profileId: 'profile', origin: 'https://owner-with-a-long-public-hostname.example', hostId: 'host-A', principalId: 'author', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
    const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
    await page.addInitScript(({ status, locale }) => {
      window.__tiktools_controller__ = {}; localStorage.setItem('tiktok-live-locale', locale);
      const state = { status, calls: [] as string[] };
      (window as unknown as { __PROPOSAL_FIXTURE__: typeof state }).__PROPOSAL_FIXTURE__ = state;
      window.ipc = { postMessage(raw) {
        const request = JSON.parse(raw) as { type?: string; id: string; method: string; params?: Record<string, unknown> };
        if (request.type === 'frontend-ready') return;
        const p = request.params ?? {}; state.calls.push(request.method);
        const h = state.status.hello!; const tuple = { epoch: state.status.epoch, connectionId: h.connectionId, hostInstanceId: h.hostInstanceId, grantRevision: h.grantRevision };
        let result: unknown; let error: { code: string; message: string } | undefined;
        if (request.method === 'controller.status') result = state.status;
        else if (request.method === 'controller.profiles') result = [];
        else if (request.method === 'controller.workflows') result = { epoch: '1', viewRevision: '1', rows: [], next: null };
        else if (request.method === 'controller.proposals.list') result = { ...tuple, storeRevision: '0', rows: [], next: null };
        else if (request.method === 'controller.proposals.inspect') result = { ...tuple, kind: p['kind'], id: p['id'], revision: '0', inspectionId: 'native-token', summary: null };
        else error = { code: 'forbidden', message: 'Unexpected controller write' };
        queueMicrotask(() => window.__webview_on_message__?.(JSON.stringify({ type: 'rpc-response', response: { jsonrpc: '2.0', id: request.id, ...(error ? { error } : { result }) } })));
      } };
    }, { status, locale });
    await page.goto('/');
    const labels = locale === 'es' ? { title: 'Propuestas de automatización', id: 'ID de la propuesta', start: 'Preparar propuesta nueva', save: 'Guardar propuesta en esta PC', consent: 'valores sensibles', builder: 'Crear una propuesta con campos', name: 'Nombre', message: 'Mensaje', actionType: 'Tipo de acción', viewer: 'ID exacto del espectador', delta: 'Cambio de puntos', prepare: 'Preparar borrador', use: 'Usar este borrador', warning: 'No se guardará ni enviará todavía' } :
      { title: 'Automation proposals', id: 'Proposal ID', start: 'Prepare new proposal', save: 'Save proposal on this PC', consent: 'sensitive values', builder: 'Create a proposal with fields', name: 'Name', message: 'Message', actionType: 'Action type', viewer: 'Exact viewer ID', delta: 'Points change', prepare: 'Prepare draft', use: 'Use this draft', warning: 'It will not be saved or sent yet' };
    const region = page.getByRole('region', { name: labels.title, exact: true });
    await region.getByLabel(labels.id, { exact: true }).fill('same'); await region.getByRole('button', { name: labels.start, exact: true }).click();
    await region.getByText(labels.builder, { exact: true }).click();
    await region.getByLabel(labels.name, { exact: true }).fill('Welcome');
    await region.getByLabel(labels.message, { exact: true }).fill('Hello from the moderator');
    await region.getByRole('button', { name: labels.prepare, exact: true }).click();
    const preview = page.getByRole('dialog');
    await expect(preview).toContainText(labels.warning);
    await expect(preview.locator('pre')).toContainText('"enabled": false');
    expect(await page.evaluate(() => (window as unknown as { __PROPOSAL_FIXTURE__: { calls: string[] } }).__PROPOSAL_FIXTURE__.calls.some((method) => method === 'controller.proposals.save' || method === 'controller.proposals.submit'))).toBe(false);
    await preview.getByRole('button', { name: labels.use, exact: true }).click();
    await expect(region.locator('textarea')).toHaveValue(/Hello from the moderator/);
    await region.getByRole('combobox', { name: labels.actionType, exact: true }).selectOption('core.points');
    await region.getByLabel(labels.viewer, { exact: true }).fill('alice');
    await region.getByLabel(labels.delta, { exact: true }).fill('-3');
    await region.getByRole('button', { name: labels.prepare, exact: true }).click();
    await expect(preview.locator('pre')).toContainText('"uniqueId": "alice"');
    await expect(preview.locator('pre')).toContainText('"delta": -3');
    await expect(preview.locator('pre')).toContainText('"enabled": false');
    await preview.getByRole('button', { name: labels.use, exact: true }).click();
    await expect(region.locator('textarea')).toHaveValue(/"typeId": "core\.points"/);
    expect(await page.evaluate(() => (window as unknown as { __PROPOSAL_FIXTURE__: { calls: string[] } }).__PROPOSAL_FIXTURE__.calls.some((method) => method === 'controller.proposals.save' || method === 'controller.proposals.submit'))).toBe(false);
    const text = JSON.stringify({ id: 'same', enabled: false, typeId: 'core.log', config: { message: 'private proposal fixture 🧡' } }, null, 2);
    await region.locator('textarea').fill(text); await region.getByRole('button', { name: labels.save, exact: true }).click();
    const dialog = page.getByRole('dialog'); await expect(dialog).toContainText(status.origin!); await expect(dialog).toContainText('host-A'); await expect(dialog).toContainText(labels.consent); await expect(dialog.locator('pre')).toHaveText(text);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath(`proposal-consent-${locale}.png`) });
    await page.evaluate(() => { const state = (window as unknown as { __PROPOSAL_FIXTURE__: { status: ControllerStatus } }).__PROPOSAL_FIXTURE__; state.status.hello!.capabilities = []; state.status.hello!.grantRevision = '2'; });
    await expect(dialog).toHaveCount(0); await expect(region).toHaveCount(0);
    expect(await page.evaluate(() => [...Object.values(localStorage), ...Object.values(sessionStorage)].some((v) => v.includes('private proposal fixture')))).toBe(false);
    expect(errors).toEqual([]);
  });
}
