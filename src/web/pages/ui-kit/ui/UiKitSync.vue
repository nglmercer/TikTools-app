<script setup lang="ts">
import { ref, shallowRef } from 'vue';
import { defineSyncAdapter, type SyncChange, type SyncOperation } from '@/entities/cloud-sync/api/sync-adapter.ts';
import SyncPanel from '@/features/sync-changes/ui/SyncPanel.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';

// An in-memory section that declares every capability, so the whole panel can be seen and tried.
const changes = shallowRef<SyncChange[]>([
  { id: 'rose', kind: 'event', name: 'Rose gift alert', status: 'modified', side: 'local', before: { name: 'Rose gift', volume: 5, color: 'red' }, after: { name: 'Rose gift alert', volume: 5, color: 'red' } },
  { id: 'wave', kind: 'event', name: 'Wave on follow', status: 'added', side: 'local', after: { name: 'Wave on follow', volume: 3 } },
  { id: 'chat', kind: 'event', name: 'Chat hello', status: 'modified', side: 'remote', device: 'Pixel 8', before: { name: 'Chat hello', volume: 5 }, after: { name: 'Chat hello', volume: 8 } },
  {
    id: 'clash', kind: 'event', name: 'Big gifts', status: 'conflict', side: 'conflict',
    base: { name: 'Big gifts', volume: 5, color: 'red' }, before: { name: 'Big gifts', volume: 5, color: 'red' },
    after: { name: 'Huge gifts', volume: 5, color: 'blue' }, remote: { name: 'Big gifts', volume: 9, color: 'red' },
  },
]);
const operations: SyncOperation[] = [
  { id: 'a41c9e', kind: 'push', title: 'Sent 2 events', device: 'This computer', at: Date.now() - 3_600_000, changes: [changes.value[0]!, changes.value[1]!] },
  { id: 'b72d10', kind: 'fetch', title: 'Fetched 1 event', device: 'Pixel 8', at: Date.now() - 86_400_000, changes: [changes.value[2]!] },
];
const auto = ref(false);
const done = async (): Promise<void> => {};
const adapter = defineSyncAdapter({
  scope: { id: 'demo', label: 'Demo' },
  requiresCloud: false,
  kindLabel: (kind) => kind,
  capabilities: { push: true, fetch: true, sync: true, reset: true, resolve: true, merge: true, force: true, history: true, fork: true, autoSync: true },
  status: () => ({ lastSyncedAt: Math.floor(Date.now() / 1000) - 600 }),
  list: async () => changes.value,
  history: async () => operations,
  push: async (ids) => { changes.value = changes.value.filter((change) => !ids.includes(change.id)); },
  resolve: async (id) => { changes.value = changes.value.filter((change) => change.id !== id); },
  reset: async (id) => { changes.value = changes.value.filter((change) => change.id !== id); },
  fetch: done, sync: done, forcePush: done, forcePull: done, restore: done, fork: done,
  autoSync: { get: () => auto.value, set: async (value) => { auto.value = value; } },
});
const open = ref(false);
</script>

<template>
  <section class="ui-kit-sync">
    <header class="ui-kit-sync__header">
      Sync changes panel
    </header>
    <div class="ui-kit-sync__body">
      <BaseButton @click="open = true">
        Open the panel with a demo section
      </BaseButton>
      <SyncPanel
        v-model:open="open"
        :adapter="adapter"
      />
    </div>
  </section>
</template>

<style scoped>
.ui-kit-sync {
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.ui-kit-sync__header {
  padding: var(--space-3) var(--space-4);
  background: var(--color-bg-panel-header);
  font-weight: var(--font-weight-semibold);
}

.ui-kit-sync__body {
  padding: var(--space-4);
}
</style>
