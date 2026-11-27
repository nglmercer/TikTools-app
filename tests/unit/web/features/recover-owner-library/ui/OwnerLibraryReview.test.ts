import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { App } from 'vue';
import type { BehaviorSnapshot, LiveAction } from '../../../../../../src/automation/behavior/types.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import type { LibraryMutation } from '@/entities/rule-template/model/library-contracts.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createRequest, isLiveAction } from '../../../../support/automation.ts';
import OwnerLibraryReview from '../../../../../../src/web/features/recover-owner-library/ui/OwnerLibraryReview.vue';
import { ACTION_TYPE, EVENT_KIND, FIXTURE_ID, LIBRARY_OPERATION, RPC, defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
beforeEach(() => activateLocale('en'));
afterEach(() => activateLocale('en'));
function fixture() {
  const source: BehaviorSnapshot = { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === RPC.appStateGet) return { state: {} };
    if (method === RPC.automationSnapshot) {
      options?.onMetadata?.({
        resourceVersions: [
          ...source.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: '1' })),
          ...source.events.map((r) => ({ kind: 'automation', scope: 'event', id: r.id, revision: '1' })),
        ],
      });
      return structuredClone(source);
    }
    if (method === RPC.automationCreate) {
      const { kind, record } = createRequest(params);
      if (isLiveAction(record)) source.actions.push(structuredClone(record));
      else source.events.push(structuredClone(record));
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: kind, id: record.id, revision: '1' }] });
      return record;
    }
    throw new Error(`Unexpected business operation ${method}`);
  });
  const fake = createFakeControlClient(native.handle);
  const journal = libraryJournal(fake.client);
  const provideClient = { install: (app: App) => void app.provide(controlClientKey, fake.client) };
  const review = mountWithPlugins(OwnerLibraryReview, { attachTo: document.body, global: { plugins: [provideClient] } });
  return { ...fake, native, journal, review };
}

test('a lost graph reply retains its reviewed source and current comparison without resubmitting', async () => {
  const f = fixture();
  await flushPromises();
  await f.journal.refreshSnapshot();
  // Arrange: a template mutation frozen at request time, so later local edits must not reach the review
  const mutation: LibraryMutation = { operation: LIBRARY_OPERATION.applyTemplate, profileId: 'default', entry: {
    actions: [
      { schemaVersion: 2, id: FIXTURE_ID.actFrozen, name: 'Frozen action', enabled: false, typeId: ACTION_TYPE.coreLog, config: { message: 'private-source' } },
    ],
    event: {
      schemaVersion: 1, id: 'evt-frozen', name: 'Frozen event', enabled: false, trigger: EVENT_KIND.tiktokChat,
      filters: [], actionIds: [FIXTURE_ID.actFrozen], cooldownMs: 0, cooldownScope: 'global', runMode: 'all',
    },
  } };
  const storing = vi.spyOn(Storage.prototype, 'setItem');
  try {
    const outcome = f.journal.request(mutation).catch((failure: unknown) => failure);
    await flushPromises();
    mutation.entry.event.name = 'Newer local configuration';
    const records = f.review.get('[data-test="library-review-records"]');
    expect(records.findAll('.change-review__card')).toHaveLength(2);
    for (const button of records.findAll('.change-review__chevron')) {
      await button.trigger('click');
      await flushPromises();
    }
    const codeTab = defined(records.findAll('[role="radio"]').find((radio) => radio.text() === 'Code'));
    await codeTab.trigger('click');
    await flushPromises();
    // Assert: the review shows the frozen source, never the configuration edited after the request
    const sources = records.text();
    expect(sources).toContain('Frozen event');
    expect(sources).toContain('private-source');
    expect(sources).not.toContain('Newer local configuration');
    // Act: the host commits but its reply is lost, so the review must block a resend instead of retrying
    f.native.library.loseNextReply();
    await f.review.get('[data-test="library-confirm"]').trigger('click');
    await flushPromises();
    expect(f.review.get('[data-test="library-confirm"]').attributes('disabled')).toBeDefined();
    expect(f.review.text()).toContain('It will not be sent again');
    await f.review.get('[data-test="library-read-current"]').trigger('click');
    await flushPromises();
    const currentReview = f.review.get('[data-test="library-review-records"]');
    // The host already applied the frozen plan before the reply was lost, so
    // the re-read rows are unchanged and stay hidden until the eye toggle.
    await currentReview.get('[aria-label="Show unchanged"]').trigger('click');
    await flushPromises();
    expect(currentReview.findAll('.change-review__card').length).toBeGreaterThan(0);
    for (const button of currentReview.findAll('.change-review__chevron')) {
      await button.trigger('click');
      await flushPromises();
    }
    const foundSource = currentReview.text();
    expect(foundSource).toContain('Frozen event');
    // Assert: exactly one submission reached the host, and nothing was written to browser storage
    expect(f.calls.filter((call) => call.method === RPC.automationLibrarySubmit)).toHaveLength(1);
    expect(storing).not.toHaveBeenCalled();
    f.journal.dismiss();
    await outcome;
  } finally { storing.mockRestore(); }
});

test('membership review names the exact saved action and requested profile', async () => {
  const f = fixture();
  await flushPromises();
  const action: LiveAction = { schemaVersion: 2, id: 'saved-target', name: 'Original saved action', enabled: false, typeId: ACTION_TYPE.coreLog, config: {} };
  await f.native.handle(RPC.automationCreate, { kind: 'action', record: action });
  f.native.library.setCatalog({
    packs: [{ id: 'requested-profile', name: 'Original selected profile', description: '', eventIds: [], actionIds: [] }],
    activeId: 'default',
    templates: [],
  });
  await f.journal.refreshSnapshot();
  const outcome = f.journal
    .request({ operation: LIBRARY_OPERATION.assignRecord, kind: 'action', id: action.id, profileId: 'requested-profile' })
    .catch((failure: unknown) => failure);
  await flushPromises();
  const records = f.review.get('[data-test="library-review-records"]');
  expect(records.findAll('.change-review__card')).toHaveLength(1);
  expect(records.get('strong').text()).toBe('Original saved action');
  expect(f.review.text()).toContain('Original selected profile');
  expect(f.journal.draft.value?.mutation).toMatchObject({ kind: 'action', id: 'saved-target', profileId: 'requested-profile' });
  expect(f.calls.some((call) => call.method === RPC.automationLibrarySubmit)).toBe(false);
  f.journal.dismiss();
  await outcome;
});
