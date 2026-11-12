import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defineComponent } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { activateLocale, i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import SettingsPage from './SettingsPage.vue';
import { collaborationStatus, emptyCollaborationPage } from '@/shared/lib/testing/collaboration.ts';

enableAutoUnmount(afterEach);

/** Every Cloud feature the host can announce; none of them may surface inside Settings except teams. */
const announced = ['teams.hosts.list', 'integrations.connections', 'bundles.packages.preflight', 'bundles.overlays.create', 'catalog.items'];

async function mountPage(query = '', announce: string[] = []) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/settings', component: defineComponent({ render: () => null }) }],
  });
  await router.push(`/settings${query}`);
  const { client } = createFakeControlClient((method) => {
    if (method === 'rpc.discover') return { methods: announce.map((name) => ({ name })) };
    if (method === 'collaboration.status') return collaborationStatus();
    if (method.startsWith('collaboration.')) return emptyCollaborationPage(method);
    return { globals: { commandPort: '46665' } };
  });
  const stores = createStoreHarness(client);
  if (announce.length > 0) await stores.use(() => useCapabilityStore()).refresh('test-host');
  const wrapper = mountWithPlugins(SettingsPage, {
    attachTo: document.body,
    global: { plugins: [router, ...stores.plugins] },
  });
  await flushPromises();
  return { wrapper, router };
}

type Wrapper = Awaited<ReturnType<typeof mountPage>>['wrapper'];
const tabNames = (wrapper: Wrapper) => wrapper.findAll('[role="tab"]').map((tab) => tab.text());
const selectedTab = (wrapper: Wrapper) => wrapper.get('[role="tab"][aria-selected="true"]').text();

describe('SettingsPage', () => {
  it('opens the general section with the language picker and app updates', async () => {
    const { wrapper } = await mountPage();
    expect(selectedTab(wrapper)).toBe('General');
    expect(wrapper.get('legend').text()).toBe('Language');
    expect((wrapper.get('#settings-language').element as HTMLInputElement).value).toBe('English');
    expect(wrapper.find('#app-update-automatic[role=switch]').exists()).toBe(true);
    expect(wrapper.get('#app-update-automatic').attributes('aria-label')).toBeTruthy();
  });

  it('lists only application-level sections as icon and label, without subtitles or a repeated heading', async () => {
    const { wrapper } = await mountPage();
    expect(tabNames(wrapper)).toEqual(['General', 'Account & Cloud', 'Collaboration']);
    expect(wrapper.findAll('[role="tab"] svg')).toHaveLength(3);
    expect(wrapper.find('h1, small, select').exists()).toBe(false);
  });

  it('switches the section through the route query', async () => {
    const { wrapper, router } = await mountPage();
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === 'Collaboration')!.trigger('click');
    await flushPromises();
    expect(router.currentRoute.value.query.section).toBe('collaboration');
    expect(selectedTab(wrapper)).toBe('Collaboration');
    expect(wrapper.findAll('input[type=checkbox]').every((input) => input.attributes('role') === 'switch')).toBe(true);
  });

  it('keeps the account, sync and diagnostics together in Account & Cloud', async () => {
    const { wrapper } = await mountPage('?section=account');
    expect(selectedTab(wrapper)).toBe('Account & Cloud');
    expect(wrapper.findAll('legend').length).toBeGreaterThanOrEqual(2);
    expect(wrapper.text()).toContain('Cloud session details');
  });

  it('no longer hosts the areas that moved to their own pages', async () => {
    for (const section of ['', '?section=account', '?section=collaboration', '?section=plugins', '?section=globals']) {
      const { wrapper } = await mountPage(section, announced);
      expect(wrapper.find('.plugin-update-policy, #plugin-update-policy-channel').exists()).toBe(false);
      expect(wrapper.find('.globals-editor').exists()).toBe(false);
      expect(wrapper.find('.cloud-overlays, .cloud-bundles, .cloud-connections').exists()).toBe(false);
      expect(wrapper.text()).not.toContain('Allow local package installation');
      expect(wrapper.text()).not.toContain('Runtime variables');
    }
  });

  it('falls back to General for sections that moved (the router redirects their links)', async () => {
    for (const section of ['plugins', 'advanced', 'globals', 'updates', 'nonsense']) {
      const { wrapper } = await mountPage(`?section=${section}`);
      expect(selectedTab(wrapper)).toBe('General');
    }
  });

  it('offers Teams as a separate section only when the host announces them', async () => {
    const without = await mountPage('?section=collaboration');
    expect(without.wrapper.find('.cloud-teams').exists()).toBe(false);
    const withTeams = await mountPage('?section=collaboration', ['teams.hosts.list']);
    expect(withTeams.wrapper.find('.cloud-teams').exists()).toBe(false);
    await withTeams.wrapper.findAll('[role="tab"]').find(tab => tab.text() === 'Teams')!.trigger('click');
    await flushPromises();
    expect(withTeams.router.currentRoute.value.query.section).toBe('teams');
    expect(withTeams.wrapper.find('.cloud-teams').exists()).toBe(true);
    const account = await mountPage('?section=account', ['teams.hosts.list']);
    expect(account.wrapper.find('.cloud-teams').exists()).toBe(false);
  });

  it('changes the application language', async () => {
    const { wrapper } = await mountPage();
    await wrapper.get('#settings-language').trigger('click');
    await flushPromises();
    Array.from(document.querySelectorAll<HTMLElement>('[role="option"]'))
      .find((option) => option.querySelector('.base-search-select__label')?.textContent?.trim() === 'Español')
      ?.click();
    await vi.waitFor(() => expect(i18n.global.locale.value).toBe('es'));
    expect(localStorage.getItem('tiktok-live-locale')).toBe('es');
    await activateLocale('en');
  });
});
