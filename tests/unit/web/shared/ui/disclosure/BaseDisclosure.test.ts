import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseDisclosure from '../../../../../../src/web/shared/ui/disclosure/BaseDisclosure.vue';

enableAutoUnmount(afterEach);

it('owns its state, seeds it from initiallyOpen and keeps the body in the DOM', async () => {
  const wrapper = mountWithPlugins(BaseDisclosure, {
    props: { title: 'Show changes', initiallyOpen: true }, // fixture title, not a catalog string
    slots: { default: '<p class="body">content</p>' },
  });
  const toggle = wrapper.get('button');
  const region = wrapper.get('[role="region"]');
  expect(toggle.attributes('aria-expanded')).toBe('true');
  expect(toggle.attributes('aria-controls')).toBe(region.attributes('id'));
  expect(region.attributes('aria-labelledby')).toBe(toggle.attributes('id'));
  expect(wrapper.find('.body').exists()).toBe(true);
  await toggle.trigger('click');
  expect(toggle.attributes('aria-expanded')).toBe('false');
  expect((wrapper.get('.base-disclosure__body').element as HTMLElement).style.display).toBe('none');
});

it('honours a controlled open prop and emits update:open', async () => {
  const wrapper = mountWithPlugins(BaseDisclosure, {
    props: { title: 'Source', open: false },
  });
  await wrapper.get('button').trigger('click');
  expect(wrapper.emitted('update:open')!.at(-1)).toEqual([true]);
  expect(wrapper.get('button').attributes('aria-expanded')).toBe('false');
  await wrapper.setProps({ open: true });
  expect(wrapper.get('button').attributes('aria-expanded')).toBe('true');
});

it('supports a rich summary slot, count and the inline variant', () => {
  const wrapper = mountWithPlugins(BaseDisclosure, {
    props: { title: 'Ignored', count: 3, inline: true, toggleClass: 'source-toggle' },
    slots: { summary: '<em class="custom">Review source</em>' },
  });
  expect(wrapper.find('.custom').text()).toBe('Review source');
  expect(wrapper.find('.base-disclosure__count').text()).toBe('3');
  expect(wrapper.classes()).toContain('base-disclosure--inline');
  expect(wrapper.get('button').classes()).toContain('source-toggle');
  expect(wrapper.element.tagName).toBe('SPAN');
});
