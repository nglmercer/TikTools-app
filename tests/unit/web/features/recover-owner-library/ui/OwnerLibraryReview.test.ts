import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../../../src/automation/behavior/types.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import type { LibraryMutation } from '@/entities/rule-template/model/library-contracts.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import OwnerLibraryReview from '../../../../../../src/web/features/recover-owner-library/ui/OwnerLibraryReview.vue';
import { ACTION_TYPE, EVENT_KIND, RPC } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach); beforeEach(() => activateLocale('en')); afterEach(() => activateLocale('en'));
function fixture() {
  const source: BehaviorSnapshot = { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === RPC.appStateGet) return { state: {} };
    if (method === RPC.automationSnapshot) {
      options?.onMetadata?.({ resourceVersions: [...source.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: '1' })), ...source.events.map((r) => ({ kind: 'automation', scope: 'event', id: r.id, revision: '1' }))] }); return structuredClone(source);
    }
    if (method === RPC.automationCreate) {
      const record = params?.record as LiveAction | LiveEvent;
      if (params?.kind === 'action') source.actions.push(structuredClone(record as LiveAction)); else source.events.push(structuredClone(record as LiveEvent));
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: record.id, revision: '1' }] }); return record;
    }
    throw new Error(`Unexpected business operation ${method}`);
  });
  const fake = createFakeControlClient(native.handle); const journal = libraryJournal(fake.client);
  const review = mountWithPlugins(OwnerLibraryReview, { attachTo: document.body, global: { provide: { [controlClientKey as symbol]: fake.client } } });
  return { ...fake, native, journal, review };
}

test('a lost graph reply retains its reviewed source and current comparison without resubmitting', async () => {
  const f = fixture(); await flushPromises(); await f.journal.refreshSnapshot();
  const mutation: LibraryMutation = { operation: 'apply-template', profileId: 'default', entry: {
    actions: [{ schemaVersion: 2, id: 'act-frozen', name: 'Frozen action', enabled: false, typeId: ACTION_TYPE.coreLog, config: { message: 'private-source' } }],
    event: { schemaVersion: 1, id: 'evt-frozen', name: 'Frozen event', enabled: false, trigger: EVENT_KIND.tiktokChat, filters: [], actionIds: ['act-frozen'], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' },
  } };
  const storing = vi.spyOn(Storage.prototype, 'setItem');
  try {
    const outcome = f.journal.request(mutation).catch((failure: unknown) => failure); await flushPromises();
    mutation.entry.event.name = 'Newer local configuration';
    const records = f.review.get('[data-test="library-review-records"]');
    expect(records.findAll('.change-review__card')).toHaveLength(2);
    for (const button of records.findAll('.change-review__chevron')) {
      await button.trigger('click'); await flushPromises();
    }
    const codeTab = records.findAll('[role="radio"]').find((radio) => radio.text() === 'Code')!;
    await codeTab.trigger('click'); await flushPromises();
    const sources = records.text();
    expect(sources).toContain('Frozen event');
    expect(sources).toContain('private-source');
    expect(sources).not.toContain('Newer local configuration');
    f.native.library.loseNextReply(); await f.review.get('[data-test="library-confirm"]').trigger('click'); await flushPromises();
    expect(f.review.get('[data-test="library-confirm"]').attributes('disabled')).toBeDefined();
    expect(f.review.text()).toContain('It will not be sent again');
    await f.review.get('[data-test="library-read-current"]').trigger('click'); await flushPromises();
    const currentReview = f.review.get('[data-test="library-review-records"]');
    // The host already applied the frozen plan before the reply was lost, so
    // the re-read rows are unchanged and stay hidden until the eye toggle.
    await currentReview.get('[aria-label="Show unchanged"]').trigger('click'); await flushPromises();
    expect(currentReview.findAll('.change-review__card').length).toBeGreaterThan(0);
    for (const button of currentReview.findAll('.change-review__chevron')) {
      await button.trigger('click'); await flushPromises();
    }
    const foundSource = currentReview.text();
    expect(foundSource).toContain('Frozen event');
    expect(f.calls.filter((call) => call.method === RPC.automationLibrarySubmit)).toHaveLength(1);
    expect(storing).not.toHaveBeenCalled();
    f.journal.dismiss(); await outcome;
  } finally { storing.mockRestore(); }
});

test('membership review names the exact saved action and requested profile', async () => {
  const f = fixture(); await flushPromises();
  const action: LiveAction = { schemaVersion: 2, id: 'saved-target', name: 'Original saved action', enabled: false, typeId: ACTION_TYPE.coreLog, config: {} };
  await f.native.handle(RPC.automationCreate, { kind: 'action', record: action });
  f.native.library.setCatalog({ packs: [{ id: 'requested-profile', name: 'Original selected profile', description: '', eventIds: [], actionIds: [] }], activeId: 'default', templates: [] });
  await f.journal.refreshSnapshot();
  const outcome = f.journal.request({ operation: 'assign-record', kind: 'action', id: action.id, profileId: 'requested-profile' }).catch((failure: unknown) => failure);
  await flushPromises();
  const records = f.review.get('[data-test="library-review-records"]');
  expect(records.findAll('.change-review__card')).toHaveLength(1);
  expect(records.get('strong').text()).toBe('Original saved action');
  expect(f.review.text()).toContain('Original selected profile');
  expect(f.journal.draft.value?.mutation).toMatchObject({ kind: 'action', id: 'saved-target', profileId: 'requested-profile' });
  expect(f.calls.some((call) => call.method === RPC.automationLibrarySubmit)).toBe(false);
  f.journal.dismiss(); await outcome;
});
