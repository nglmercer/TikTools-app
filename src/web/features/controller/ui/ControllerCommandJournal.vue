<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { ControllerModel } from '../model/controller.ts';
import { useControllerLabels } from '../lib/use-controller-labels.ts';
const props = defineProps<{ browserMode?: boolean; model: Pick<ControllerModel, 'status' | 'busy' | 'commandSending' | 'canResolve' | 'canReview' | 'resolveCommand'> }>();
const { status, busy, commandSending } = props.model;
const { t } = useI18n();
const { cmd } = useControllerLabels();
const emit = defineEmits<{ reviewStorage: []; reviewCommand: [] }>();
</script>

<template>
  <section
    v-if="status.commandJournal && (status.commandBusy || status.commandJournal.pending || status.commandJournal.unavailable || status.commandJournal.recent.length)"
    class="controller__panel controller__journal"
    :aria-label="cmd.journal"
  >
    <h2 class="controller__section-title">
      {{ cmd.journal }}
    </h2>
    <code class="controller__code">{{ status.commandJournal.binding.origin }}</code>
    <code class="controller__code">Host {{ status.commandJournal.binding.hostId }}</code>
    <p
      v-if="status.commandJournal.unavailable"
      class="controller__text"
      role="alert"
    >
      {{ cmd.unavailable }}
    </p>
    <button
      v-if="props.browserMode && status.browserJournalMissing"
      class="controller__button"
      :disabled="commandSending || busy"
      @click="emit('reviewStorage')"
    >
      {{ t('controller.root.reviewCommandStorage') }}
    </button>
    <p
      v-if="status.commandBusy"
      class="controller__text"
      role="status"
    >
      {{ cmd.waiting }}
    </p>
    <template v-if="status.commandJournal.pending">
      <p
        v-if="!status.commandBusy"
        class="controller__text"
      >
        {{ cmd.unknown }}
      </p>
      <p class="controller__text">
        {{ cmd[status.commandJournal.pending.operation] }} · {{ status.commandJournal.pending.resourceId }}
      </p>
      <p
        v-if="status.commandJournal.pending.delta !== undefined"
        class="controller__text"
      >
        {{ cmd.requested }}: {{ status.commandJournal.pending.delta }} {{ status.commandJournal.pending.currencyName }}
      </p>
      <p class="controller__text">
        <span v-if="status.commandJournal.pending.expectedRevision">{{ cmd.revision }} {{ status.commandJournal.pending.expectedRevision }} · </span>{{ new Date(status.commandJournal.pending.createdAt).toLocaleString() }}
      </p>
      <p class="controller__text">
        {{ cmd.pendingId }}:
      </p><code class="controller__code">{{ status.commandJournal.pending.commandId }}</code>
      <p
        v-if="status.commandErrorCode"
        class="controller__text"
      >
        {{ status.commandErrorCode }}
      </p>
      <div class="controller__actions">
        <button
          class="controller__button"
          :disabled="!model.canResolve.value"
          @click="model.resolveCommand()"
        >
          {{ cmd.check }}
        </button>
        <button
          class="controller__button"
          :disabled="!model.canReview.value"
          @click="emit('reviewCommand')"
        >
          {{ cmd.review }}
        </button>
      </div>
    </template>
    <template v-if="status.commandJournal.recent[0]">
      <p
        class="controller__text"
        aria-live="polite"
      >
        {{ cmd.outcome[status.commandJournal.recent[0].outcome] }} · {{ status.commandJournal.recent[0].intent.resourceId }}
      </p>
      <details>
        <summary class="controller__summary">
          {{ cmd.history }}
        </summary>
        <article
          v-for="record in status.commandJournal.recent"
          :key="record.intent.commandId"
        >
          <p class="controller__text">
            {{ cmd[record.intent.operation] }} · {{ record.intent.resourceId }} · {{ cmd.outcome[record.outcome] }}
          </p>
          <code class="controller__code">{{ record.intent.commandId }}</code>
          <p
            v-if="record.intent.delta !== undefined"
            class="controller__text"
          >
            {{ cmd.requested }}: {{ record.intent.delta }} {{ record.intent.currencyName }}
          </p>
          <p
            v-if="record.pointsResult"
            class="controller__text"
          >
            {{ cmd.effective }}: {{ record.pointsResult.delta }} {{ record.intent.currencyName }} · {{ cmd.balance }}: {{ record.pointsResult.totalPoints }} {{ record.intent.currencyName }}
          </p>
          <p
            v-if="record.revision"
            class="controller__text"
          >
            {{ cmd.revision }} {{ record.revision }}
          </p>
          <p
            v-if="record.errorCode"
            class="controller__text"
          >
            {{ record.errorCode }}
          </p>
        </article>
      </details>
    </template>
  </section>
</template>
