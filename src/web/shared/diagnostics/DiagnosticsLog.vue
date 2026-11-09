<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { diagnosticLog, diagnosticView, formatDiagnostics } from './log.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import CollectionToolbar from '@/shared/ui/collection-toolbar/CollectionToolbar.vue';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
const client = inject(controlClientKey, null);
const { t } = useI18n();
const { entries, dropped } = diagnosticLog;
const { source, entityId, selectedId } = diagnosticView;
const level = ref('');
const search = ref('');
const limit = ref(100);
const notice = ref('');
const fallback = ref('');
const sources = ['live', 'monitor', 'discovery', 'plugins', 'account', 'sync', 'updates', 'transport'] as const;
const filtered = computed(() => entries.value.filter(e => (!source.value || e.source === source.value) && (!entityId.value || e.entityId === entityId.value) && (!level.value || e.level === level.value) && (!search.value || `${e.message} ${e.details} ${e.code}`.toLowerCase().includes(search.value.toLowerCase()))));
const selected = computed(() => entries.value.find(e => e.id === selectedId.value));
const sourceOptions = computed(() => [{ value: '', label: t('diagnostics.allSources') }, ...sources.map(value => ({ value, label: t(`diagnostics.sources.${value}`) }))]);
const levelOptions = computed(() => [{ value: '', label: t('diagnostics.allLevels') }, ...(['info', 'warning', 'error'] as const).map(value => ({ value, label: t(`diagnostics.${value}`) }))]);
const visible = computed(() => {
  const recent = filtered.value.slice(-limit.value).toReversed();
  const active = selected.value;
  return active && filtered.value.includes(active) && !recent.some(entry => entry.id === active.id) ? [active, ...recent] : recent;
});
function setSource(value: string | undefined): void { source.value = value ?? ''; entityId.value = ''; selectedId.value = ''; }

