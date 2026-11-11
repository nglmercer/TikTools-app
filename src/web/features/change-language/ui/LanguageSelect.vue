<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { supportedLocales, type Locale } from '@/shared/i18n/i18n.ts';
import { localeLabel } from '@/shared/i18n/locale-label.ts';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';

const props = withDefaults(defineProps<{
  /** Locales on offer; every locale the app ships by default. */
  locales?: readonly Locale[];
}>(), { locales: () => supportedLocales });

const model = defineModel<Locale>({ required: true });
const { t, locale } = useI18n();

// Searchable by native name or code, so the list keeps working with any number of languages.
const options = computed(() => {
  void locale.value;
  return props.locales.map((code) => ({ value: code, label: localeLabel(code), detail: code }));
});

function pick(value: string): void {
  // Free text never leaves the picker: only a listed locale is a valid answer.
  if (props.locales.includes(value as Locale)) model.value = value as Locale;
}
</script>

<template>
  <BaseSearchSelect
    :model-value="model"
    :options="options"
    :placeholder="t('settings.language.search')"
    @update:model-value="pick"
  />
</template>
