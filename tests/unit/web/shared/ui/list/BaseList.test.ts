import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import BaseList from '../../../../../../src/web/shared/ui/list/BaseList.vue';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);

interface Scene {
  id: string;
  name: string;
}

const scenes: Scene[] = [
  { id: 'intro', name: 'Intro' },
  { id: 'live', name: 'Live' },
  { id: 'outro', name: 'Outro' },
];

function mountList(items: Scene[] = scenes) {
  const wrapper = mount(BaseList<Scene, string>, {
    attachTo: document.body,
    props: {
      items,
      itemKey: (scene: Scene) => scene.id,
      selected: null,
      'onUpdate:selected': (value: string | null) => wrapper.setProps({ selected: value }),
    },
    slots: {
      default: '<template #default="{ item }">{{ item.name }}</template>',
      empty: 'No scenes',
    },
  });
  return wrapper;
}

describe('BaseList', () => {
  it('selects with the pointer and the keyboard', async () => {
    const wrapper = mountList();
    const options = wrapper.findAll('[role="option"]');
    await defined(options[1]).trigger('pointerdown');
    expect(wrapper.props('selected')).toBe('live');
    await wrapper.get('[role="listbox"]').trigger('keydown', { key: 'ArrowDown' });
    expect(wrapper.props('selected')).toBe('outro');
    await wrapper.get('[role="listbox"]').trigger('keydown', { key: 'ArrowDown' });
    expect(wrapper.props('selected')).toBe('outro');
    expect(wrapper.get('[role="listbox"]').attributes('aria-activedescendant')).toBe(defined(options[2]).attributes('id'));
  });

  it('selects on right click before a context menu opens', async () => {
    const wrapper = mountList();
    await defined(wrapper.findAll('[role="option"]')[2]).trigger('contextmenu');
    expect(wrapper.props('selected')).toBe('outro');
  });

  it('activates on Enter and double click', async () => {
    const wrapper = mountList();
    const listbox = wrapper.get('[role="listbox"]');
    await listbox.trigger('keydown', { key: 'Home' });
    await listbox.trigger('keydown', { key: 'Enter' });
    await defined(wrapper.findAll('[role="option"]')[1]).trigger('dblclick');
    expect(wrapper.emitted('activate')).toEqual([[scenes[0]], [scenes[1]]]);
  });

  it('renders the empty slot without items', () => {
    expect(mountList([]).text()).toBe('No scenes');
  });
});
