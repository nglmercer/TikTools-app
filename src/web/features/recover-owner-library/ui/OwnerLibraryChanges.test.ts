import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import type { LibraryIntent, LibraryMutation } from '@/entities/rule-template/model/library-contracts.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import OwnerLibraryChanges from './OwnerLibraryChanges.vue';

enableAutoUnmount(afterEach); beforeEach(() => activateLocale('en')); afterEach(() => activateLocale('en'));
function fixture() {
  const source: BehaviorSnapshot = { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === 'app.state.get') return { state: {} };
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: [...source.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: '1' })), ...source.events.map((r) => ({ kind: 'automation', scope: 'event', id: r.id, revision: '1' }))] }); return structuredClone(source);
    }
    if (method === 'automation.create') {
      const record = params?.record as LiveAction | LiveEvent;
      if (params?.kind === 'action') source.actions.push(structuredClone(record as LiveAction)); else source.events.push(structuredClone(record as LiveEvent));
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: record.id, revision: '1' }] }); return record;
    }
    throw new Error(`Unexpected business operation ${method}`);
  });
  const fake = createFakeControlClient(native.handle); const journal = libraryJournal(fake.client);
  const wrapper = mountWithPlugins(OwnerLibraryChanges, { props: { selectionKey: 'action:other' }, attachTo: document.body,
    global: { provide: { [controlClientKey as symbol]: fake.client } } });
  return { ...fake, native, journal, wrapper };
}

test('a lost graph reply retains its reviewed source and original IDs, exposes history and current comparison without resubmitting', async () => {
  const f = fixture(); await flushPromises(); await f.journal.refreshSnapshot();
  const mutation: LibraryMutation = { operation: 'apply-template', profileId: 'default', entry: {
    actions: [{ schemaVersion: 2, id: 'act-frozen', name: 'Frozen action', enabled: false, typeId: 'core.log', config: { message: 'private-source' } }],
    event: { schemaVersion: 1, id: 'evt-frozen', name: 'Frozen event', enabled: false, trigger: 'tiktok.chat', filters: [], actionIds: ['act-frozen'], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' },
  } };
  const storing = vi.spyOn(Storage.prototype, 'setItem');
  try {
    const outcome = f.journal.request(mutation).catch((failure: unknown) => failure); await flushPromises();
    mutation.entry.event.name = 'Newer local configuration';
    expect(f.wrapper.get('[data-test="library-review-records"]').text()).toContain('Frozen event');
    expect(f.wrapper.get('[data-test="library-review-records"]').text()).not.toContain('Newer local configuration');
    f.native.library.loseNextReply(); await f.wrapper.get('[data-test="library-confirm"]').trigger('click'); await flushPromises();
    expect(f.wrapper.get('[data-test="library-confirm"]').attributes('disabled')).toBeDefined();
    expect(f.wrapper.text()).toContain('It will not be sent again');
    await f.wrapper.get('[data-test="library-history"]').trigger('click'); await flushPromises();
    const original = f.native.library.state().recent[0]!.intent.commandId;
    expect(f.wrapper.get('[data-test="library-receipt"]').text()).toContain(original);
    await f.wrapper.get('[data-test="library-read-current"]').trigger('click'); await flushPromises();
    expect(f.wrapper.get('[data-test="library-current-review"]').text()).toContain('Frozen event');
    expect(f.wrapper.get('[data-test="library-current-review"]').text()).toContain('private-source');
    expect(f.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
    expect(storing).not.toHaveBeenCalled();
    f.journal.dismiss(); await outcome;
  } finally { storing.mockRestore(); }
});

test('selection-independent Spanish recovery retains exact revisions and resets stale unknown consent', async () => {
  await activateLocale('es'); const f = fixture(); await flushPromises();
  const intent: LibraryIntent = { commandId: `owner-library-${'a'.repeat(32)}`, operation: 'delete-profile', target: 'unselected-profile', expectedRevision: '9007199254740993', definitionDigest: 'b'.repeat(64), commandHash: 'c'.repeat(64), hostInstanceId: 'instance-a', createdAt: 1 };
  f.native.library.setState({ ...f.native.library.state(), pending: intent });
  await f.wrapper.get('[data-test="library-history"]').trigger('click'); await flushPromises();
  expect(f.wrapper.text()).toContain('unselected-profile'); expect(f.wrapper.text()).toContain('No se enviará de nuevo');
  await f.wrapper.get('[data-test="library-unknown-consent"]').setValue(true);
  await f.wrapper.setProps({ selectionKey: 'event:new' });
  expect((f.wrapper.get('[data-test="library-unknown-consent"]').element as HTMLInputElement).checked).toBe(false);
  await f.wrapper.get('[data-test="library-unknown-consent"]').setValue(true);
  await f.wrapper.get('[data-test="library-archive"]').trigger('click'); await flushPromises();
  expect(f.wrapper.get('[data-test="library-receipt"]').text()).toContain('9007199254740993');
  expect(f.native.library.state().pending).toBeNull(); expect(f.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
});

test.each(['en', 'es'] as const)('membership review names the exact saved action and requested profile after selection changes (%s)', async (locale) => {
  await activateLocale(locale); const f = fixture(); await flushPromises();
  const action: LiveAction = { schemaVersion: 2, id: 'saved-target', name: 'Original saved action', enabled: false, typeId: 'core.log', config: {} };
  await f.native.handle('automation.create', { kind: 'action', record: action });
  f.native.library.setCatalog({ packs: [{ id: 'requested-profile', name: 'Original selected profile', description: '', eventIds: [], actionIds: [] }], activeId: 'default', templates: [] });
  await f.journal.refreshSnapshot();
  const outcome = f.journal.request({ operation: 'assign-record', kind: 'action', id: action.id, profileId: 'requested-profile' }).catch((failure: unknown) => failure);
  await f.wrapper.setProps({ selectionKey: 'event:different-selection' }); await flushPromises();
  const records = f.wrapper.get('[data-test="library-review-records"]');
  expect(records.findAll('li')).toHaveLength(1);
  expect(records.get('strong').text()).toBe('Original saved action');
  expect(f.wrapper.text()).toContain('Original selected profile');
  expect(f.journal.draft.value?.mutation).toMatchObject({ kind: 'action', id: 'saved-target', profileId: 'requested-profile' });
  expect(f.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
  f.journal.dismiss(); await outcome;
});
