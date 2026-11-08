import { test, expect } from '@playwright/test';
import type { ControllerStatus } from '../../src/web/features/controller/model/controller.ts';
import type { AutomationRow } from '../../src/web/features/controller/model/automations.ts';

for (const locale of ['en', 'es'] as const) {
  test(`controller confirms whole events and preserves unknown outcomes (${locale})`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: locale === 'es' ? 390 : 1100, height: 900 });
    const origin = 'https://moderator-owner-with-a-long-public-hostname.example';
    const status: ControllerStatus = { epoch: '1', viewRevision: '1', automationRevision: '1', phase: 'ready', profile: 'desk', origin, errorCode: null,
      hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host-A', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['automation.read', 'automation.toggle', 'automation.edit', 'commands.read'], features: { automationSummary: true, automationEventDeactivate: true } },
      commandJournal: { binding: { profileId: 'desk', origin, hostId: 'host-A', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
    const rows: AutomationRow[] = ['event', 'action'].map((kind) => ({ summary: { kind: kind as 'event' | 'action', id: 'same', name: kind === 'event' ? 'Welcome event with a long descriptive name for moderator review' : 'Read-only action', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: kind, id: 'same', revision: '1' } }));
    const errors: string[] = []; page.on('pageerror', () => errors.push('Controller page failed'));
    await page.addInitScript(({ status, rows, locale }) => {
      window.__tiktools_controller__ = {}; localStorage.setItem('tiktok-live-locale', locale);
      const state = { status, rows, calls: [] as Array<{ method: string; params: Record<string, unknown> }>, unhandled: [] as string[] };
      (window as unknown as { __AUTOMATION_FIXTURE__: typeof state }).__AUTOMATION_FIXTURE__ = state;
      window.ipc = { postMessage(raw) {
        const request = JSON.parse(raw) as { type?: string; id: string; method: string; params?: Record<string, unknown> };
        if (request.type === 'frontend-ready') return;
        const params = request.params ?? {}; state.calls.push({ method: request.method, params });
        let result: unknown; let error: { code: string; message: string } | undefined;
        if (request.method === 'controller.status') result = state.status;
        else if (request.method === 'controller.profiles') result = [];
        else if (request.method === 'controller.workflows') result = { epoch: state.status.epoch, viewRevision: state.status.viewRevision, rows: [], next: null };
        else if (request.method === 'controller.automations') result = { epoch: state.status.epoch, automationRevision: state.status.automationRevision, rows: state.rows, next: null };
        else if (request.method === 'controller.automation.command') {
          const intent = { commandId: crypto.randomUUID(), operation: params['operation'] as 'disable-event' | 'delete-event', resourceId: String(params['id']), hostInstanceId: 'instance', expectedRevision: String(params['expectedRevision']), createdAt: Date.now() };
          state.status.viewRevision = String(Number(state.status.viewRevision) + 1);
          if (intent.operation === 'disable-event') {
            state.rows[0]!.summary.enabled = false; state.rows[0]!.version.revision = '2'; state.status.automationRevision = '2';
            state.status.commandJournal!.recent.unshift({ intent, outcome: 'succeeded', revision: '2', errorCode: null, recordedAt: Date.now() }); result = {};
          } else { state.status.commandJournal!.pending = intent; error = { code: 'transport', message: 'Acknowledgement lost' }; }
        } else { state.unhandled.push(request.method); error = { code: 'forbidden', message: 'Unexpected controller call' }; }
        const response = JSON.stringify({ type: 'rpc-response', response: { jsonrpc: '2.0', id: request.id, ...(error ? { error } : { result }) } });
        queueMicrotask(() => window.__webview_on_message__?.(response));
      } };
    }, { status, rows, locale });
    await page.goto('/');
    const text = locale === 'es' ? { board: 'Automatizaciones del anfitrión', disable: 'Desactivar evento', remove: 'Eliminar evento', cancel: 'Cancelar', revision: 'Revisión 1', unknown: 'las nuevas modificaciones están bloqueadas', history: 'Comandos de este perfil' } :
      { board: 'Host automations', disable: 'Disable event', remove: 'Delete event', cancel: 'Cancel', revision: 'Revision 1', unknown: 'new changes are blocked', history: 'Commands for this profile' };
    const board = page.getByRole('region', { name: text.board, exact: true });
    await expect(board.getByRole('listitem')).toHaveCount(2);
    await expect(board.getByRole('listitem').nth(1).getByRole('button')).toHaveCount(0);
    await expect(page.getByRole('textbox')).toHaveCount(0);
    await board.getByRole('button', { name: text.disable, exact: true }).click();
    const dialog = page.getByRole('dialog'); await expect(dialog).toContainText(origin); await expect(dialog).toContainText('host-A'); await expect(dialog).toContainText(text.revision);
    await expect(dialog).toContainText(rows[0]!.summary.name);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath('automation-event-confirmation.png') });
    await dialog.getByRole('button', { name: text.cancel, exact: true }).click();
    await board.getByRole('button', { name: text.disable, exact: true }).click();
    await dialog.getByRole('button', { name: text.disable, exact: true }).click();
    await expect(board.getByRole('button', { name: text.disable, exact: true })).toBeDisabled();
    await board.getByRole('button', { name: text.remove, exact: true }).click();
    await page.evaluate(() => { const s = (window as unknown as { __AUTOMATION_FIXTURE__: { status: ControllerStatus } }).__AUTOMATION_FIXTURE__.status; s.viewRevision = '77'; s.pointsRevision = '900'; });
    await page.waitForTimeout(1100);
    await expect(dialog.getByRole('button', { name: text.remove, exact: true })).toBeEnabled();
    await dialog.getByRole('button', { name: text.remove, exact: true }).click();
    const journal = page.getByRole('region', { name: text.history, exact: true });
    await expect(journal).toContainText(text.unknown); await expect(journal).toContainText(text.remove);
    await expect(board.getByRole('button', { name: text.remove, exact: true })).toBeDisabled();
    const trace = await page.evaluate(() => (window as unknown as { __AUTOMATION_FIXTURE__: { calls: Array<{ method: string; params: Record<string, unknown> }>; unhandled: string[] } }).__AUTOMATION_FIXTURE__);
    expect(errors).toEqual([]); expect(trace.unhandled).toEqual([]);
    expect(trace.calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward')).toBe(true);
    expect(trace.calls.filter((c) => c.method === 'controller.automation.command').map((c) => c.params)).toEqual([
      { epoch: '1', automationRevision: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', id: 'same', expectedRevision: '1', operation: 'disable-event' },
      { epoch: '1', automationRevision: '2', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', id: 'same', expectedRevision: '2', operation: 'delete-event' },
    ]);
  });
}
