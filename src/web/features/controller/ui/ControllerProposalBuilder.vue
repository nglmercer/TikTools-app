<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalConfirmation, ProposalModel } from '../model/proposals.ts';
import type { AutomationModel } from '../model/automations.ts';
import { useProposalBuilderDraft } from '../model/use-proposal-builder-draft.ts';
import ProposalActionFields from './ProposalActionFields.vue';
import ProposalEventFields from './ProposalEventFields.vue';
import ProposalActionSelection from './ProposalActionSelection.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';

const props = defineProps<{ model: ProposalModel; automation?: AutomationModel }>();
const { tm } = useI18n();
const labels = computed(() => tm('controller.proposalBuilder.labels') as MessageSchema['controller']['proposalBuilder']['labels']);

const fields = useProposalBuilderDraft(props);
const { name, actionType, viewerTarget, trigger, cooldown, loadedSource, loadError, draft, loadFields } = fields;

type Capture = { before: ProposalConfirmation; generated: string };
const capture = ref<Capture | null>(null);
const open = computed({ get: () => capture.value !== null, set: (value: boolean) => { if (!value) capture.value = null; } });
const current = computed(() => capture.value !== null && draft.value === capture.value.generated && props.model.current(capture.value.before));
function prepare(): void {
  const before = props.model.prepare('save');
  if (before && draft.value !== null) capture.value = { before, generated: draft.value };
}
function apply(): void {
  if (!capture.value || !current.value) return;
  const generated = capture.value.generated;
  capture.value = null;
  props.model.edit(generated);
  if (loadedSource.value) loadedSource.value = { text: generated, record: JSON.parse(generated) as Record<string, unknown> };
}
watch(() => [props.model.identityKey.value, props.model.kind.value, props.model.id.value], () => { capture.value = null; }, { flush: 'sync' });
</script>

<template>
  <details class="proposal-builder">
    <summary>{{ labels.title }}</summary>
    <p>{{ labels.note }}</p>
    <p>{{ labels.loadFieldsHint }}</p>
    <button
      type="button"
      :disabled="model.busy.value || !model.editable.value"
      @click="loadFields"
    >
      {{ labels.loadFields }}
    </button>
    <p
      v-if="loadError"
      role="alert"
    >
      {{ labels.loadFieldsError }}
    </p>
    <p
      v-if="loadedSource && loadedSource.text !== model.text.value"
      role="status"
    >
      {{ labels.staleFields }}
    </p>
    <div class="fields">
      <label>{{ labels.name }}<input
        v-model="name"
        maxlength="256"
        :disabled="model.busy.value"
      ></label>
      <ProposalActionFields
        v-if="model.kind.value === 'action'"
        :model="model"
        :fields="fields"
      />
      <ProposalEventFields
        v-else
        :model="model"
        :fields="fields"
      />
    </div>
    <ProposalActionSelection
      :model="model"
      :fields="fields"
    />
    <p v-if="model.kind.value === 'event'">
      {{ labels.actionNote }}
    </p>
    <p v-if="model.kind.value === 'event' && trigger === 'tiktok.gift'">
      {{ labels.giftHint }}
    </p>
    <p v-if="model.kind.value === 'event' && trigger === 'custom'">
      {{ labels.customHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.emit'">
      {{ labels.emitHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.fetch'">
      {{ labels.fetchHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'audio.play'">
      {{ labels.audioHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.code'">
      {{ labels.scriptHint }}
    </p>
    <p v-if="model.kind.value === 'event' && cooldown !== '0'">
      {{ labels.cooldownNote }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.points'">
      {{ viewerTarget === 'event' ? labels.dynamicPointsNote : labels.pointsNote }}
    </p>
    <p
      v-if="draft === null"
      role="status"
    >
      {{ labels.invalid }}
    </p>
    <button
      type="button"
      :disabled="draft === null || !model.prepare('save')"
      @click="prepare"
    >
      {{ labels.generate }}
    </button>
  </details>
  <BaseConfirmDialog
    v-model:open="open"
    :title="labels.confirm"
    :message="labels.warning"
    :confirm-label="labels.apply"
    :busy="!current"
    @confirm="apply"
  >
    <template v-if="capture">
      <p>{{ labels.preview }}</p><pre>{{ capture.generated }}</pre>
      <p
        v-if="!current"
        role="status"
      >
        {{ labels.changed }}
      </p>
    </template>
  </BaseConfirmDialog>
</template>

<style scoped src="./proposal-builder.css"></style>
