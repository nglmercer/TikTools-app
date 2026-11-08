import { test, expect } from '@playwright/test';
import type { ControllerStatus, WorkflowRow } from '../../src/web/features/controller/model/controller.ts';

for (const locale of ['en', 'es'] as const) {
  test(`controller confirms commands and reviews unknown outcomes (${locale})`, async ({ page }) => {
    await page.setViewportSize({ width: locale === 'es' ? 390 : 1100, height: 900 });
    const origin = locale === 'es' ? 'https://moderator-owner-with-a-long-public-hostname.example' : 'https://host.example';
    const status: ControllerStatus = { epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin, errorCode: null,
      hello: { hostId: '11111111-1111-4111-8111-111111111111', principalId: 'moderator', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['workflows.read', 'workflows.toggle', 'workflows.edit', 'commands.read'], features: { workflowDisable: true, workflowDelete: true } },
      commandJournal: { binding: { profileId: 'desk', origin, hostId: '11111111-1111-4111-8111-111111111111', principalId: 'moderator', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
    const rows: WorkflowRow[] = [{ graph: { id: 'welcome', name: 'Hosted workflow', enabled: true }, version: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '1' } }];
    const errors: string[] = []; page.on('pageerror', () => errors.push('Controller page failed'));
    await page.addInitScript(({ initial, locale }) => {
      window.__tiktools_controller__ = {};
      localStorage.setItem('tiktok-live-locale', locale);
      const state = { ...initial, calls: [] as Array<{ method: string; params: Record<string, unknown> }>, unhandled: [] as string[] };
      (window as unknown as { __CONTROLLER_FIXTURE__: typeof state }).__CONTROLLER_FIXTURE__ = state;
      window.ipc = { postMessage(raw) {
        const request = JSON.parse(raw) as { type?: string; id: string; method: string; params?: Record<string, unknown> };
        if (request.type === 'frontend-ready') return;
        const params = request.params ?? {}; state.calls.push({ method: request.method, params });
        let result: unknown; let error: { code: string; message: string } | undefined;
        const journal = state.status.commandJournal!;
        if (request.method === 'controller.status') result = state.status;
        else if (request.method === 'controller.profiles') result = [];
        else if (request.method === 'controller.workflows') result = { epoch: state.status.epoch, viewRevision: state.status.viewRevision, rows: state.rows, next: null };
        else if (request.method === 'controller.workflow.command') {
          const intent = { commandId: crypto.randomUUID(), operation: params['operation'] as 'disable' | 'delete', resourceId: String(params['id']), expectedRevision: String(params['expectedRevision']), hostInstanceId: 'instance', createdAt: Date.now() };
          state.status.viewRevision = String(Number(state.status.viewRevision) + 1);
          if (intent.operation === 'disable') {
            state.rows[0]!.graph['enabled'] = false; state.rows[0]!.version.revision = '2';
            journal.recent.unshift({ intent, outcome: 'succeeded', revision: '2', errorCode: null, recordedAt: Date.now() });
            result = journal;
          } else {
            state.rows = []; journal.pending = intent;
            error = { code: 'transport', message: 'Acknowledgement lost' };
          }
        } else if (request.method === 'controller.command.resolve') error = { code: 'forbidden', message: 'Outcome not confirmed' };
        else if (request.method === 'controller.command.review' && params['acknowledgeUnknown'] === true && params['commandId'] === journal.pending?.commandId) {
          journal.recent.unshift({ intent: journal.pending!, outcome: 'reviewed-unknown', revision: null, errorCode: 'command_unknown', recordedAt: Date.now() });
          journal.pending = null; state.status.viewRevision = String(Number(state.status.viewRevision) + 1); result = journal;
        } else { state.unhandled.push(request.method); error = { code: 'forbidden', message: 'Unexpected controller call' }; }
        // Snapshot now; delivery is asynchronous like the actual native bridge.
        const response = JSON.stringify({ type: 'rpc-response', response: { jsonrpc: '2.0', id: request.id, ...(error ? { error } : { result }) } });
        queueMicrotask(() => window.__webview_on_message__?.(response));
      } };
    }, { initial: { status, rows }, locale });
    await page.goto('/');
    await page.getByRole('button', { name: 'Hosted workflow · 1', exact: true }).click();
    const text = locale === 'es' ? { disable: 'Desactivar workflow', remove: 'Eliminar workflow', cancel: 'Cancelar', draft: 'Borrador local', confirm: 'Confirmado por el anfitrión', check: 'Consultar resultado', review: 'Revisar resultado incierto', acknowledge: 'Conservar como incierto', reviewed: 'Resultado incierto revisado' } :
      { disable: 'Disable workflow', remove: 'Delete workflow', cancel: 'Cancel', draft: 'Local draft', confirm: 'Confirmed by host', check: 'Check outcome', review: 'Review unknown outcome', acknowledge: 'Keep outcome unknown', reviewed: 'Reviewed unknown outcome' };
    await page.getByRole('textbox', { name: text.draft, exact: true }).fill('local draft retained');
    await page.getByRole('button', { name: text.disable, exact: true }).click();
    const dialog = page.getByRole('dialog'); await expect(dialog).toContainText(origin);
    await expect(dialog).toContainText('11111111-1111-4111-8111-111111111111');
    await dialog.getByRole('button', { name: text.cancel, exact: true }).click();
    await page.getByRole('button', { name: text.disable, exact: true }).click();
    await dialog.getByRole('button', { name: text.disable, exact: true }).click();
    await expect(page.getByText(`${text.confirm} · welcome`, { exact: true })).toBeVisible();
    await expect(page.getByRole('textbox', { name: text.draft, exact: true })).toHaveValue('local draft retained');
    await page.getByRole('button', { name: text.remove, exact: true }).click();
    await dialog.getByRole('button', { name: text.remove, exact: true }).click();
    await expect(page.getByRole('textbox', { name: text.draft, exact: true })).toHaveValue('local draft retained');
    await page.getByRole('button', { name: text.check, exact: true }).click();
    await page.getByRole('button', { name: text.review, exact: true }).click();
    await expect(dialog).toContainText('welcome');
    await page.screenshot({ path: `/tmp/tiktools-controller-command-${locale}.png` });
    await dialog.getByRole('button', { name: text.acknowledge, exact: true }).click();
    await expect(page.getByText(`${text.reviewed} · welcome`, { exact: true })).toBeVisible();
    const trace = await page.evaluate(() => (window as unknown as { __CONTROLLER_FIXTURE__: { calls: Array<{ method: string; params: Record<string, unknown> }>; unhandled: string[] } }).__CONTROLLER_FIXTURE__);
    expect(trace.unhandled).toEqual([]); expect(errors).toEqual([]);
    expect(trace.calls.every((c) => c.method.startsWith('controller.'))).toBe(true);
    const mutations = trace.calls.filter((c) => c.method === 'controller.workflow.command'); expect(mutations).toHaveLength(2);
    expect(mutations.map((c) => c.params['expectedRevision'])).toEqual(['1', '2']);
    expect(mutations.every((c) => !('commandId' in c.params) && !('graph' in c.params))).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
}
