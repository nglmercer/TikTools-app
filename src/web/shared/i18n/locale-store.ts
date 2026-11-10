import { defineStore } from 'pinia';
import { computed } from 'vue';
import { activateLocale, i18n, type Locale } from './i18n.ts';
import { storeLocale } from './locale-preference.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';

export const useLocaleStore = defineStore('locale', () => {
  const locale = computed<Locale>(() => i18n.global.locale.value);
  const client = useControlClient();

  async function setLocale(next: Locale): Promise<void> {
    if (next === locale.value) return;
    storeLocale(next);
    await activateLocale(next);
    // Best-effort v2 staging: never blocks local locale switching.
    void client.call('sync.v2.stagePreferences', { locale: next }).catch(() => {});
  }

  return { locale, setLocale };
});
