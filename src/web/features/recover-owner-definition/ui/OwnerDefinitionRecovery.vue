<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { definitionJournal } from '@/entities/automation/model/definition-journal.ts';
import type { DefinitionState } from '@/entities/automation/model/definition-contracts.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';

const props = defineProps<{ selectionKey: string }>();
const emit = defineEmits<{ recovered: []; observed: [state: DefinitionState | null] }>();
const { t } = useI18n();
const journal = definitionJournal(useControlClient());
const open = ref(false);
const acknowledged = ref(false);
const busy = computed(() => journal.busy.value || journal.gate.busy.value);
const pending = computed(() => journal.state.value?.pending);
const hasPending = computed(() => !!pending.value);
const consentKey = computed(() => JSON.stringify([props.selectionKey, journal.state.value]));
watch(consentKey, () => { acknowledged.value = false; }, { flush: 'sync' });
watch(open, () => { acknowledged.value = false; });
watch(journal.gate.owner, (owner) => { if (owner && owner !== 'definition') acknowledged.value = false; }, { flush: 'sync' });
watch(journal.recoveryEpoch, () => emit('recovered'));
watch(journal.state, (state) => emit('observed', state), { immediate: true });
onMounted(() => { void journal.refresh(); });
function showHistory(): void { open.value = true; acknowledged.value = false; void journal.refresh(); }
defineExpose({ openHistory: showHistory, busy, hasPending });
async function archiveUnknown(): Promise<void> {
  const current = pending.value;
  if (!current || !acknowledged.value) return;
  await journal.reviewUnknown(current.commandId, () => acknowledged.value);
  acknowledged.value = false;
}
</script>

<template>
  <BaseButton
    size="sm"
    :disabled="busy"
    data-test="definition-history"
    @click="showHistory"
  >
    {{ t(pending ? 'definitionRecovery.pendingButton' : 'definitionRecovery.history') }}
  </BaseButton>
  <BaseDialog
    v-model:open="open"
    :title="t('definitionRecovery.title')"
    :width="900"
    class="definition-recovery"
  >
    <p>{{ t('definitionRecovery.lead') }}</p>
    <p
      v-if="journal.state.value"
      class="definition-recovery__identity"
    >
      {{ t('definitionRecovery.host', { id: journal.state.value.hostId, instance: journal.state.value.hostInstanceId }) }}
    </p>
    <p
      v-if="journal.unavailable.value"
      role="alert"
    >
      {{ t('definitionRecovery.unavailable') }}
    </p>
    <p
      v-if="journal.error.value"
      role="alert"
    >
      {{ journal.error.value }}
    </p>
    <section
      v-if="pending"
      data-test="definition-pending"
    >
      <p>{{ t('definitionRecovery.pending', { operation: t(`definitionRecovery.${pending.operation}`), kind: t(`definitionRecovery.${pending.kind}`), id: pending.id, revision: pending.expectedRevision }) }}</p>
      <p class="definition-recovery__identity">
        {{ t('definitionRecovery.command', { id: pending.commandId }) }}
      </p>
      <p class="definition-recovery__identity">
        {{ t('definitionRecovery.originalInstance', { instance: pending.hostInstanceId }) }}
      </p>
      <BaseButton
        :disabled="busy"
        data-test="definition-resolve"
        @click="journal.resolve"
      >
        {{ t('definitionRecovery.resolve') }}
      </BaseButton>
      <BaseCheckbox
        v-model="acknowledged"
        :disabled="busy"
        data-test="definition-unknown-consent"
      >
        {{ t('definitionRecovery.unknown') }}
      </BaseCheckbox>
      <BaseButton
        :disabled="busy || !acknowledged"
        data-test="definition-archive"
        @click="archiveUnknown"
      >
        {{ t('definitionRecovery.archive') }}
      </BaseButton>
    </section>
    <p v-else>
      {{ t('definitionRecovery.noPending') }}
    </p>
    <p>{{ t('definitionRecovery.compare') }}</p>
    <BaseButton
      :disabled="busy"
      @click="journal.refresh"
    >
      {{ t('definitionRecovery.refresh') }}
    </BaseButton>
    <ol
      v-if="journal.state.value?.recent.length"
      class="definition-recovery__history"
    >
      <li
        v-for="record in journal.state.value.recent"
        :key="record.intent.commandId"
        data-test="definition-receipt"
      >
        <p>{{ t(`definitionRecovery.${record.intent.operation}`) }} · {{ t(`definitionRecovery.${record.intent.kind}`) }} {{ record.intent.id }} · {{ t(`definitionRecovery.${record.outcome}`) }}</p>
        <p class="definition-recovery__identity">
          {{ t('definitionRecovery.command', { id: record.intent.commandId }) }}
        </p>
        <p>{{ t('definitionRecovery.base', { revision: record.intent.expectedRevision }) }}</p>
        <p class="definition-recovery__identity">
          {{ t('definitionRecovery.originalInstance', { instance: record.intent.hostInstanceId }) }}
        </p>
        <p v-if="record.receipt">
          {{ t('definitionRecovery.revision', { revision: record.receipt.revision, count: record.receipt.affectedResources }) }}
        </p>
        <p v-if="record.errorCode">
          {{ record.errorCode }}
        </p>
      </li>
    </ol>
    <p v-else>
      {{ t('definitionRecovery.empty') }}
    </p>
  </BaseDialog>
</template>

<style scoped>
.definition-recovery :deep(.base-dialog__body) { display: grid; gap: 12px; min-width: 0; }
.definition-recovery section { display: grid; gap: 10px; min-width: 0; }
.definition-recovery__identity, .definition-recovery__history p { overflow-wrap: anywhere; }
.definition-recovery__history { display: grid; gap: 14px; padding-inline-start: 22px; }
</style>
