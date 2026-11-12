<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { ControllerModel } from '../model/controller.ts';
import { useControllerLabels } from '../lib/use-controller-labels.ts';
import type { WorkflowOperation } from '../model/controller.ts';
import type { ControllerWorkflowDrafts } from '../model/use-controller-workflow-drafts.ts';
const props = defineProps<{ browserMode?: boolean; model: Pick<ControllerModel, 'status' | 'rows' | 'ready' | 'canChange' | 'draft' | 'draftStorage' | 'edit' | 'editRetainedDraft' | 'discard' | 'rebaseDraft'>; drafts: ControllerWorkflowDrafts }>();
const { status, rows, ready } = props.model;
const { selected, row, retainedDrafts, draftText, conflict, savedDraft, draftMatches, draftsSupported, draftError, draftBusy, draftLoading, draftErrorText } = props.drafts;
const { t } = useI18n();
const { labels, cmd, dft } = useControllerLabels();
const emit = defineEmits<{ change: [operation: WorkflowOperation]; saveDraft: [] }>();
</script>

<template>
  <section
    v-if="ready && status.hello?.capabilities.includes('workflows.read')"
    class="controller__panel"
  >
    <h2 class="controller__section-title">
      {{ labels.workflows }}
    </h2>
    <p class="controller__text">
      {{ props.browserMode ? t('controller.root.availableActions') : labels.readOnly }}
    </p>
    <p
      v-if="!rows.length"
      class="controller__text"
    >
      {{ labels.empty }}
    </p>
    <div class="controller__workflows">
      <nav
        class="controller__workflow-nav"
        :aria-label="labels.workflows"
      >
        <button
          v-for="item in rows"
          :key="item.version.id"
          class="controller__button"
          :aria-pressed="selected === item.version.id"
          @click="selected = item.version.id"
        >
          {{ item.graph['name'] || item.version.id }} · {{ item.version.revision }}
        </button>
        <template v-if="retainedDrafts.length">
          <h3>{{ cmd.retainedDrafts }}</h3>
          <button
            v-for="id in retainedDrafts"
            :key="`draft-${id}`"
            class="controller__button"
            :aria-pressed="selected === id"
            @click="selected = id"
          >
            {{ id }} · {{ labels.draft }}
          </button>
        </template>
      </nav>
      <div v-if="row || retainedDrafts.includes(selected)">
        <div
          v-if="row"
          class="controller__actions"
        >
          <button
            v-if="status.hello?.capabilities.includes('workflows.toggle') && status.hello.features['workflowDisable'] && row.graph['enabled'] === true"
            class="controller__button"
            :disabled="!model.canChange('disable')"
            @click="emit('change', 'disable')"
          >
            {{ cmd.disable }}
          </button>
          <button
            v-if="status.hello?.capabilities.includes('workflows.edit') && status.hello.features['workflowDelete']"
            class="controller__button"
            :disabled="!model.canChange('delete')"
            @click="emit('change', 'delete')"
          >
            {{ cmd.delete }}
          </button>
        </div>
        <h3>{{ props.browserMode ? t('controller.root.workflowDetails') : labels.draft }}</h3>
        <p
          v-if="!props.browserMode"
          class="controller__text"
        >
          {{ row ? labels.keep : cmd.hostUnavailableDraft }}
        </p><p
          v-if="conflict"
          class="controller__text"
          role="status"
        >
          {{ labels.conflict }}
        </p>
        <section
          v-if="conflict && row"
          class="controller__comparison"
        >
          <h4>{{ dft.compare }} · {{ row.version.revision }}</h4>
          <textarea
            class="controller__textarea"
            :value="JSON.stringify(row.graph, null, 2)"
            :aria-label="dft.compare"
            readonly
            spellcheck="false"
          />
          <p class="controller__text">
            {{ dft.base }}: {{ model.draft(selected)?.revision }}
          </p>
          <p class="controller__text">
            {{ dft.merge }}
          </p>
          <button
            class="controller__button"
            :disabled="draftBusy || draftLoading"
            @click="model.rebaseDraft(row)"
          >
            {{ dft.rebase }} · {{ row.version.revision }}
          </button>
        </section>
        <textarea
          class="controller__textarea"
          :value="draftText"
          :aria-label="props.browserMode ? t('controller.root.workflowDetails') : labels.draft"
          spellcheck="false"
          :readonly="props.browserMode"
          :disabled="draftBusy || draftLoading"
          @input="row ? model.edit(row, ($event.target as HTMLTextAreaElement).value) : model.editRetainedDraft(selected, ($event.target as HTMLTextAreaElement).value)"
        /><button
          v-if="model.draft(selected)"
          class="controller__button"
          :disabled="draftBusy || draftLoading"
          @click="model.discard(selected)"
        >
          {{ labels.discard }}
        </button>
        <section
          v-if="draftsSupported"
          class="controller__draftStorage"
        >
          <p
            v-if="draftError"
            class="controller__text"
            role="alert"
          >
            {{ draftErrorText }}
          </p>
          <p class="controller__text">
            {{ dft.limit }}
          </p>
          <button
            class="controller__button"
            :disabled="!model.draftStorage.prepare(selected)"
            @click="emit('saveDraft')"
          >
            {{ dft.save }}
          </button>
          <button
            class="controller__button"
            :disabled="draftBusy || draftLoading"
            @click="model.draftStorage.load(selected, true)"
          >
            {{ dft.reload }}
          </button>
          <p
            v-if="model.draft(selected)"
            class="controller__text"
          >
            {{ draftMatches ? dft.matches : dft.unsaved }}
          </p>
          <details v-if="savedDraft">
            <summary class="controller__summary">
              {{ dft.saved }} · {{ savedDraft.baseRevision }}
            </summary>
            <textarea
              class="controller__textarea"
              :value="savedDraft.text"
              :aria-label="dft.saved"
              readonly
              spellcheck="false"
            />
            <button
              class="controller__button"
              :disabled="draftBusy || draftLoading"
              @click="model.draftStorage.remove(selected, false)"
            >
              {{ dft.remove }}
            </button>
          </details>
        </section>
      </div>
    </div>
    <details>
      <summary class="controller__summary">
        {{ props.browserMode ? t('controller.root.availableHere') : labels.rights }}
      </summary><ul>
        <li
          v-for="capability in status.hello?.capabilities"
          :key="capability"
        >
          {{ capability }}
        </li>
      </ul>
    </details>
  </section>
  <p
    v-else-if="!ready && !['idle', 'pairing', 'failed'].includes(status.phase)"
    class="controller__text"
  >
    {{ labels.hidden }}
  </p>
</template>