async function copy(text: string): Promise<void> {
  try { await navigator.clipboard.writeText(text); notice.value = t('diagnostics.copied'); fallback.value = ''; }
  catch { fallback.value = text; notice.value = t('diagnostics.copyFailed'); }
}
async function exportText(): Promise<void> {
  const content = formatDiagnostics(filtered.value);
  try {
    if (window.ipc && client) await client.call('system.saveFile', { filename: 'tiktools-diagnostics.txt', content });
    else {
      const url = URL.createObjectURL(new Blob([content], { type: 'text/plain;charset=utf-8' }));
      const link = document.createElement('a'); link.href = url; link.download = 'tiktools-diagnostics.txt'; link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
    notice.value = t('diagnostics.exported');
  } catch { notice.value = t('diagnostics.exportFailed'); fallback.value = content; }
}
watch(() => diagnosticView.request.value, () => { search.value = ''; level.value = ''; limit.value = 100; });
watch(entries, () => { diagnosticLog.markRead(); fallback.value = ''; });
function reset(): void { source.value = ''; entityId.value = ''; level.value = ''; search.value = ''; selectedId.value = ''; }
</script>
<template>
  <section
    class="logs"
    :aria-label="t('diagnostics.title')"
  >
    <p class="logs__muted">
      {{ t('diagnostics.session') }}
    </p>
    <CollectionToolbar :label="t('diagnostics.search')">
      <template #search>
        <BaseTextField
          v-model="search"
          type="search"
          :placeholder="t('diagnostics.search')"
          :aria-label="t('diagnostics.search')"
        />
      </template>
      <BaseComboBox
        :model-value="source"
        :options="sourceOptions"
        :aria-label="t('diagnostics.allSources')"
        @update:model-value="setSource"
      />
      <BaseComboBox
        :model-value="level"
        :options="levelOptions"
        :aria-label="t('diagnostics.allLevels')"
        @update:model-value="level = $event ?? ''"
      />
      <template #actions>
        <BaseButton
          variant="ghost"
          @click="reset"
        >
          {{ t('diagnostics.resetFilters') }}
        </BaseButton>
      </template>
    </CollectionToolbar>
    <p v-if="entityId">
      {{ t('diagnostics.filterEntity', { entity: entityId }) }}
    </p>
    <div
      class="logs__tools"
      role="group"
      :aria-label="t('diagnostics.title')"
    >
      <BaseButton @click="copy(formatDiagnostics(entries))">
        {{ t('diagnostics.copyAll') }}
      </BaseButton>
      <BaseButton @click="copy(formatDiagnostics(filtered))">
        {{ t('diagnostics.copyFiltered') }}
      </BaseButton>
      <BaseButton @click="exportText">
        {{ t('diagnostics.export') }}
      </BaseButton>
      <BaseButton
        variant="ghost"
        @click="diagnosticLog.clear(); notice = t('diagnostics.cleared')"
      >
        {{ t('diagnostics.clear') }}
      </BaseButton>
    </div>
    <p
      v-show="notice"
      role="status"
    >
      {{ notice }}
    </p>
    <textarea
      v-if="fallback"
      readonly
      :value="fallback"
      :aria-label="t('diagnostics.copyAll')"
      @focus="($event.target as HTMLTextAreaElement).select()"
    />
    <p
      v-if="dropped"
      class="logs__muted"
    >
      {{ t('diagnostics.dropped', { count: dropped }) }}
    </p>
    <p v-if="selectedId && !selected">
      {{ t('diagnostics.missing') }}
    </p>
    <p v-if="!filtered.length">
      {{ t('diagnostics.empty') }}
    </p>
    <ol class="logs__list">
      <li
        v-for="entry in visible"
        :key="entry.id"
        :class="{ 'logs__row--selected': selectedId === entry.id }"
      >
        <button
          class="logs__entry"
          :aria-expanded="selectedId === entry.id"
          :aria-controls="`details-${entry.id}`"
          @click="selectedId = selectedId === entry.id ? '' : entry.id"
        >
          <span class="logs__metadata"><time :datetime="new Date(entry.lastSeenAt).toISOString()">{{ new Date(entry.lastSeenAt).toLocaleTimeString() }}</time><BaseStatus
            :label="t(`diagnostics.${entry.level}`)"
            :tone="entry.level"
          /><span>{{ t(`diagnostics.sources.${entry.source}`) }}{{ entry.entityId ? ` / ${entry.entityId}` : '' }}</span></span>
          <span class="logs__message">{{ entry.message }}</span>
          <small v-if="entry.repeatCount > 1">{{ t('diagnostics.repeated', { count: entry.repeatCount }) }}</small>
        </button>
        <div
          v-show="selectedId === entry.id"
          :id="`details-${entry.id}`"
          class="logs__selected"
        >
          <small>{{ entry.code }} · {{ new Date(entry.firstSeenAt).toLocaleString() }}</small>
          <small v-if="entry.repeatCount > 1">{{ t('diagnostics.last') }}: {{ new Date(entry.lastSeenAt).toLocaleString() }}</small>
          <pre v-if="entry.details">{{ entry.details }}</pre>
          <BaseButton
            size="sm"
            @click="copy(formatDiagnostics([entry]))"
          >
            {{ t('diagnostics.copyEntry') }}
          </BaseButton>
        </div>
      </li>
    </ol>
    <BaseButton
      v-if="filtered.length > limit"
      @click="limit += 100"
    >
      +100
    </BaseButton>
  </section>
</template>
<style scoped>
.logs { display: flex; flex-direction: column; gap: var(--space-4); padding: var(--space-5); overflow: auto; min-height: 0; color: var(--color-text); font-size: var(--font-size-md); }
.logs p { margin: 0; }
.logs__tools { display: flex; gap: var(--space-2); flex-wrap: wrap; }
.logs__muted,.logs small,.logs__metadata { color: var(--color-text-muted); }
.logs__list { list-style: none; padding: 0; margin: 0; border-top: var(--border-width) solid var(--color-border-subtle); }
.logs__list li { border-bottom: var(--border-width) solid var(--color-border-subtle); }
.logs__entry { display: flex; flex-direction: column; text-align: left; width: 100%; gap: var(--space-2); padding: var(--space-4); border: 0; background: transparent; color: inherit; font: inherit; overflow-wrap: anywhere; cursor: pointer; }
.logs__entry:hover { background: var(--color-bg-hover); }
.logs__entry:focus-visible { outline: var(--border-width) solid var(--color-border-focus); outline-offset: calc(-1 * var(--border-width)); }
.logs__row--selected { border-left: var(--border-width) solid var(--color-border-selected); background: var(--color-bg-app); }
.logs__metadata { display: flex; flex-wrap: wrap; align-items: baseline; gap: var(--space-4); font-size: var(--font-size-sm); }
.logs__message { white-space: pre-wrap; overflow-wrap: anywhere; }
.logs__selected { display: flex; flex-direction: column; align-items: flex-start; gap: var(--space-3); padding: 0 var(--space-4) var(--space-4); overflow-wrap: anywhere; }
.logs pre { max-width: 100%; margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font: var(--font-size-sm) var(--font-family-mono); }
.logs textarea { box-sizing: border-box; width: 100%; min-height: 120px; padding: var(--space-4); border: var(--border-width) solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-bg-field); font: var(--font-size-sm) var(--font-family-mono); }
.logs textarea:focus-visible { outline: var(--border-width) solid var(--color-border-focus); }
</style>
