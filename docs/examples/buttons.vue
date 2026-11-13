<script setup lang="ts">
defineOptions({ name: "ButtonsExample" });
import { ref, computed } from 'vue';
import { exampleText } from './text.ts';
const props = withDefaults(defineProps<{ state?: string; locale?: string }>(), { state: 'normal', locale: 'en' });
const t = computed(() => exampleText(props.locale));
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const count = ref(0);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseButton :disabled="disabled" @click="count++">{{ props.state === 'loading' ? t.loading : props.state === 'long' ? t.long : t.action }} · {{ count }}</BaseButton></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
