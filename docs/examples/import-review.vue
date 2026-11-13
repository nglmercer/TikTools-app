<script setup lang="ts">
defineOptions({ name: "ImportReviewExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseImportDialog from '@/shared/ui/import-dialog/BaseImportDialog.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const open = ref(false); const staged = ref<Array<{ key: string; label: string }>>([]); const errors = ref<string[]>([]); const done = ref(false);
const tab = ref('manual');
function content(text: string) { try { const value: unknown = JSON.parse(text); if (!value || typeof value !== 'object') throw new Error(); staged.value = [{ key: 'demo', label: 'Demo' }]; errors.value = []; } catch { errors.value = [t.value.error]; } }
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseButton :disabled="disabled" @click="open = true">{{ t.open }}</BaseButton><BaseImportDialog v-model:open="open" v-model:tab="tab" :title="t.name" :tabs="[{ value: 'manual', label: t.open }]" :staged="staged" :errors="errors" :busy="props.state === 'loading'" :success="done ? t.ready : ''" :max-bytes="4096" @content="content" @import="done = true" /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
