import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import BaseTable from '../../../../../../src/web/shared/ui/table/BaseTable.vue';
import { nextSort, type TableSort } from '../../../../../../src/web/shared/ui/table/table-types.ts';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);

interface Viewer {
  id: string;
  name: string;
  points: number;
}

const columns = [
  { key: 'name', label: 'Viewer', sortable: true },
  { key: 'points', label: 'Points', align: 'end' as const, sortable: true, width: '96px' },
];

const rows: Viewer[] = [
  { id: 'a', name: 'Ana', points: 120 },
  { id: 'b', name: 'Beto', points: 80 },
];

function mountTable(props: Record<string, unknown> = {}) {
  const wrapper = mount(BaseTable<Viewer, string>, {
    attachTo: document.body,
    props: {
      columns,
      rows,
      rowKey: (row: Viewer) => row.id,
      selectable: true,
      selected: null,
      sort: null,
      'onUpdate:selected': (value: string | null) => wrapper.setProps({ selected: value }),
      'onUpdate:sort': (value: TableSort | null) => wrapper.setProps({ sort: value }),
      ...props,
    },
    slots: { 'cell-points': '<template #cell-points="{ value }"><b>{{ value }} pts</b></template>' },
  });
  return wrapper;
}

describe('nextSort', () => {
  it('starts ascending and toggles the same column', () => {
    expect(nextSort(null, 'name')).toEqual({ key: 'name', direction: 'asc' });
    expect(nextSort({ key: 'name', direction: 'asc' }, 'name')).toEqual({ key: 'name', direction: 'desc' });
    expect(nextSort({ key: 'name', direction: 'desc' }, 'points')).toEqual({ key: 'points', direction: 'asc' });
  });
});

describe('BaseTable', () => {
  it('renders default and slotted cells', () => {
    const wrapper = mountTable();
    const cells = defined(wrapper.findAll('tbody tr')[0]).findAll('td');
    expect(defined(cells[0]).text()).toBe('Ana');
    expect(defined(cells[1]).html()).toContain('<b>120 pts</b>');
  });

  it('cycles the sort model from the header', async () => {
    const wrapper = mountTable();
    const header = defined(wrapper.findAll('th')[1]);
    expect(header.attributes('aria-sort')).toBe('none');
    await header.get('button').trigger('click');
    expect(wrapper.props('sort')).toEqual({ key: 'points', direction: 'asc' });
    expect(header.attributes('aria-sort')).toBe('ascending');
    await header.get('button').trigger('click');
    expect(header.attributes('aria-sort')).toBe('descending');
  });

  it('selects rows with pointer and keyboard and activates with Enter', async () => {
    const wrapper = mountTable();
    expect(wrapper.get('table').attributes('role')).toBe('grid');
    await defined(wrapper.findAll('tbody tr')[0]).trigger('pointerdown');
    expect(wrapper.props('selected')).toBe('a');
    await wrapper.trigger('keydown', { key: 'ArrowDown' });
    expect(wrapper.props('selected')).toBe('b');
    expect(defined(wrapper.findAll('tbody tr')[1]).attributes('aria-selected')).toBe('true');
    await wrapper.trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('activate')).toEqual([[rows[1]]]);
  });

  it('stays read-only when not selectable', async () => {
    const wrapper = mountTable({ selectable: false });
    await defined(wrapper.findAll('tbody tr')[0]).trigger('pointerdown');
    expect(wrapper.props('selected')).toBeNull();
    expect(wrapper.get('table').attributes('role')).toBeUndefined();
  });
});
