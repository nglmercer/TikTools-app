import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import type { Locale } from '@/shared/i18n/i18n.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { expectElement } from '../../../../support/dom.ts';
import LanguageSelect from '../../../../../../src/web/features/change-language/ui/LanguageSelect.vue';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);
afterEach(() => { document.body.innerHTML = ''; });

// More languages than the app ships today: the picker must be driven by the list, not by what exists now.
const many = ['en', 'es', 'fr', 'pt', 'de', 'it', 'ja', 'ko', 'nl', 'pl', 'tr', 'sv'];

function mountSelect(locales?: readonly string[], modelValue: Locale = 'en') {
  const wrapper = mount(LanguageSelect, {
    attachTo: document.body,
    // The list may name codes the app does not ship, which the Locale-typed prop forbids; attrs reach the prop at runtime.
    attrs: locales ? { locales } : {},
    props: { modelValue, 'onUpdate:modelValue': (value: Locale) => void wrapper.setProps({ modelValue: value }) },
    global: { plugins: [i18n] },
  });
  return wrapper;
}

const optionLabels = () => Array.from(document.querySelectorAll('[role="option"] .base-search-select__label')).map((node) => node.textContent);

describe('LanguageSelect', () => {
  it('shows the current language by its own name', () => {
    expect(mountSelect().get<HTMLInputElement>('input').element.value).toBe('English');
    expect(mountSelect(undefined, 'pt').get<HTMLInputElement>('input').element.value).toBe(t('menu.languages.pt'));
  });

  it('lists every offered locale, with native names even when the catalog has none', async () => {
    const wrapper = mountSelect(many);
    await wrapper.get('input').trigger('click');
    await nextTick();
    expect(optionLabels()).toHaveLength(12);
    expect(optionLabels()).toEqual(expect.arrayContaining(['English', t('menu.languages.es'), 'Français', 'Deutsch', t('menu.languages.ja'), 'Nederlands']));
  });

  it('filters a long list by name or code', async () => {
    const wrapper = mountSelect(many);
    const input = wrapper.get('input');
    await input.setValue('deu');
    await nextTick();
    expect(optionLabels()).toEqual(['Deutsch']);
    await input.setValue('ja');
    await nextTick();
    expect(optionLabels()).toEqual([t('menu.languages.ja')]);
    await input.setValue('zzz');
    await nextTick();
    expect(optionLabels()).toEqual([]);
    expect(document.querySelector('.base-search-select__empty')?.textContent).toContain('No');
  });

  it('selects the picked locale and ignores free text', async () => {
    const wrapper = mountSelect(many);
    await wrapper.get('input').setValue('ita');
    await nextTick();
    expectElement(document, '[role="option"]', HTMLElement).click();
    await nextTick();
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['it']);
  });
});
