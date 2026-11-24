import { testMessage } from '@/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { ReviewRow } from '../../../../../../src/web/features/review-shared/model/review-rows.ts';
import ReviewComparisonTable from '../../../../../../src/web/features/review-shared/ui/ReviewComparisonTable.vue';

enableAutoUnmount(afterEach);

const rows: ReviewRow[] = [
  { id: 'one', name: 'First record', kind: 'action', change: 'add', after: { name: 'First record' } },
  { id: 'two', name: 'Second record', kind: 'event', change: 'change', before: { name: 'Old' }, after: { name: 'New' }, editable: true },
];

function mountTable() {
  return mountWithPlugins(ReviewComparisonTable, {
    attachTo: document.body,
    props: { rows, docsPath: 'user/cloud/changes.html', docsAnchor: 'review-changes-by-area', nameLabel: 'Record name' },
  });
}

test('renders a numbered table with a docs link and no inline source column', () => {
  const wrapper = mountTable();
  const bodyRows = wrapper.findAll('tbody tr');
  expect(bodyRows).toHaveLength(2);
  expect(bodyRows.map((row) => row.get('td').text())).toEqual(['1', '2']);
  expect(wrapper.get('table').findAll('th').map((th) => th.text())).toEqual([
    t('jsonView.reviewTable.index'), t('jsonView.reviewTable.record'), t('jsonView.reviewTable.type'),
    t('jsonView.reviewTable.change'), t('jsonView.reviewTable.actions'),
  ]);
  expect(wrapper.text()).not.toContain(testMessage('en', 'libraryRecovery.source'));
  expect(wrapper.findAll('.base-json-diff').length).toBe(0);
  const link = wrapper.get('a.review-docs-link');
  expect(link.attributes('href')).toContain('user/cloud/changes.html#review-changes-by-area');
  expect(link.attributes('target')).toBe('_blank');
});

test('opens the full diff in a separate dialog instead of squeezing it into the table', async () => {
  const wrapper = mountTable();
  const action = wrapper.findAll('.review-table__action')[1]!;
  expect(wrapper.findAll('.base-json-diff')).toHaveLength(0);
  await action.trigger('click');
  const dialog = wrapper.get('.review-source-dialog');
  expect(dialog.text()).toContain('New');
  expect(dialog.findAll('.base-json-diff')).toHaveLength(1);
  expect(dialog.find('button[aria-label="Unified"]').exists()).toBe(true);
});

test('emits a rename for editable rows', async () => {
  const wrapper = mountTable();
  await wrapper.get('input').setValue('Renamed');
  expect(wrapper.emitted('rename')?.at(-1)).toEqual(['two', 'Renamed']);
});
