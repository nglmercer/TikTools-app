<script setup lang="ts">
defineOptions({ name: "JsonExample" });
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
const before = { title: 'Demo', enabled: false }; const after = { title: 'Demo', enabled: true };
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseJsonView :value="before" :label="t.name" /><BaseJsonDiff :before="before" :after="after" :label="t.name" switchable /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
