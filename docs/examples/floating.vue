<script setup lang="ts">
defineOptions({ name: "FloatingExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseFloatingPanel from '@/shared/ui/floating-panel/BaseFloatingPanel.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const open = ref(false);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseButton :disabled="disabled" @click="open = true">{{ t.open }}</BaseButton><BaseFloatingPanel v-model:open="open" :title="t.name" :move-label="t.move" :close-label="t.close" :minimize-label="t.minimize" :restore-label="t.open" :reset-label="t.reset"><p>{{ t.hint }}</p></BaseFloatingPanel></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
