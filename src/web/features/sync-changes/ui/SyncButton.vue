<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import { useSyncChanges } from '../model/use-sync-changes.ts';
import SyncSetup from './SyncSetup.vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import SyncPanel from './SyncPanel.vue';

const props = withDefaults(defineProps<{ adapter: SyncAdapter; size?: 'sm' | 'md' }>(), { size: 'md' });
const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();
const capabilities = useCapabilityStore();
const open = ref(false);
const preparing = ref(false);
const client = useControlClient();
async function show(): Promise<void> {
  if (props.adapter.requiresCloud === false) { open.value = true; return; }
  await account.ensureRestored();
  await sync.refresh();
  try {
    preparing.value = sync.status?.enabled !== true || await client.call('sync.v2.selected', {}) !== true;
  } catch { preparing.value = true; }
  open.value = true;
}

// The account and opt-in state are read lazily elsewhere; read them here so the button reflects them.
onMounted(() => {
  if (props.adapter.requiresCloud === false) return;
  void account.ensureRestored();
  if (sync.status === null) void sync.refresh();
});

/** Why the panel cannot open, or null when it can. Sections that do not need Cloud are never blocked. */
const reason = computed<string | null>(() => {
  if (props.adapter.requiresCloud === false) return null;
  const own = props.adapter.unavailableReason?.();
  if (own) return own;
  if (capabilities.supports('sync.status') === false) return t('syncChanges.unsupported');
  if (account.status?.state !== 'signed-in') return t('syncChanges.signedOut');
  return null;
});
const usable = computed(() => reason.value === null);
const { count } = useSyncChanges(props.adapter, usable);
const label = computed(() => (count.value > 0 ? t('syncChanges.buttonCount', { count: count.value }) : t('syncChanges.button')));
</script>

<template>
  <span class="sync-button">
    <BaseButton
      data-guide="changes"
      :aria-label="label"
      :title="reason ?? undefined"
      :aria-description="reason ?? undefined"
      variant="default"
      :size="size"
      :disabled="!usable"
      @click="show"
    >
      {{ count > 0 ? t('syncChanges.changesCount', { count }) : t('syncChanges.changes') }}
    </BaseButton>
    <SyncPanel
      v-if="usable"
      v-model:open="open"
      :adapter="adapter"
      :preparing="preparing"
    >
      <template #preparation>
        <SyncSetup
          v-if="open && preparing"
          @ready="preparing = false"
        />
      </template>
    </SyncPanel>
  </span>
</template>

<style scoped>
.sync-button {
  position: relative;
  display: inline-flex;
}

</style>
