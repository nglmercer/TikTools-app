import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, readHostState, waitForFrontend, writeHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

for (const locale of ['en', 'es'] as const) {
  test(`owner editor retains conflicts and reviews recreation (${locale})`, async ({ page }, info) => {
    if (locale === 'es') await page.setViewportSize({ width: 390, height: 844 });
    const state = emptyState();
    // This spec exercises the opt-in advanced review flow.
    state.appState['app.resolution.mode'] = 'manual';
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
    const t = (key: string): string => testMessage(locale, key);
    await expect(page.getByText(t('automations.conflict.hostChanged'))).toBeVisible();
    await expect(name).toHaveValue('My retained draft');
    const save = page.getByRole('button', { name: t('automations.editor.save'), exact: true });
    await save.click();
    await expect(save).toBeDisabled();
    const failed = await readHostState(page);
    expect(failed.calls.find((r) => r.method === 'automation.update')?.meta?.['expectedRevision']).toBe('1');
    await page.getByRole('button', { name: t('automations.conflict.compare'), exact: true }).click();
    const dialog = page.getByRole('dialog').filter({ has: page.locator('.change-review') });
    await expect(dialog.locator('.change-review__card')).toHaveCount(2);
    for (const key of ['jsonView.draftChanges', 'jsonView.hostChanges']) {
      await expect(dialog).toContainText(t(key));
    }
    // Both diffs expand inline inside the same dialog; no nested source dialog.
    const chevrons = dialog.locator('.change-review__chevron');
    await chevrons.nth(0).click();
    await chevrons.nth(1).click();
    await dialog.getByRole('radio', { name: t('changeReview.code'), exact: true }).click();
    await expect(dialog.locator('.base-json-diff').first()).toBeVisible();
    await expect(dialog).toContainText('Host changed name');
    await expect(page.locator('.review-source-dialog')).toHaveCount(0);
    expect(await dialog.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    for (const button of await dialog.locator('.base-dialog__footer .base-button__label').all()) {
      expect(await button.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    }
    await page.screenshot({ path: info.outputPath(`owner-automation-review-${locale}.png`) });
    await dialog.getByRole('button', { name: t('automations.conflict.useBase'), exact: true }).click();
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
    await page.getByRole('button', { name: t('automations.conflict.compare'), exact: true }).click();
    await expect(dialog).toContainText(t('automations.conflict.recreateLead'));
    await expect(dialog.locator('.change-review__card--deleted')).toHaveCount(1);
    await dialog.getByRole('button', { name: t('automations.conflict.prepareRecreate'), exact: true }).click();
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
