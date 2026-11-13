<script setup lang="ts">
defineOptions({ name: "MenusExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const open = ref(false); const anchor = ref<HTMLElement | null>(null); const result = ref('');
</script>
<template>
  <div class="component-example" :data-state="props.state"><span ref="anchor"><BaseButton :disabled="disabled" @click="open = !open">{{ t.action }}</BaseButton></span><BaseMenu v-model:open="open" :anchor="anchor" :items="[{ id: 'save', label: t.save }, { id: 'disabled', label: t.disabled, disabled: true }]" @select="result = $event.id" /><output>{{ result }}</output></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
