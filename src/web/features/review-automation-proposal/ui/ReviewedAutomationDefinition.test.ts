import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import ReviewedAutomationDefinition from './ReviewedAutomationDefinition.vue';

enableAutoUnmount(afterEach);

describe('ReviewedAutomationDefinition', () => {
  it('shows the saved record as highlighted JSON whose pre text is exactly the pretty-printed record', () => {
    const record = { id: 'same', name: 'Gift', enabled: false, filters: [{ field: 'a', value: 1 }], note: null };
    const wrapper = mountWithPlugins(ReviewedAutomationDefinition, {
      props: { title: 'Source', definition: { record, provenance: null }, revision: '3' },
    });
    expect(wrapper.findAll('pre')).toHaveLength(1);
    expect(wrapper.get('pre').element.textContent).toBe(JSON.stringify(record, null, 2));
    expect(wrapper.find('.json-token--key').exists()).toBe(true);
  });

  it('shows no JSON when the record is absent', () => {
    const wrapper = mountWithPlugins(ReviewedAutomationDefinition, {
      props: { title: 'Source', definition: { record: null, provenance: null }, revision: '3' },
    });
    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.text()).toContain('Source');
  });
});
