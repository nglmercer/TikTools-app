import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, readHostState, waitForFrontend, writeHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

for (const locale of ['en', 'es'] as const) {
  test(`owner editor retains conflicts and reviews recreation (${locale})`, async ({ page }, info) => {
    if (locale === 'es') await page.setViewportSize({ width: 390, height: 844 });
    const state = emptyState();
    state.automationSnapshot = {
      actions: [{ schemaVersion: 2, id: 'same', name: 'Local action', enabled: false, typeId: 'core.log', config: { message: 'fixture only' } }],
      events: [{ schemaVersion: 1, id: 'same', name: 'Starting event', enabled: false, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'user', runMode: 'all', actionIds: ['same'] }],
      plugins: [], actionTypes: [], eventTypes: [], translations: {},
    };
    const capture = await installFakeHost(page, state, { locale });
    await page.goto('/#/automations'); await waitForFrontend(page);
    await page.locator('.automation-library tbody tr').filter({ hasText: 'Starting event' }).click();
    const name = page.locator('.event-editor input[type="text"]').first();
    await name.fill('My retained draft');
    const host = await readHostState(page);
    (host.automationSnapshot['events'] as Array<Record<string, unknown>>)[0]!['name'] = 'Host changed name';
    host.automationVersions = { 'action:same': '1', 'event:same': '2' };
    await writeHostState(page, host); await emitTopic(page, 'workflow.changed', { kind: 'event', id: 'same', change: 'updated' });
    await expect(page.getByText(locale === 'en'
      ? 'The host record changed since you opened it. Your starting version and draft are retained.'
      : 'El registro del anfitrión cambió desde que lo abriste. Tu versión inicial y tu borrador se conservan.')).toBeVisible();
    await expect(name).toHaveValue('My retained draft');
    const save = page.getByRole('button', { name: locale === 'en' ? 'Save' : 'Guardar', exact: true });
    await save.click();
    await expect(save).toBeDisabled();
    const failed = await readHostState(page);
    expect(failed.calls.find((r) => r.method === 'automation.update')?.meta?.['expectedRevision']).toBe('1');
    await page.getByRole('button', { name: locale === 'en' ? 'Compare with host' : 'Comparar con el anfitrión', exact: true }).click();
    const dialog = page.getByRole('dialog').filter({ has: page.locator('.automation-review') });
    await expect(dialog.locator('pre')).toHaveCount(3);
    await expect(dialog).toContainText('Starting event'); await expect(dialog).toContainText('My retained draft'); await expect(dialog).toContainText('Host changed name');
    expect(await dialog.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    for (const button of await dialog.locator('.base-dialog__footer .base-button__label').all()) {
      expect(await button.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    }
    await page.screenshot({ path: info.outputPath(`owner-automation-review-${locale}.png`) });
    await dialog.getByRole('button', { name: locale === 'en' ? 'Use this version as the draft base' : 'Usar esta versión como base del borrador', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    expect((await readHostState(page)).calls.filter((r) => r.method === 'automation.update')).toHaveLength(1);
    await expect(name).toHaveValue('My retained draft'); await save.click();
    await expect(save).toBeDisabled();
    await expect.poll(async () => (await readHostState(page)).automationVersions?.['event:same']).toBe('3');
    const afterSave = await readHostState(page);
    expect(afterSave.calls.filter((r) => r.method === 'automation.update').at(-1)?.meta?.['expectedRevision']).toBe('2');
    afterSave.automationSnapshot['events'] = []; afterSave.automationVersions!['event:same'] = '4';
    await writeHostState(page, afterSave); await emitTopic(page, 'workflow.changed', { kind: 'event', id: 'same', change: 'deleted' });
    await name.fill('Draft after deletion'); await save.click();
    await expect(save).toBeDisabled();
    await page.getByRole('button', { name: locale === 'en' ? 'Compare with host' : 'Comparar con el anfitrión', exact: true }).click();
    await expect(dialog).toContainText(locale === 'en' ? 'The record is absent on the host.' : 'El registro no existe en el anfitrión.');
    await dialog.getByRole('button', { name: locale === 'en' ? 'Prepare recreation with this draft' : 'Preparar recreación con este borrador', exact: true }).click();
    expect((await readHostState(page)).calls.some((r) => r.method === 'automation.create')).toBe(false);
    await save.click();
    await expect.poll(async () => (await readHostState(page)).automationVersions?.['event:same']).toBe('5');
    const final = await readHostState(page);
    expect(final.calls.find((r) => r.method === 'automation.create')?.meta?.['expectedRevision']).toBe('4');
    expect(final.automationVersions?.['action:same']).toBe('1');
    expect(final.calls.some((r) => ['automation.fire', 'automation.enable', 'live.connect', 'points.adjust'].includes(r.method))).toBe(false);
    await assertNoUnhandledCalls(page); capture.assertClean();
  });
}
