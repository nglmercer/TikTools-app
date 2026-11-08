import { expect, test } from 'vitest';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { BUILTIN_RULE_TEMPLATES } from '../lib/rule-templates.ts';
import { libraryJournal } from './library-journal.ts';
import { useRuleTemplatesStore } from './rule-templates-store.ts';

test('template gallery imports freeze collision IDs across uncertainty and never fall back to app-state writes', async () => {
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'app.state.get') return { state: {} };
    if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
    throw new Error(`Unexpected legacy mutation ${method}`);
  });
  const fake = createFakeControlClient(native.handle); const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore()); const templates = harness.use(() => useRuleTemplatesStore());
  await automation.refresh();
  const journal = libraryJournal(fake.client); const documents = [structuredClone(BUILTIN_RULE_TEMPLATES[0]!)];
  native.library.loseNextReply();
  const importing = templates.importTemplates(documents); const original = journal.draft.value?.mutation;
  expect(original?.operation).toBe('import-templates');
  const id = original?.operation === 'import-templates' ? original.templates[0]!.id : '';
  expect(id).not.toBe(documents[0]!.id);
  await journal.confirm(); journal.dismiss(); expect(await importing).toBe(0);
  const retry = templates.importTemplates(documents);
  expect(journal.draft.value?.mutation).toMatchObject({ operation: 'import-templates', templates: [{ id }] });
  journal.dismiss(); expect(await retry).toBe(0);
  expect(fake.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
  expect(fake.calls.some((call) => call.method === 'app.state.set')).toBe(false);
  automation.dispose();
});

test('retained raw template catalogs normalize UI defaults without rewriting source or applying import-size/count caps', async () => {
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
    throw new Error(`Unexpected fallback ${method}`);
  });
  const documents = Array.from({ length: 35 }, (_, index) => ({ templateVersion: 1, id: `retained-${index}`, title: 'Retained', actions: [{ name: 'Log', typeId: 'core.log', config: { message: index === 0 ? 'x'.repeat(100_000) : 'message' } }], event: { name: 'Chat', trigger: 'tiktok.chat' } }));
  native.library.setCatalog({ packs: [], activeId: 'default', templates: documents as never });
  const fake = createFakeControlClient(native.handle); const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore()); const templates = harness.use(() => useRuleTemplatesStore());
  await automation.refresh();
  expect(templates.custom).toHaveLength(35);
  expect(templates.custom[0]?.actions[0]?.enabled).toBe(true);
  expect(templates.custom[0]?.event).toMatchObject({ enabled: true, filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' });
  expect(templates.custom[0]?.actions[0]?.config.message).toHaveLength(100_000);
  expect(libraryJournal(fake.client).snapshot.value?.catalog.templates).toEqual(documents);
  expect(fake.calls.some((call) => call.method === 'app.state.set' || call.method === 'automation.library.submit')).toBe(false);
  automation.dispose();
});
