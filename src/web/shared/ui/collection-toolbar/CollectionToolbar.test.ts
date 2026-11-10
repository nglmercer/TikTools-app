import { expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CollectionToolbar from './CollectionToolbar.vue';
test('groups collection controls without replacing native keyboard navigation', () => {
  const wrapper = mountWithPlugins(CollectionToolbar, { props:{ label:'Filter collection', count:'3 items' }, slots:{ search:'<input aria-label="Search">', default:'<select aria-label="Status"><option>All</option></select>', actions:'<button>Refresh</button>' } });
  expect(wrapper.attributes('role')).toBe('group');
  expect(wrapper.attributes('aria-label')).toBe('Filter collection');
  expect(wrapper.text()).toContain('3 items');
  for (const control of wrapper.findAll('input,select,button')) expect(control.attributes('tabindex')).toBeUndefined();
});
