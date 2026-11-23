<script setup lang="ts">
import { computed } from 'vue';
import { localeNames, supportedLocales, type Locale } from '@/shared/i18n/supported-locales.ts';

/**
 * Locale picker shared by the documentation site and the browser demo.
 * `links` keeps navigation crawlable in Markdown pages; `select` keeps the
 * compact control used by the demo toolbar. Both render native names from the
 * same locale metadata, so adding a language never needs duplicated copy.
 */
const props = withDefaults(defineProps<{
  locale: Locale;
  locales?: readonly Locale[];
  label: string;
  variant?: 'links' | 'select';
  hrefFor?: (locale: Locale) => string;
}>(), { locales: () => supportedLocales, variant: 'links', hrefFor: undefined });

const emit = defineEmits<{ change: [locale: Locale] }>();

const options = computed(() => props.locales.map((code) => ({ value: code, label: localeNames[code] ?? code })));

function onChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  if (props.locales.includes(value as Locale)) emit('change', value as Locale);
}
</script>

<template>
  <nav
    v-if="variant === 'links'"
    class="language-switcher language-switcher--links"
    :aria-label="label"
  >
    <a
      v-for="option in options"
      :key="option.value"
      :href="hrefFor?.(option.value)"
      :lang="option.value"
      :aria-current="option.value === locale ? 'true' : undefined"
    >{{ option.label }}</a>
  </nav>
  <label
    v-else
    class="language-switcher language-switcher--select"
  >
    <span class="language-switcher__label">{{ label }}</span>
    <select
      :value="locale"
      @change="onChange"
    >
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
      >{{ option.label }}</option>
    </select>
  </label>
</template>

<style scoped>
.language-switcher {
  display: flex;
  align-items: center;
  gap: 8px 14px;
  min-width: 0;
  font-size: inherit;
  color: inherit;
}

.language-switcher--links a {
  color: inherit;
  text-decoration: underline;
  text-underline-offset: 4px;
  white-space: nowrap;
}

.language-switcher--links a[aria-current='true'] {
  font-weight: 600;
  text-decoration: none;
}

.language-switcher__label {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.language-switcher--select select {
  width: 100%;
  min-height: 30px;
  padding: 4px 8px;
  border: 1px solid var(--vp-c-divider, var(--color-border, currentColor));
  border-radius: 5px;
  background: var(--vp-c-bg, transparent);
  color: inherit;
  font: inherit;
  cursor: pointer;
}
</style>
