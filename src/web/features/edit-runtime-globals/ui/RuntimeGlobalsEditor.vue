<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useGlobalsStore } from '@/entities/runtime-global/model/globals-store.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import { globalsFromRows, variablesFromRows, useGlobalsDraft } from '../model/globals-draft.ts';
import { variableKinds, type RuntimeVariableKind } from '@/entities/runtime-global/lib/runtime-variable.ts';

const { t } = useI18n();
const store = useGlobalsStore();
const { globals, variables, revision, typedSupported, loading, error } = storeToRefs(store);
const draft = useGlobalsDraft(globals, computed(() => ({ variables: variables.value, revision: revision.value })));
const { rows, dirty, baseRevision, errors, blocked } = draft;
const saving = ref(false);
const incompatibleDraft = computed(() => !typedSupported.value && baseRevision.value !== null);

async function save(): Promise<void> {
  if (blocked.value || saving.value || incompatibleDraft.value) return;
  saving.value = true;
  try {
    if (typedSupported.value) {
      if (!baseRevision.value) return;
      await store.saveTyped(baseRevision.value, variablesFromRows(rows.value));
    } else {
      await store.save(globalsFromRows(rows.value));
    }
    draft.markSaved();
  } catch {
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  void store.loadTyped();
});
watch(typedSupported, () => { void store.loadTyped(); });
</script>

<template>
  <div
    class="globals-editor"
    :class="{ 'globals-editor--typed': typedSupported }"
  >
    <!-- The tab already names the area: the bar holds the explanation (as a tooltip) and the icon actions. -->
    <header class="globals-editor__bar">
      <span
        v-tooltip="t('globals.hint', { token: '{{ globals.key }}' })"
        class="globals-editor__info"
        role="img"
        :aria-label="t('globals.hint', { token: '{{ globals.key }}' })"
      >
        <IconInfo :size="16" />
      </span>
      <span class="globals-editor__spacer" />
      <slot
        name="sync"
        :dirty="dirty || saving || loading || incompatibleDraft"
      />
      <BaseIconButton
        :label="t('globals.add')"
        variant="default"
        :disabled="loading || saving"
        @click="draft.add()"
      >
        <IconPlus />
      </BaseIconButton>
      <BaseIconButton
        :label="t('globals.discard')"
        variant="default"
        :disabled="(!dirty && !incompatibleDraft) || saving"
        @click="draft.discard(); store.loadTyped()"
      >
        <IconUndo />
      </BaseIconButton>
      <BaseIconButton
        :label="saving ? t('globals.saving') : t('globals.save')"
        variant="primary"
        :disabled="blocked || saving || loading || !dirty || incompatibleDraft || (typedSupported && !baseRevision)"
        @click="save"
      >
        <IconCheck />
      </BaseIconButton>
    </header>
    <p
      v-if="error || incompatibleDraft"
      class="globals-editor__failure"
      role="alert"
    >
      {{ error ?? t('globals.typedUnavailable') }}
    </p>
    <div
      class="globals-editor__table"
      role="table"
      :aria-label="t('globals.title')"
    >
      <div
        class="globals-editor__row globals-editor__row--header"
        role="row"
      >
        <span role="columnheader">{{ t('globals.key') }}</span>
        <span
          v-if="typedSupported"
          role="columnheader"
        >{{ t('globals.type') }}</span>
        <span role="columnheader">{{ t('globals.value') }}</span>
        <span role="columnheader" />
      </div>
      <p
        v-if="!rows.length"
        class="globals-editor__empty"
      >
        {{ t('globals.empty') }}
      </p>
      <div
        v-for="row in rows"
        :key="row.id"
        class="globals-editor__row"
        role="row"
      >
        <span role="cell">
          <BaseTextField
            class="globals-editor__field"
            :model-value="row.key"
            :aria-label="t('globals.key')"
            placeholder="commandPort"
            :invalid="errors.has(row.id)"
            :disabled="loading || saving"
            spellcheck="false"
            @update:model-value="(key) => draft.update(row.id, { key })"
          />
        </span>
        <span
          v-if="typedSupported"
          role="cell"
        >
          <select
            class="globals-editor__select"
            :value="row.kind ?? 'text'"
            :aria-label="t('globals.type')"
            :disabled="loading || saving"
            @change="draft.update(row.id, { kind: ($event.target as HTMLSelectElement).value as RuntimeVariableKind })"
          >
            <option
              v-for="kind in variableKinds"
              :key="kind"
              :value="kind"
            >{{ t(`globals.types.${kind}`) }}</option>
          </select>
        </span>
        <span role="cell">
          <select
            v-if="row.kind === 'boolean'"
            class="globals-editor__select"
            :value="row.value"
            :aria-label="t('globals.value')"
            :disabled="loading || saving"
            @change="draft.update(row.id, { value: ($event.target as HTMLSelectElement).value })"
          >
            <option
              value=""
              disabled
            >{{ t('globals.value') }}</option>
            <option value="true">{{ t('globals.true') }}</option>
            <option value="false">{{ t('globals.false') }}</option>
          </select>
          <BaseTextField
            v-else
            class="globals-editor__field"
            :type="row.kind === 'secret' ? 'password' : 'text'"
            :model-value="row.value"
            :aria-label="t('globals.value')"
            :placeholder="row.kind === 'list' ? '[&quot;a&quot;, &quot;b&quot;]' : '8080'"
            :disabled="loading || saving"
            :invalid="errors.has(row.id)"
            autocomplete="off"
            spellcheck="false"
            @update:model-value="(value) => draft.update(row.id, { value })"
          />
        </span>
        <span role="cell">
          <BaseIconButton
            variant="danger"
            :label="t('globals.remove')"
            :disabled="loading || saving"
            @click="draft.remove(row.id)"
          >
            <IconTrash />
          </BaseIconButton>
        </span>
        <p
          v-if="errors.has(row.id)"
          class="globals-editor__error"
          role="alert"
        >
          {{ t(`globals.errors.${errors.get(row.id)}`) }}
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.globals-editor {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-height: 0;
  padding: var(--space-4) var(--space-3);
  overflow-y: auto;
}

.globals-editor__bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
}

.globals-editor__info {
  display: inline-flex;
  color: var(--color-text-muted);
}

.globals-editor__table {
  flex: none;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
}

.globals-editor__row {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 3fr) var(--control-height-md);
  gap: var(--space-3);
  align-items: center;
  padding: var(--space-2) var(--space-3);
}

.globals-editor__row + .globals-editor__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.globals-editor--typed .globals-editor__row {
  grid-template-columns: minmax(0, 2fr) minmax(100px, 1fr) minmax(0, 3fr) var(--control-height-md);
}

.globals-editor__select {
  width: 100%;
  min-width: 0;
  height: var(--control-height-md);
  padding: 0 var(--space-2);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
  color: var(--color-text);
  font: inherit;
}

.globals-editor__row--header {
  background: var(--color-bg-panel-header);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.globals-editor__field {
  width: 100%;
  font-family: var(--font-family-mono);
}

.globals-editor__error {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}

.globals-editor__empty {
  margin: 0;
  padding: var(--space-6);
  border-top: var(--border-width) solid var(--color-border-subtle);
  color: var(--color-text-subtle);
  text-align: center;
}

.globals-editor__failure {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.globals-editor__spacer {
  flex: 1;
}
</style>
