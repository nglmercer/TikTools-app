import { afterEach, describe, expect, it } from 'vitest';
import { defineComponent, ref } from 'vue';
import { enableAutoUnmount } from '@vue/test-utils';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import SettingsDisclosure from './SettingsDisclosure.vue';

enableAutoUnmount(afterEach);

describe('SettingsDisclosure', () => {
  it('links its button and region and preserves drafts when collapsed', async () => {
    const Draft = defineComponent({ setup: () => ({ value: ref('8') }), template: '<input aria-label="Limit" v-model="value">' });
    const wrapper = mountWithPlugins(SettingsDisclosure, { attachTo: document.body, props: { title: 'Network limits' }, slots: { default: Draft } });
    const button = wrapper.get('button'); const region = wrapper.get('[role=region]');
    expect(button.attributes('type')).toBe('button');
    expect(button.attributes('aria-controls')).toBe(region.attributes('id'));
    expect(region.attributes('aria-labelledby')).toBe(button.attributes('id'));
    expect(button.attributes('aria-expanded')).toBe('false');
    await button.trigger('click');
    const input = wrapper.get('input'); await input.setValue('16');
    await button.trigger('click'); expect(input.isVisible()).toBe(false);
    await button.trigger('click'); expect((input.element as HTMLInputElement).value).toBe('16');
    expect(button.attributes('aria-expanded')).toBe('true');
  });
  it('starts expanded when permission review must be visible', () => {
    const wrapper = mountWithPlugins(SettingsDisclosure, { attachTo: document.body, props: { title: 'Permissions', initiallyOpen: true } });
    expect(wrapper.get('button').attributes('aria-expanded')).toBe('true');
  });
});
