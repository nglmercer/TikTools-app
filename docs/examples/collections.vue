<script setup lang="ts">
defineOptions({ name: "CollectionsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';
import type { TableSort } from '@/shared/ui/table/table-types.ts';
const selected = ref<string | null>(null);
const sort = ref<TableSort | null>(null);
const rows = computed(() => {
  if (props.state === 'empty') return [];
  const values = [3, 1, 2].map((score) => ({ id: String(score), name: `${props.state === 'long' ? t.value.long : t.value.item} ${score}`, score }));
  const current = sort.value;
  if (current) values.sort((a, b) => (current.key === 'score' ? a.score - b.score : a.name.localeCompare(b.name, locale.value)) * (current.direction === 'asc' ? 1 : -1));
  return values;
});
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseTable v-model:selected="selected" v-model:sort="sort" :columns="[{ key: 'name', label: t.name, sortable: true }, { key: 'score', label: '#', sortable: true }]" :rows="rows" :row-key="row => row.id" selectable /><BaseList v-model:selected="selected" :items="rows" :item-key="row => row.id"><template #default="{ item }">{{ item.name }}</template><template #empty>{{ t.empty }}</template></BaseList><output aria-live="polite">{{ selected ?? '—' }}</output></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
