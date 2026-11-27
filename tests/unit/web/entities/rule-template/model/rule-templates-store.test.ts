import { expect, test } from 'vitest';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { RuleTemplate } from '../../../../../../src/web/entities/rule-template/lib/rule-templates.ts';
import { libraryJournal } from '../../../../../../src/web/entities/rule-template/model/library-journal.ts';
import { useRuleTemplatesStore } from '../../../../../../src/web/entities/rule-template/model/rule-templates-store.ts';
import { ACTION_TYPE, EVENT_KIND, LIBRARY_OPERATION, RPC } from '../../../../../shared/index.ts';


function hostTemplateDocument(id: string): RuleTemplate {
  return {
    templateVersion: 1,
    id,
    title: { default: id },
    description: { default: '' },
    icon: 'plugin',
    actions: [{ name: 'Log', typeId: ACTION_TYPE.coreLog, enabled: true, config: { message: 'hi' } }],
    event: { name: id, enabled: true, trigger: EVENT_KIND.tiktokChat, filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' },
  };
}

test('template gallery imports freeze collision IDs across uncertainty and never fall back to app-state writes', async () => {
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === RPC.systemInfo) return { features: { persistence: true } };
    if (method === RPC.automationRuns) return { runs: [] };
    if (method === RPC.appStateGet) return { state: {} };
    if (method === RPC.automationSnapshot) {
      options?.onMetadata?.({ resourceVersions: [] });
      return { actions: [], events: [], plugins: [], actionTypes: [], translations: {}, hostTemplates: [hostTemplateDocument('host-gift-log')] };
    }
    throw new Error(`Unexpected legacy mutation ${method}`);
  });
  const fake = createFakeControlClient(native.handle);
  const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore());
  const templates = harness.use(() => useRuleTemplatesStore());
  await automation.refresh();
  const journal = libraryJournal(fake.client);
  const documents = [hostTemplateDocument('host-gift-log')];
  native.library.loseNextReply();
  const importing = templates.importTemplates(documents);
  const original = journal.draft.value?.mutation;
  expect(original?.operation).toBe(LIBRARY_OPERATION.importTemplates);
  const id = original?.operation === LIBRARY_OPERATION.importTemplates ? original.templates[0]!.id : '';
  expect(id).not.toBe(documents[0]!.id);
  await journal.confirm();
  journal.dismiss();
  expect(await importing).toBe(0);
  const retry = templates.importTemplates(documents);
  expect(journal.draft.value?.mutation).toMatchObject({ operation: LIBRARY_OPERATION.importTemplates, templates: [{ id }] });
  journal.dismiss();
  expect(await retry).toBe(0);
  expect(fake.calls.filter((call) => call.method === RPC.automationLibrarySubmit)).toHaveLength(1);
  expect(fake.calls.some((call) => call.method === RPC.appStateSet)).toBe(false);
  automation.dispose();
});

test('retained raw template catalogs normalize UI defaults without rewriting source or applying import-size/count caps', async () => {
  const documents = Array.from({ length: 35 }, (_, index) => ({
    templateVersion: 1,
    id: `retained-${index}`,
    title: 'Retained',
    actions: [{ name: 'Log', typeId: ACTION_TYPE.coreLog, config: { message: index === 0 ? 'x'.repeat(100_000) : 'message' } }],
    event: { name: 'Chat', trigger: EVENT_KIND.tiktokChat },
  }));
  // The host keeps the catalog as JSON text in app state, so the raw documents reach the store exactly as stored
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === RPC.systemInfo) return { features: { persistence: true } };
    if (method === RPC.automationRuns) return { runs: [] };
    if (method === RPC.appStateGet) return { state: { [RPC.behaviorTemplatesCustom]: JSON.stringify(documents) } };
    if (method === RPC.automationSnapshot) {
      options?.onMetadata?.({ resourceVersions: [] });
      return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
    }
    throw new Error(`Unexpected fallback ${method}`);
  });
  const fake = createFakeControlClient(native.handle);
  const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore());
  const templates = harness.use(() => useRuleTemplatesStore());
  await automation.refresh();
  expect(templates.custom).toHaveLength(35);
  expect(templates.custom[0]?.actions[0]?.enabled).toBe(true);
  expect(templates.custom[0]?.event).toMatchObject({ enabled: true, filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' });
  expect(templates.custom[0]?.actions[0]?.config.message).toHaveLength(100_000);
  expect(libraryJournal(fake.client).snapshot.value?.catalog.templates).toEqual(documents);
  expect(fake.calls.some((call) => call.method === RPC.appStateSet || call.method === RPC.automationLibrarySubmit)).toBe(false);
  automation.dispose();
});
