<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import { useSyncChanges } from '../model/use-sync-changes.ts';
import SyncPanel from './SyncPanel.vue';

const props = withDefaults(defineProps<{ adapter: SyncAdapter; size?: 'sm' | 'md' }>(), { size: 'md' });
const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();
const capabilities = useCapabilityStore();
const open = ref(false);

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
  if (sync.status?.enabled !== true) return t('syncChanges.syncOff');
  return null;
});
const usable = computed(() => reason.value === null);
const { count } = useSyncChanges(props.adapter, usable);
const label = computed(() => (count.value > 0 ? t('syncChanges.buttonCount', { count: count.value }) : t('syncChanges.button')));
</script>

<template>
  <span class="sync-button">
    <BaseIconButton
      :label="label"
      :hint="reason ?? undefined"
      variant="default"
      :size="size"
      :disabled="!usable"
      @click="open = true"
    >
      <IconGitMerge />
    </BaseIconButton>
    <span
      v-if="usable && count > 0"
      class="sync-button__badge"
      aria-hidden="true"
    >{{ count > 99 ? '99+' : count }}</span>
    <SyncPanel
      v-if="usable"
      v-model:open="open"
      :adapter="adapter"
    />
  </span>
</template>

<style scoped>
.sync-button {
  position: relative;
  display: inline-flex;
}

.sync-button__badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 14px;
  height: 14px;
  padding: 0 3px;
  border-radius: 7px;
  background: var(--color-accent);
  color: var(--color-text-on-accent);
  font-size: 10px;
  line-height: 14px;
  text-align: center;
  pointer-events: none;
}
</style>
