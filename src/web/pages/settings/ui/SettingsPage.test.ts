import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { activateLocale, i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import SettingsPage from './SettingsPage.vue';
import { collaborationStatus, emptyCollaborationPage } from '@/shared/lib/testing/collaboration.ts';

enableAutoUnmount(afterEach);

async function mountPage(query = '') {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/settings', component: defineComponent({ render: () => null }) }],
  });
  await router.push(`/settings${query}`);
  const { client } = createFakeControlClient(method => method === 'collaboration.status' ? collaborationStatus() : method.startsWith('collaboration.') ? emptyCollaborationPage(method) : ({ globals: { commandPort: '46665' } }));
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
    expect(wrapper.get('[role="option"][aria-selected="true"]').text()).toBe('GeneralLanguage and application');
    expect(wrapper.get('legend').text()).toBe('Language');
    expect(wrapper.get('#settings-language').text()).toContain('English');
    expect(wrapper.find('#app-update-automatic[role=switch]').exists()).toBe(true);
    expect(wrapper.get('#app-update-automatic').attributes('aria-label')).toBeTruthy();
  });

  it('uses a complete mobile section selector and preserves route navigation', async () => {
    const { wrapper, router } = await mountPage();
    const select = wrapper.get('.settings-page__mobile-nav select');
    await select.setValue('collaboration'); await flushPromises();
    expect(router.currentRoute.value.query.section).toBe('collaboration');
    expect(wrapper.get('h1').text()).toBe('Collaboration');
    expect(select.findAll('option').map(option => option.text())).toContain('Plugins');
  });

  it('lists the simplified sections', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.findAll('[role="option"]').map((option) => option.text())).toEqual([
      'GeneralLanguage and application',
      'PluginsSources and updates',
      'Account & CloudSession and synchronization',
      'CollaborationDevices and access',
    ]);
  });

  it('switches the section through the route query', async () => {
    const { wrapper, router } = await mountPage();
    await wrapper.findAll('[role="option"]').find((item) => item.text().startsWith('Plugins'))!.trigger('pointerdown');
    await flushPromises();
    expect(router.currentRoute.value.query.section).toBe('plugins');
    expect(wrapper.findAll('legend').map((legend) => legend.text())).toContain('Plugin updates');
    expect(wrapper.find('#plugin-sources-url').exists()).toBe(true);
    expect(wrapper.find('details, summary').exists()).toBe(false);
    expect(wrapper.findAll('input[type=checkbox]').every(input => input.attributes('role') === 'switch')).toBe(true);
  });

  it('preserves Advanced links to plugin sources, with registry creation collapsed', async () => {
    const { wrapper } = await mountPage('?section=advanced');
    expect(wrapper.find('#plugin-sources-url').isVisible()).toBe(false);
    expect(wrapper.text()).toContain('Add a registry');
  });

  it('opens runtime variables through legacy globals links', async () => {
    const { wrapper } = await mountPage('?section=globals');
    expect(wrapper.get('[role="option"][aria-selected="true"]').text()).toBe('GeneralLanguage and application');
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
