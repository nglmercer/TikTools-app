<script setup lang="ts">
import { computed, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { defineSyncAdapter, type SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import SyncPanel from '@/features/sync-changes/ui/SyncPanel.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import { exampleText } from '../../../../../docs/examples/text.ts';
const { t, locale } = useI18n();
const copy = computed(() => exampleText(locale.value));
// This is a presentational adapter with real memory mutations. It announces
// only implemented operations, without enabling force, merging or auto-sync.
const changes = shallowRef<SyncChange[]>([
  { id: 'local', kind: 'event', name: 'Welcome', status: 'modified', side: 'local', before: { enabled: false }, after: { enabled: true } },
  { id: 'remote', kind: 'event', name: 'Demo chat', status: 'modified', side: 'remote', before: { message: 'Hello' }, after: { message: 'Welcome' } },
  { id: 'conflict', kind: 'event', name: 'Rose', status: 'conflict', side: 'conflict', before: { count: 1 }, after: { count: 2 }, remote: { count: 3 } },
]);
const seed = structuredClone(changes.value);
const adapter = defineSyncAdapter({
  scope: { id: 'component-example', label: 'Demo' }, requiresCloud: false,
  kindLabel: () => t('syncChanges.kinds.event'),
  capabilities: { push: true, fetch: true, apply: true, resolve: true },
  list: async () => changes.value,
  push: async (ids) => { if (ids.some((id) => !changes.value.some((row) => row.id === id))) throw new Error(copy.value.error); changes.value = changes.value.filter((row) => !ids.includes(row.id)); },
  fetch: async () => { if (!changes.value.some((row) => row.id === 'remote')) changes.value = [...changes.value, structuredClone(seed[1]!)]; },
  apply: async (id) => { if (!changes.value.some((row) => row.id === id)) throw new Error(copy.value.error); changes.value = changes.value.filter((row) => row.id !== id); },
  resolve: async (id, resolution) => { if (resolution.choice === 'merged' || !changes.value.some((row) => row.id === id)) throw new Error(copy.value.error); changes.value = changes.value.filter((row) => row.id !== id); },
});
const open = ref(false);
</script>
<template>
  <section class="ui-kit-sync">
    <h2>{{ t('syncChanges.title') }}</h2>
    <BaseButton @click="open = true">
      {{ copy.open }}
    </BaseButton>
    <SyncPanel
      v-model:open="open"
      :adapter="adapter"
    />
  </section>
</template>
<style scoped>
.ui-kit-sync { padding: var(--space-5); border: 1px solid var(--color-border); }
.ui-kit-sync h2 { margin: 0 0 var(--space-4); font-size: var(--font-size-lg); }
</style>
