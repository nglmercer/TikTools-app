<script setup lang="ts">
defineOptions({ name: "LayoutsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
import MasterDetailLayout from '@/shared/ui/layout/MasterDetailLayout.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const detail = ref(false); const size = ref(280);
</script>
<template>
  <div class="component-example" :data-state="props.state"><div style="height: 320px"><MasterDetailLayout v-model:size="size" :narrow="props.state === 'mobile'" :detail-open="detail" :back-label="t.back" :detail-title="t.name" @back="detail = false"><template #master><BaseButton @click="detail = true">{{ t.open }}</BaseButton></template><template #detail><p>{{ props.state === 'long' ? t.long : t.item }}</p></template></MasterDetailLayout></div></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
