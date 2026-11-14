<script setup lang="ts">
defineOptions({ name: "TabsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import BaseToolbar from '@/shared/ui/toolbar/BaseToolbar.vue';
const tab = ref('one');
// Drafts belong to this parent even when an editor is unmounted on tab change.
const drafts = ref<Record<string, string>>({ one: '', two: '' });
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseTabs v-model="tab" :label="t.name" :tabs="[{ value: 'one', label: t.item }, { value: 'two', label: t.long, disabled }]"><BaseToolbar><label :for="`draft-${tab}`">{{ t.name }} {{ tab }}</label></BaseToolbar><textarea :id="`draft-${tab}`" :key="tab" v-model="drafts[tab]" :disabled="disabled" /></BaseTabs></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
