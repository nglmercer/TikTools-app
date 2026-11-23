import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { localeNames } from '@/shared/i18n/supported-locales.ts';
import LanguageSwitcher from './LanguageSwitcher.vue';

enableAutoUnmount(afterEach);

describe('LanguageSwitcher', () => {
  it('links to every locale with native names and marks the current one', () => {
    const wrapper = mount(LanguageSwitcher, {
      props: {
        locale: 'es',
        label: 'Cambiar idioma',
        hrefFor: (locale) => `/${locale}/user/`,
      },
    });
    const links = wrapper.findAll('a');
    expect(links.map((link) => link.text())).toEqual(Object.values(localeNames));
    expect(links.map((link) => link.attributes('href'))).toEqual(Object.keys(localeNames).map((locale) => `/${locale}/user/`));
    expect(links.filter((link) => link.attributes('aria-current') === 'true').map((link) => link.text())).toEqual(['Español']);
    expect(wrapper.get('nav').attributes('aria-label')).toBe('Cambiar idioma');
  });

  it('emits only offered locales from its select variant', async () => {
    const wrapper = mount(LanguageSwitcher, {
      props: { locale: 'en', label: 'Language', variant: 'select', locales: ['en', 'ja'] },
    });
    const select = wrapper.get('select');
    expect(select.findAll('option').map((option) => option.text())).toEqual(['English', '日本語']);
    await select.setValue('ja');
    expect(wrapper.emitted('change')?.at(-1)).toEqual(['ja']);
    await select.setValue('fr');
    expect(wrapper.emitted('change')).toHaveLength(1);
  });
});
