import { expect, test, type Page } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, readHostState, waitForFrontend, writeHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { ACTION_TYPE, EVENT_KIND, ROUTE, RPC, TOPIC } from '../shared/index.ts';
import { SEL } from './support/selectors.ts';

/**
 * The edit notices float over the editor's header actions;
 * close them so those actions are reachable.
 */
async function dismissNotices(page: Page): Promise<void> {
  const toast = page.locator('.automation-edit-notice__toast');
  while (await toast.count() > 0) await toast.first().getByRole('button').click();
}

for (const locale of ['en', 'es'] as const) {
  test(`owner editor retains conflicts and reviews recreation (${locale})`, async ({ page }, info) => {
    if (locale === 'es') await page.setViewportSize({ width: 390, height: 844 });
    const state = emptyState();
    // This spec exercises the opt-in advanced review flow.
    state.appState['app.resolution.mode'] = 'manual';
    state.automationSnapshot = {
      actions: [{ schemaVersion: 2, id: 'same', name: 'Local action', enabled: false, typeId: ACTION_TYPE.coreLog, config: { message: 'fixture only' } }],
      events: [
        {
          schemaVersion: 1, id: 'same', name: 'Starting event', enabled: false, trigger: EVENT_KIND.tiktokChat, filters: [],
          cooldownMs: 0, cooldownScope: 'user', runMode: 'all', actionIds: ['same'],
        },
      ],
      plugins: [], actionTypes: [], eventTypes: [], translations: {},
    };
    const capture = await installFakeHost(page, state, { locale });
    await test.step('open the event editor from the automations library', async () => {
      await page.goto(ROUTE.automations);
      await waitForFrontend(page);
      await page.locator('.automation-library tbody tr').filter({ hasText: 'Starting event' }).click();
    });
    const name = page.locator('.event-editor input[type="text"]').first();
    await test.step('type a draft, then change the same event on the host', async () => {
      await name.fill('My retained draft');
      const host = await readHostState(page);
      (host.automationSnapshot['events'] as Array<Record<string, unknown>>)[0]!['name'] = 'Host changed name';
      host.automationVersions = { 'action:same': '1', 'event:same': '2' };
      await writeHostState(page, host);
      await emitTopic(page, TOPIC.workflowChanged, { kind: 'event', id: 'same', change: 'updated' });
    });
    const t = (key: string): string => testMessage(locale, key);
    await test.step('the conflict notice keeps the local draft', async () => {
      await expect(page.getByText(t('automations.conflict.hostChanged'))).toBeVisible();
      await expect(name).toHaveValue('My retained draft');
      await dismissNotices(page);
    });
    const save = page.getByRole('button', { name: t('automations.editor.save'), exact: true });
    await test.step('the first save fails on the stale revision and opens the comparison', async () => {
      await save.click();
      await expect(save).toBeDisabled();
      const failed = await readHostState(page);
      expect(failed.calls.find((r) => r.method === RPC.automationUpdate)?.meta?.['expectedRevision']).toBe('1');
      await dismissNotices(page);
      await page.getByRole('button', { name: t('automations.conflict.compare'), exact: true }).click();
    });
    const dialog = page.getByRole('dialog').filter({ has: page.locator('.change-review') });
    await test.step('review both copies, use the base copy, then recreate the deleted event', async () => {
      await expect(dialog.locator(SEL.changeReviewCard)).toHaveCount(2);
      for (const key of ['jsonView.draftChanges', 'jsonView.hostChanges']) {
        await expect(dialog).toContainText(t(key));
      }
      // Both diffs expand inline inside the same dialog; no nested source dialog.
      const chevrons = dialog.locator('.change-review__chevron');
      await chevrons.nth(0).click();
      await chevrons.nth(1).click();
      await dialog.getByRole('radio', { name: t('changeReview.code'), exact: true }).click();
      await expect(dialog.locator(SEL.baseJsonDiff).first()).toBeVisible();
      await expect(dialog).toContainText('Host changed name');
      await expect(page.locator('.review-source-dialog')).toHaveCount(0);
      expect(await dialog.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
      for (const button of await dialog.locator('.base-dialog__footer .base-button__label').all()) {
        expect(await button.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
      }
      await page.screenshot({ path: info.outputPath(`owner-automation-review-${locale}.png`) });
      await dialog.getByRole('button', { name: t('automations.conflict.useBase'), exact: true }).click();
      await expect(dialog).not.toBeVisible();
      expect((await readHostState(page)).calls.filter((r) => r.method === RPC.automationUpdate)).toHaveLength(1);
      await expect(name).toHaveValue('My retained draft');
      await dismissNotices(page);
      await save.click();
      await expect(save).toBeDisabled();
      await expect.poll(async () => (await readHostState(page)).automationVersions?.['event:same']).toBe('3');
      const afterSave = await readHostState(page);
      expect(afterSave.calls.filter((r) => r.method === RPC.automationUpdate).at(-1)?.meta?.['expectedRevision']).toBe('2');
      afterSave.automationSnapshot['events'] = [];
      afterSave.automationVersions!['event:same'] = '4';
      await writeHostState(page, afterSave);
      await emitTopic(page, TOPIC.workflowChanged, { kind: 'event', id: 'same', change: 'deleted' });
      await name.fill('Draft after deletion');
      await dismissNotices(page);
      await save.click();
      await expect(save).toBeDisabled();
      await dismissNotices(page);
      await page.getByRole('button', { name: t('automations.conflict.compare'), exact: true }).click();
      await expect(dialog).toContainText(t('automations.conflict.recreateLead'));
      await expect(dialog.locator('.change-review__card--deleted')).toHaveCount(1);
      await dialog.getByRole('button', { name: t('automations.conflict.prepareRecreate'), exact: true }).click();
      expect((await readHostState(page)).calls.some((r) => r.method === RPC.automationCreate)).toBe(false);
      await dismissNotices(page);
      await save.click();
      await expect.poll(async () => (await readHostState(page)).automationVersions?.['event:same']).toBe('5');
      const final = await readHostState(page);
      expect(final.calls.find((r) => r.method === RPC.automationCreate)?.meta?.['expectedRevision']).toBe('4');
      expect(final.automationVersions?.['action:same']).toBe('1');
      expect(final.calls.some((r) => ['automation.fire', 'automation.enable', RPC.liveConnect, RPC.pointsAdjust].includes(r.method))).toBe(false);
      await assertNoUnhandledCalls(page);
      capture.assertClean();
    });
  });
}
