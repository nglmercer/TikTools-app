import { testMessage } from '@/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { defineComponent, markRaw } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import type { NavigationSection } from '../model/navigation-types.ts';
import AppNavigation from './AppNavigation.vue';

const Empty = defineComponent({ render: () => null });

const sections: NavigationSection[] = [
  {
    id: 'workspace',
    label: 'Workspace',
    items: [
      { id: 'feed', label: t('navigation.feed'), icon: markRaw(IconChat), to: '/feed' },
      { id: 'settings', label: t('navigation.settings'), icon: markRaw(IconGear), to: '/settings' },
    ],
  },
];

describe('AppNavigation', () => {
  it('renders labelled sections and marks the current page', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/feed', component: Empty },
        { path: '/settings', component: Empty },
      ],
    });
    await router.push('/settings');
    const wrapper = mountWithPlugins(AppNavigation, { props: { sections }, global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.get('nav').attributes('aria-label')).toBe('Main navigation');
    expect(wrapper.get('h2').text()).toBe('Workspace');
    const links = wrapper.findAll('a');
    expect(links.map((link) => link.text())).toEqual([t('navigation.feed'), t('navigation.settings')]);
    expect(links[1]!.attributes('aria-current')).toBe('page');
    expect(links[0]!.attributes('aria-current')).toBeUndefined();
  });

  it('collapses to an icon rail with accessible names', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/feed', component: Empty },
        { path: '/settings', component: Empty },
      ],
    });
    await router.push('/feed');
    const wrapper = mountWithPlugins(AppNavigation, { props: { sections, collapsed: true }, global: { plugins: [router] } });
    await flushPromises();
    expect(wrapper.get('nav').classes()).toContain('app-navigation--collapsed');
    expect(wrapper.findAll('.app-navigation__label')).toHaveLength(0);
    const links = wrapper.findAll('a');
    expect(links.map((link) => link.attributes('aria-label'))).toEqual([t('navigation.feed'), t('navigation.settings')]);
  });
});
