<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { onMounted, ref } from 'vue';
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
import { globalsFromRows, useGlobalsDraft } from '../model/globals-draft.ts';

const { t } = useI18n();
const store = useGlobalsStore();
const { globals, loading, error } = storeToRefs(store);
const draft = useGlobalsDraft(globals);
const { rows, dirty, errors, blocked } = draft;
const saving = ref(false);

async function save(): Promise<void> {
  if (blocked.value || saving.value) return;
  saving.value = true;
  try {
    await store.save(globalsFromRows(rows.value));
    draft.markSaved();
  } catch {
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  void store.load();
});
</script>

<template>
  <div class="globals-editor">
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
      <BaseIconButton
        :label="t('globals.add')"
        variant="default"
        @click="draft.add()"
      >
        <IconPlus />
      </BaseIconButton>
      <BaseIconButton
        :label="t('globals.discard')"
        variant="default"
        :disabled="!dirty || saving"
        @click="draft.discard()"
      >
        <IconUndo />
      </BaseIconButton>
      <BaseIconButton
        :label="saving ? t('globals.saving') : t('globals.save')"
        variant="primary"
        :disabled="blocked || saving || loading || !dirty"
        @click="save"
      >
        <IconCheck />
      </BaseIconButton>
    </header>
    <p
      v-if="error"
      class="globals-editor__failure"
      role="alert"
    >
      {{ error }}
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
            spellcheck="false"
            @update:model-value="(key) => draft.update(row.id, { key })"
          />
        </span>
        <span role="cell">
          <BaseTextField
            class="globals-editor__field"
            :model-value="row.value"
            :aria-label="t('globals.value')"
            placeholder="8080"
            spellcheck="false"
            @update:model-value="(value) => draft.update(row.id, { value })"
          />
        </span>
        <span role="cell">
          <BaseIconButton
            variant="danger"
            :label="t('globals.remove')"
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
