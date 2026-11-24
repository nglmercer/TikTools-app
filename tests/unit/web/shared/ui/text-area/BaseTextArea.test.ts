import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseTextArea from '../../../../../../src/web/shared/ui/text-area/BaseTextArea.vue';

describe('BaseTextArea', () => {
  it('keeps label and accessibility attributes on the editable control', () => {
    const wrapper = mountWithPlugins(BaseTextArea, {
      attrs: { id: 'content', class: 'wide', 'aria-describedby': 'hint', name: 'content' },
    });
    expect(wrapper.get('textarea').attributes()).toMatchObject({
      id: 'content', 'aria-describedby': 'hint', name: 'content',
    });
    expect(wrapper.attributes('id')).toBeUndefined();
    expect(wrapper.classes()).toContain('wide');
  });
});
