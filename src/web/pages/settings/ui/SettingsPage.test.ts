import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { activateLocale, i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import SettingsPage from './SettingsPage.vue';

enableAutoUnmount(afterEach);

async function mountPage(query = '') {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/settings', component: defineComponent({ render: () => null }) }],
  });
  await router.push(`/settings${query}`);
  const { client } = createFakeControlClient(() => ({ globals: { commandPort: '46665' } }));
  const wrapper = mountWithPlugins(SettingsPage, {
    attachTo: document.body,
    global: {
      plugins: [router, ...storePlugins(client)],
    },
  });
  await flushPromises();
  return { wrapper, router };
}

describe('SettingsPage', () => {
  it('opens the general section with the language selector by default', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('[role="option"][aria-selected="true"]').text()).toBe('General');
    expect(wrapper.get('legend').text()).toBe('Language');
    expect(wrapper.get('#settings-language').text()).toContain('English');
  });

  it('lists the simplified sections', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.findAll('[role="option"]').map((option) => option.text())).toEqual([
      'General',
      'Account',
      'Updates',
      'Advanced',
    ]);
  });

  it('switches the section through the route query', async () => {
    const { wrapper, router } = await mountPage();
    await wrapper.findAll('[role="option"]')[3]!.trigger('pointerdown');
    await flushPromises();
    expect(router.currentRoute.value.query.section).toBe('advanced');
    expect(wrapper.findAll('legend').map((legend) => legend.text())).toContain('Runtime globals');
    expect(wrapper.text()).toContain('{{ globals.key }}');
  });

  it('keeps registry setup collapsed under Advanced', async () => {
    const { wrapper } = await mountPage('?section=advanced');
    expect(wrapper.find('#plugin-sources-url').isVisible()).toBe(false);
    expect(wrapper.text()).toContain('Add a registry');
  });

  it('maps legacy section links to Advanced', async () => {
    const { wrapper } = await mountPage('?section=globals');
    expect(wrapper.get('[role="option"][aria-selected="true"]').text()).toBe('Advanced');
    expect(wrapper.findAll('input').map((input) => input.element.value)).toContain('commandPort');
  });

  it('changes the application language', async () => {
    const { wrapper } = await mountPage();
    await wrapper.get('#settings-language').trigger('click');
    document.querySelectorAll<HTMLElement>('[role="option"]').forEach((option) => {
      if (option.textContent?.trim() === 'Español') option.click();
    });
    await vi.waitFor(() => expect(i18n.global.locale.value).toBe('es'));
    expect(localStorage.getItem('tiktok-live-locale')).toBe('es');
    await activateLocale('en');
  });
});
