import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import type { Locale } from '@/shared/i18n/i18n.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import LanguageSelect from './LanguageSelect.vue';

enableAutoUnmount(afterEach);
afterEach(() => { document.body.innerHTML = ''; });

// More languages than the app ships today: the picker must be driven by the list, not by what exists now.
const many = ['en', 'es', 'fr', 'pt', 'de', 'it', 'ja', 'ko', 'nl', 'pl', 'tr', 'sv'] as unknown as readonly Locale[];

function mountSelect(locales?: readonly Locale[], modelValue: Locale = 'en') {
  const wrapper = mount(LanguageSelect, {
    attachTo: document.body,
    props: { modelValue, ...(locales ? { locales } : {}), 'onUpdate:modelValue': (value: Locale) => void wrapper.setProps({ modelValue: value }) },
    global: { plugins: [i18n] },
  });
  return wrapper;
}

const optionLabels = () => Array.from(document.querySelectorAll('[role="option"] .base-search-select__label')).map((node) => node.textContent);

describe('LanguageSelect', () => {
  it('shows the current language by its own name', () => {
    expect((mountSelect().get('input').element as HTMLInputElement).value).toBe('English');
    expect((mountSelect(undefined, 'pt').get('input').element as HTMLInputElement).value).toBe('Português (Brasil)');
  });

  it('lists every offered locale, with native names even when the catalog has none', async () => {
    const wrapper = mountSelect(many);
    await wrapper.get('input').trigger('click');
    await nextTick();
    expect(optionLabels()).toHaveLength(12);
    expect(optionLabels()).toEqual(expect.arrayContaining(['English', 'Español', 'Français', 'Deutsch', '日本語', 'Nederlands']));
  });

  it('filters a long list by name or code', async () => {
    const wrapper = mountSelect(many);
    const input = wrapper.get('input');
    await input.setValue('deu');
    await nextTick();
    expect(optionLabels()).toEqual(['Deutsch']);
    await input.setValue('ja');
    await nextTick();
    expect(optionLabels()).toEqual(['日本語']);
    await input.setValue('zzz');
    await nextTick();
    expect(optionLabels()).toEqual([]);
    expect(document.querySelector('.base-search-select__empty')?.textContent).toContain('No');
  });

  it('selects the picked locale and ignores free text', async () => {
    const wrapper = mountSelect(many);
    await wrapper.get('input').setValue('ita');
    await nextTick();
    (document.querySelector('[role="option"]') as HTMLElement).click();
    await nextTick();
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['it']);
  });
});
