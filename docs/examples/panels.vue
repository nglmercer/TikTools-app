<script setup lang="ts">
defineOptions({ name: "PanelsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const open = ref(false);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BasePanel :title="t.name" :subtitle="t.hint"><BaseButton :disabled="disabled" @click="open = true">{{ t.open }}</BaseButton></BasePanel><BaseDialog v-model:open="open" :title="t.name"><p>{{ t.hint }}</p><template #footer><BaseButton @click="open = false">{{ t.close }}</BaseButton></template></BaseDialog></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
