<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useSyncV2 } from '@/features/cloud-sync/model/use-sync-v2.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
const emit = defineEmits<{ ready: [] }>();
const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();
let generation = 0;
const client = useControlClient();
const inventory = ref<{ review: string; plugins: string[] } | null>(null);
const inventoryBusy = ref(false);
const inventoryError = ref('');
const protocol = useSyncV2(client, sync, {
  generation: () => generation,
  available: () => account.status?.state === 'signed-in' && sync.status?.enabled === true,
  refreshUncertain: async () => {},
});
const { v2Selected, v2Busy, v2Error, v2ErrorMessage, migrationPreview, protocolReady } = protocol;
async function prepare(): Promise<void> {
  await sync.setEnabled(true);
  if (sync.status?.enabled !== true) return;
  await protocol.loadProtocol();
  await protocol.checkV2();
  if (protocol.v2State.value === 'backendCompatible') await protocol.previewMigration();
}
function parseInventory(value: unknown): { review: string; plugins: string[] } {
  if (!value || typeof value !== 'object') throw new Error(t('syncChanges.actionFailed'));
  const item = value as { review: unknown; plugins: unknown };
  if (typeof item.review !== 'string' || !/^[a-f0-9]{64}$/.test(item.review) || !Array.isArray(item.plugins) || item.plugins.length > 8192 || !item.plugins.every(id => typeof id === 'string' && /^[a-zA-Z0-9._-]{1,128}$/.test(id))) throw new Error(t('syncChanges.actionFailed'));
  return { review: item.review, plugins: item.plugins as string[] };
}
async function reviewInventory(): Promise<void> {
  if (inventoryBusy.value) return;
  const ticket = generation;
  inventoryBusy.value = true; inventory.value = null; inventoryError.value = '';
  try {
    const result = parseInventory(await client.call('sync.v2.inventory.review', {}));
    if (ticket === generation) inventory.value = result;
  } catch (failure) { if (ticket === generation) inventoryError.value = errorMessage(failure); }
  finally { if (ticket === generation) inventoryBusy.value = false; }
}
async function repairInventory(): Promise<void> {
  if (inventoryBusy.value || !inventory.value?.plugins.length) return;
  const ticket = generation;
  const review = inventory.value.review;
  inventoryBusy.value = true; inventoryError.value = '';
  try {
    parseInventory(await client.call('sync.v2.inventory.repair', { review }));
    if (ticket !== generation) return;
    inventory.value = null;
    protocol.invalidatePreview();
    await protocol.previewMigration();
  } catch (failure) {
    if (ticket === generation) { inventory.value = null; inventoryError.value = errorMessage(failure); }
  } finally { if (ticket === generation) inventoryBusy.value = false; }
}
watch(v2Selected, (selected) => { if (selected) emit('ready'); });
watch(() => account.status, () => { generation++; protocol.reset(); inventory.value = null; inventoryBusy.value = false; inventoryError.value = ''; });
onMounted(async () => { await account.ensureRestored(); await sync.refresh(); });
onUnmounted(() => { generation++; protocol.reset(); });
</script>
<template>
  <div>
    <BaseAlert
      v-if="sync.error || v2Error"
      tone="error"
      :title="v2ErrorMessage || sync.error || t('syncChanges.actionFailed')"
    />
    <BaseButton
      v-if="!migrationPreview"
      :disabled="v2Busy || sync.working || account.status?.state !== 'signed-in'"
      @click="prepare"
    >
      {{ t('syncChanges.prepare') }}
    </BaseButton>
    <template v-if="migrationPreview">
      <p role="status">
        {{ t('sync.v2.previewResult', migrationPreview) }}
      </p>
      <p>{{ t('sync.v2.migrateWarning') }}</p>
      <BaseButton
        :disabled="!protocolReady || v2Busy || !migrationPreview.migrationAvailable"
        @click="protocol.migrate()"
      >
        {{ t('sync.v2.migrate') }}
      </BaseButton>
    </template>
    <section v-if="v2Error || inventory || inventoryError">
      <BaseAlert
        v-if="inventoryError"
        tone="error"
        :title="inventoryError"
      />
      <BaseButton
        :disabled="inventoryBusy || v2Busy || sync.working"
        @click="reviewInventory"
      >
        {{ t('syncChanges.inventoryReview') }}
      </BaseButton>
      <template v-if="inventory">
        <p
          v-if="!inventory.plugins.length"
          role="status"
        >
          {{ t('syncChanges.inventoryEmpty') }}
        </p>
        <template v-else>
          <ul>
            <li
              v-for="id in inventory.plugins"
              :key="id"
            >
              {{ id }}
            </li>
          </ul>
          <p>{{ t('syncChanges.inventoryWarning') }}</p>
          <BaseButton
            :disabled="inventoryBusy || v2Busy || sync.working"
            @click="repairInventory"
          >
            {{ t('syncChanges.inventoryRepair') }}
          </BaseButton>
        </template>
      </template>
    </section>
  </div>
</template>
