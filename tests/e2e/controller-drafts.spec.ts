import { test, expect } from '@playwright/test';
import type { ControllerStatus, WorkflowRow } from '../../src/web/features/controller/model/controller.ts';

for (const locale of ['en', 'es'] as const) {
  test(`controller explicitly saves, compares and restores a local draft (${locale})`, async ({ page }) => {
    await page.setViewportSize({ width: locale === 'es' ? 390 : 1100, height: 900 });
    const origin = 'https://moderator-owner-with-a-long-public-hostname.example';
    const status: ControllerStatus = { draftsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin, errorCode: null,
      hello: { hostId: 'host-A', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['workflows.read'], resourceRestrictions: { workflow: ['welcome'] }, features: {} } };
    let row: WorkflowRow = { graph: { id: 'welcome', name: 'Hosted workflow' }, version: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '1' } };
    let stored: { id: string; baseRevision: string; text: string; updatedAt: number } | null = null; let revision = '0';
    const calls: Array<{ method: string; params: Record<string, unknown> }> = []; const unhandled: string[] = []; const errors: string[] = [];
    page.on('pageerror', () => errors.push('Controller page failed'));
    // The fake native file store lives outside the page so reload exercises
    // a new Vue model and does not turn localStorage into product draft storage.
    await page.exposeFunction('__DRAFT_NATIVE__', (request: { id: string; method: string; params?: Record<string, unknown> }) => {
      const params = request.params ?? {}; calls.push({ method: request.method, params });
      const envelope = { epoch: status.epoch, viewRevision: status.viewRevision, storeRevision: revision }; let result: unknown;
      if (request.method === 'controller.status') result = status;
      else if (request.method === 'controller.profiles') result = [];
      else if (request.method === 'controller.workflows') result = { ...envelope, rows: [row], next: null };
      else if (request.method === 'controller.drafts.list') result = { ...envelope, rows: stored ? [{ id: stored.id, baseRevision: stored.baseRevision, updatedAt: 1, textBytes: stored.text.length }] : [], next: null };
      else if (request.method === 'controller.drafts.get') result = { ...envelope, draft: stored };
      else if (request.method === 'controller.drafts.save') {
        if (params['acknowledgeStorage'] !== true || params['expectedStoreRevision'] !== revision) throw new Error('Unexpected unreviewed draft replacement');
        stored = { id: String(params['id']), baseRevision: String(params['baseRevision']), text: String(params['text']), updatedAt: 1 };
        revision = String(Number(revision) + 1); result = { ...envelope, storeRevision: revision };
      } else if (request.method === 'controller.drafts.delete') { stored = null; revision = String(Number(revision) + 1); result = { ...envelope, storeRevision: revision }; }
      else { unhandled.push(request.method); result = null; }
      return JSON.stringify({ type: 'rpc-response', response: { jsonrpc: '2.0', id: request.id, result } });
    });
    await page.addInitScript(({ locale }) => {
      window.__tiktools_controller__ = {}; localStorage.setItem('tiktok-live-locale', locale);
      window.ipc = { postMessage(raw) {
        const request = JSON.parse(raw) as { type?: string; id: string; method: string; params?: Record<string, unknown> };
        if (request.type === 'frontend-ready') return;
        const native = (window as unknown as { __DRAFT_NATIVE__: (request: unknown) => Promise<string> }).__DRAFT_NATIVE__;
        void native(request).then((response) => window.__webview_on_message__?.(response));
      } };
    }, { locale });
    const text = locale === 'es' ? { draft: 'Borrador local', save: 'Guardar borrador en esta PC', cancel: 'Cancelar', consent: 'incluidos los valores sensibles', matches: 'Este texto coincide con la copia guardada en esta PC.', current: 'Versión actual del anfitrión', rebase: 'Usar la revisión actual para esta combinación local · 2', discard: 'Descartar borrador' } :
      { draft: 'Local draft', save: 'Save draft on this PC', cancel: 'Cancel', consent: 'including any sensitive values', matches: 'This text matches the saved copy on this PC.', current: 'Current host version', rebase: 'Use the current revision for this local merge · 2', discard: 'Discard draft' };
    await page.goto('/'); await page.getByRole('button', { name: 'Hosted workflow · 1', exact: true }).click();
    const editor = page.getByRole('textbox', { name: text.draft, exact: true }); await editor.fill('private unfinished fixture draft');
    const save = () => page.getByRole('button', { name: text.save, exact: true });
    await save().click(); const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText(origin); await expect(dialog).toContainText('host-A'); await expect(dialog).toContainText(text.consent);
    await dialog.getByRole('button', { name: text.cancel, exact: true }).click(); expect(calls.filter((c) => c.method === 'controller.drafts.save')).toHaveLength(0);
    await save().click(); await dialog.getByRole('button', { name: text.save, exact: true }).click(); await expect(page.getByText(text.matches, { exact: true })).toBeVisible();
    await page.reload(); await page.getByRole('button', { name: 'Hosted workflow · 1', exact: true }).click(); await expect(editor).toHaveValue('private unfinished fixture draft');
    row = { graph: { id: 'welcome', name: 'Owner changed' }, version: { ...row.version, revision: '2' } }; status.viewRevision = '2';
    await expect(page.getByRole('textbox', { name: text.current, exact: true })).toHaveValue(JSON.stringify(row.graph, null, 2));
    await expect(editor).toHaveValue('private unfinished fixture draft'); await editor.fill('explicit local merge');
    await page.getByRole('button', { name: text.rebase, exact: true }).click();
    await save().click(); await expect(dialog).toContainText('welcome');
    await page.screenshot({ path: `/home/meme/.cache/tiktools-test-runs/controller-drafts-${locale}.png` });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await dialog.getByRole('button', { name: text.save, exact: true }).click(); await expect(page.getByText(text.matches, { exact: true })).toBeVisible();
    await page.reload(); await page.getByRole('button', { name: 'Owner changed · 2', exact: true }).click(); await expect(editor).toHaveValue('explicit local merge');
    await page.getByRole('button', { name: text.discard, exact: true }).click(); await expect(editor).toHaveValue(JSON.stringify(row.graph, null, 2));
    expect(stored).toBeNull(); expect(errors).toEqual([]); expect(unhandled).toEqual([]);
    expect(calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward' && !c.method.endsWith('.command'))).toBe(true);
    const saves = calls.filter((c) => c.method === 'controller.drafts.save'); expect(saves).toHaveLength(2);
    expect(saves.map((c) => c.params['baseRevision'])).toEqual(['1', '2']);
  });
}
