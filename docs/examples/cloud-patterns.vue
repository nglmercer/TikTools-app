<script setup lang="ts">
defineOptions({ name: "CloudPatternsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const applied = ref(false);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BasePanel :title="t.name"><BaseJsonDiff :before="{ greeting: 'Hello' }" :after="{ greeting: 'Welcome' }" switchable /><BaseButton :disabled="disabled || applied" @click="applied = true">{{ t.save }}</BaseButton><output>{{ applied ? t.ready : t.hint }}</output></BasePanel></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
