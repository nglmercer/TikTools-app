<script setup lang="ts">
defineOptions({ name: "FeedbackExample" });
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseAlert v-if="props.state === 'error'" :title="t.error" /><BaseEmptyState v-else-if="props.state === 'empty'" :title="t.empty" :description="t.hint" /><BaseStatus v-else :label="props.state === 'loading' ? t.loading : t.ready" :tone="props.state === 'loading' ? 'info' : 'success'" /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
