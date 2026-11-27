import { computed, ref } from 'vue';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { testMessage } from '@/shared/i18n/test-message.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { V2ConflictReview, V2ConflictTarget } from '../../../../../../src/web/features/cloud-sync/model/recovery-contracts.ts';
import type { SyncV2Conflicts } from '../../../../../../src/web/features/cloud-sync/model/use-sync-v2-conflicts.ts';
import SyncV2ConflictReview from '../../../../../../src/web/features/cloud-sync/ui/SyncV2ConflictReview.vue';

enableAutoUnmount(afterEach);

const t = (key: string): string => testMessage('en', key);

function model(targets: V2ConflictTarget[] = []): SyncV2Conflicts {
  return {
    v2Conflicts: ref(targets),
    v2ConflictCount: computed(() => targets.length),
    v2ConflictReview: ref<V2ConflictReview | null>(null),
    v2ConflictBusy: ref(false),
    v2ConflictError: ref(''),
    v2ConflictResolved: ref(false),
    refreshV2Conflicts: async () => {},
    reviewV2Conflict: async () => {},
    resolveV2Conflict: async () => {},
    v2ConflictValueText: () => '',
    reset: () => {},
  };
}

test('the manual conflict queue only appears with pending conflicts and shows their count', () => {
  const empty = mountWithPlugins(SyncV2ConflictReview, { props: { model: model() } });
  expect(empty.find('section[aria-label]').exists()).toBe(false);

  const wrapper = mountWithPlugins(SyncV2ConflictReview, {
    props: { model: model([{ collection: 'plugins', key: 'alpha' }, { collection: 'variables', key: 'beta' }]) },
  });
  const section = wrapper.get('section');
  expect(section.attributes('aria-label')).toBe(t('sync.v2.conflictTitle'));
  expect(wrapper.get('[data-testid="v2-conflict-count"]').text()).toBe('2');
  expect(section.text()).toContain('plugins/alpha');
  expect(section.text()).toContain('variables/beta');
});
