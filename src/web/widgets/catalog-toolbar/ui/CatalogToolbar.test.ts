import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { DEFAULT_CATALOG_FILTERS, type CatalogFilters } from '@/entities/catalog/model/catalog-query.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CatalogToolbar from './CatalogToolbar.vue';

function mount(filters: CatalogFilters = DEFAULT_CATALOG_FILTERS, signedIn = true) {
  return mountWithPlugins(CatalogToolbar, {
    props: { modelValue: filters, signedIn },
    attachTo: document.body,
  });
}

describe('CatalogToolbar', () => {
  it('starts with search and closed disclosures only', () => {
    const wrapper = mount();
    expect(wrapper.findAll('details[open]')).toHaveLength(0);
    expect(wrapper.find('summary').exists()).toBe(true);
    wrapper.unmount();
  });

  it('reports filter changes as a complete value', async () => {
    const wrapper = mount();
    const radios = wrapper.findAll('input[name="catalog-type"]');
    await radios[1]!.setValue(true);
    const emitted = wrapper.emitted('update:modelValue')!;
    expect(emitted.at(-1)![0]).toEqual({ ...DEFAULT_CATALOG_FILTERS, type: 'plugin' });
    wrapper.unmount();
  });

  it('shows the active filter count and resets to defaults', async () => {
    const wrapper = mount({ ...DEFAULT_CATALOG_FILTERS, view: 'installed', compatibleOnly: false });
    expect(wrapper.find('.catalog-toolbar__badge').text()).toBe('2');
    await wrapper.find('.catalog-toolbar__reset').trigger('click');
    expect(wrapper.emitted('update:modelValue')!.at(-1)![0]).toEqual({ ...DEFAULT_CATALOG_FILTERS, text: '' });
    wrapper.unmount();
  });

  it('offers sign-in only when Mine is selected without a session', async () => {
    expect(mount({ ...DEFAULT_CATALOG_FILTERS, view: 'mine' }, false).find('.catalog-toolbar__signin').exists()).toBe(true);
    expect(mount({ ...DEFAULT_CATALOG_FILTERS, view: 'mine' }, true).find('.catalog-toolbar__signin').exists()).toBe(false);
    expect(mount(DEFAULT_CATALOG_FILTERS, false).find('.catalog-toolbar__signin').exists()).toBe(false);
  });

  it('closes an open disclosure on Escape and on outside clicks', async () => {
    const wrapper = mount();
    const disclosure = wrapper.find('details').element as HTMLDetailsElement;
    disclosure.setAttribute('open', '');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await flushPromises();
    expect(disclosure.hasAttribute('open')).toBe(false);
    disclosure.setAttribute('open', '');
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    await flushPromises();
    expect(disclosure.hasAttribute('open')).toBe(false);
    wrapper.unmount();
  });
});
