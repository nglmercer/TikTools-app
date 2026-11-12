<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalModel } from '../model/proposals.ts';
import type { ProposalBuilderDraft } from '../model/use-proposal-builder-draft.ts';
const props = defineProps<{
  model: Pick<ProposalModel, 'busy' | 'editable' | 'kind'>;
  fields: Pick<ProposalBuilderDraft,
    'actionSearch' | 'actionId' | 'selectedActionIds' | 'actionsValid' | 'actionsReady' | 'matchingActions' | 'addAction' | 'removeAction' | 'moveAction' | 'useAction'>;
}>();
const { t, tm } = useI18n();
const labels = computed(() => tm('controller.proposalBuilder.labels') as MessageSchema['controller']['proposalBuilder']['labels']);
const { actionSearch, actionId, selectedActionIds, actionsValid, actionsReady, matchingActions, addAction, removeAction, moveAction, useAction } = props.fields;
</script>

<template>
  <div
    v-if="model.kind.value === 'event'"
    class="selected-actions"
  >
    <button
      type="button"
      :disabled="model.busy.value || !model.editable.value || !actionId || !actionsValid"
      @click="addAction"
    >
      {{ labels.addAction }}
    </button>
    <p
      v-if="!actionsValid"
      role="status"
    >
      {{ labels.actionError }}
    </p>
    <template v-if="selectedActionIds.length">
      <h4>{{ labels.selectedActions }}</h4>
      <ol>
        <li
          v-for="(id, index) in selectedActionIds"
          :key="id"
        >
          <span>{{ id }}</span>
          <button
            type="button"
            :aria-label="t('controller.proposalBuilder.labels.moveEarlier', { id })"
            :disabled="model.busy.value || !model.editable.value || index === 0"
            @click="moveAction(index, -1)"
          >
            ↑
          </button>
          <button
            type="button"
            :aria-label="t('controller.proposalBuilder.labels.moveLater', { id })"
            :disabled="model.busy.value || !model.editable.value || index === selectedActionIds.length - 1"
            @click="moveAction(index, 1)"
          >
            ↓
          </button>
          <button
            type="button"
            :aria-label="t('controller.proposalBuilder.labels.removeAction', { id })"
            :disabled="model.busy.value || !model.editable.value"
            @click="removeAction(index)"
          >
            ×
          </button>
        </li>
      </ol>
    </template>
  </div>
  <section
    v-if="model.kind.value === 'event' && actionsReady"
    :aria-label="labels.availableActions"
    class="available-actions"
  >
    <h4>{{ labels.availableActions }}</h4>
    <p>{{ labels.actionStatusNote }}</p>
    <label>{{ labels.actionSearch }}<input
      v-model="actionSearch"
      type="search"
      maxlength="128"
      autocomplete="off"
      :disabled="model.busy.value"
    ></label>
    <p v-if="!matchingActions.length">
      {{ labels.noActions }}
    </p>
    <ul v-else>
      <li
        v-for="row in matchingActions.slice(0, 50)"
        :key="row.summary.id"
      >
        <span>{{ row.summary.name }} · {{ row.summary.id }} · {{ row.summary.enabled ? labels.actionEnabled : labels.actionDisabled }}</span>
        <button
          type="button"
          :disabled="model.busy.value || !model.editable.value"
          @click="useAction(row)"
        >
          {{ labels.selectAction }}
        </button>
      </li>
    </ul>
    <p v-if="matchingActions.length > 50">
      {{ labels.actionLimit }}
    </p>
  </section>
</template>

<style scoped src="./proposal-builder.css"></style>
