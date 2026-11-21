<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';

/** Compact link to the docs/tutorial that explains a review dialog. The
 * documentation lives outside the UI; it replaces long inline explanations. */
const props = withDefaults(defineProps<{
  path?: string;
  anchor?: string;
  label?: string;
  /** Open the in-app tutorial catalog instead of the external documentation. */
  tutorial?: boolean;
}>(), {
  path: 'developer/components/import-review.html',
  anchor: 'import-and-review',
  label: '',
  tutorial: false,
});
const emit = defineEmits<{ openTutorial: [] }>();

const { locale, t } = useI18n();
const href = computed(() => new URL(`../${locale.value}/${props.path}#${props.anchor}`, location.href).href);
const text = computed(() => props.label || String(t('jsonView.reviewTable.docs')));
</script>

<template>
  <button
    v-if="tutorial"
    type="button"
    class="review-docs-link"
    @click="emit('openTutorial')"
  >
    <IconInfo
      :size="14"
      aria-hidden="true"
    />
    <span>{{ text }}</span>
  </button>
  <a
    v-else
    class="review-docs-link"
    :href="href"
    target="_blank"
    rel="noopener noreferrer"
  >
    <IconInfo
      :size="14"
      aria-hidden="true"
    />
    <span>{{ text }}</span>
    <span aria-hidden="true">↗</span>
  </a>
</template>

<style scoped>
.review-docs-link {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 0;
  border: 0;
  background: none;
  color: var(--color-text-muted);
  font: inherit;
  font-size: var(--font-size-sm);
  text-decoration: none;
  cursor: pointer;
}
.review-docs-link:hover {
  color: var(--color-text);
  text-decoration: underline;
}
.review-docs-link:focus-visible {
  box-shadow: var(--focus-ring);
}
</style>
