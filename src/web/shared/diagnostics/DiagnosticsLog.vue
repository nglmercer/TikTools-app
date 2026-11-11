<script setup lang="ts">
import { computed, inject, ref, watch, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import { diagnosticLog, diagnosticView, formatDiagnostics } from './log.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconFilter from '@/shared/ui/icon/icons/IconFilter.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
const client = inject(controlClientKey, null);
const { t } = useI18n();
const { entries, dropped } = diagnosticLog;
const { source, entityId, selectedId } = diagnosticView;
const level = ref('');
const search = ref('');
const limit = ref(100);
const notice = ref('');
const filtersOpen = ref(false);
const filtersButton = ref<InstanceType<typeof BaseButton> | null>(null);
const filterId = useId();
const filterCount = computed(() => [source.value, entityId.value, level.value, search.value.trim()].filter(Boolean).length);
const hasFilters = computed(() => filterCount.value > 0);
function closeFilters(): void { filtersOpen.value = false; (filtersButton.value?.$el as HTMLElement | undefined)?.focus(); }
const fallback = ref('');
const sources = ['live', 'monitor', 'discovery', 'plugins', 'account', 'sync', 'updates', 'transport', 'catalog', 'install', 'collaboration', 'relay', 'integrations'] as const;
const filtered = computed(() => entries.value.filter(e => (!source.value || e.source === source.value) && (!entityId.value || e.entityId === entityId.value) && (!level.value || e.level === level.value) && (!search.value.trim() || `${e.message} ${e.details} ${e.code} ${e.entityId} ${e.requestId}`.toLowerCase().includes(search.value.trim().toLowerCase()))));
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
watch([source, entityId, level, search], () => { limit.value = 100; notice.value = ''; fallback.value = ''; });
function reset(): void { source.value = ''; entityId.value = ''; level.value = ''; search.value = ''; selectedId.value = ''; }
</script>
<template>
  <section
    class="logs"
    :aria-label="t('diagnostics.title')"
  >
    <div class="logs__toolbar">
      <BaseTextField
        v-model="search"
        class="logs__search"
        type="search"
        :placeholder="t('diagnostics.searchShort')"
        :aria-label="t('diagnostics.search')"
      />
      <BaseButton
        ref="filtersButton"
        class="logs__filter-button"
        :aria-expanded="filtersOpen"
        :aria-controls="filterId"
        @click="filtersOpen = !filtersOpen"
      >
        <template #icon>
          <IconFilter />
        </template>
        {{ t('diagnostics.filters') }}<span
          v-if="filterCount"
          class="logs__count"
        >{{ filterCount }}</span>
      </BaseButton>
      <div
        class="logs__tools"
        role="group"
        :aria-label="t('diagnostics.actions')"
      >
        <BaseIconButton
          :label="t(hasFilters ? 'diagnostics.copyFiltered' : 'diagnostics.copyAll')"
          :disabled="!filtered.length"
          @click="copy(formatDiagnostics(filtered))"
        >
          <IconCopy />
        </BaseIconButton>
        <BaseIconButton
          :label="t('diagnostics.export')"
          :disabled="!filtered.length"
          @click="exportText"
        >
          <IconDownload />
        </BaseIconButton>
        <BaseIconButton
          :label="t('diagnostics.clear')"
          :disabled="!entries.length"
          @click="diagnosticLog.clear(); notice = t('diagnostics.cleared')"
        >
          <IconTrash />
        </BaseIconButton>
      </div>
    </div>
    <div
      v-show="filtersOpen"
      :id="filterId"
      class="logs__filters"
      role="group"
      :aria-label="t('diagnostics.filters')"
      @keydown.esc.stop.prevent="closeFilters"
    >
      <label class="logs__filter"><span>{{ t('diagnostics.sourceLabel') }}</span><BaseComboBox
        :model-value="source"
        :options="sourceOptions"
        :aria-label="t('diagnostics.allSources')"
        @update:model-value="setSource"
      /></label>
      <label class="logs__filter"><span>{{ t('diagnostics.levelLabel') }}</span><BaseComboBox
        :model-value="level"
        :options="levelOptions"
        :aria-label="t('diagnostics.allLevels')"
        @update:model-value="level = $event ?? ''"
      /></label>
      <div class="logs__filter-actions">
        <BaseButton
          variant="ghost"
          :disabled="!hasFilters"
          @click="reset"
        >
          {{ t('diagnostics.resetFilters') }}
        </BaseButton>
        <BaseButton
          v-if="hasFilters"
          :disabled="!entries.length"
          @click="copy(formatDiagnostics(entries))"
        >
          <template #icon>
            <IconCopy />
          </template>{{ t('diagnostics.copyAll') }}
        </BaseButton>
      </div>
    </div>
    <div
      v-if="source || level || entityId"
      class="logs__chips"
      :aria-label="t('diagnostics.activeFilters')"
    >
      <button
        v-if="source"
        class="logs__chip"
        type="button"
        :aria-label="t('diagnostics.removeFilter', { value: t(`diagnostics.sources.${source}`) })"
        @click="setSource('')"
      >
        {{ t(`diagnostics.sources.${source}`) }}<IconClose aria-hidden="true" />
      </button>
      <button
        v-if="level"
        class="logs__chip"
        type="button"
        :aria-label="t('diagnostics.removeFilter', { value: t(`diagnostics.${level}`) })"
        @click="level = ''"
      >
        {{ t(`diagnostics.${level}`) }}<IconClose aria-hidden="true" />
      </button>
      <button
        v-if="entityId"
        class="logs__chip"
        type="button"
        :aria-label="t('diagnostics.removeFilter', { value: entityId })"
        @click="entityId = ''; selectedId = ''"
      >
        {{ entityId }}<IconClose aria-hidden="true" />
      </button>
    </div>
    <div class="logs__summary">
      <span>{{ t('diagnostics.results', { count: filtered.length, total: entries.length }) }}</span><span>{{ t('diagnostics.sessionShort') }}</span>
    </div>
    <p
      v-show="notice"
      class="logs__notice"
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
    <p
      v-if="selectedId && !selected"
      class="logs__muted"
    >
      {{ t('diagnostics.missing') }}
    </p>
    <div
      v-if="filtered.length && selected && !filtered.includes(selected)"
      class="logs__hidden-selection"
    >
      <span>{{ t('diagnostics.selectionFiltered') }}</span><BaseButton
        v-if="!filtersOpen"
        variant="ghost"
        @click="reset"
      >
        {{ t('diagnostics.resetFilters') }}
      </BaseButton>
    </div>
    <div
      v-if="!filtered.length"
      class="logs__empty"
    >
      <IconFilter aria-hidden="true" /><p>{{ t(entries.length ? 'diagnostics.empty' : 'diagnostics.emptyHistory') }}</p><BaseButton
        v-if="hasFilters && !filtersOpen"
        variant="ghost"
        @click="reset"
      >
        {{ t('diagnostics.resetFilters') }}
      </BaseButton>
    </div>
    <ol class="logs__list">
      <li
        v-for="entry in visible"
        :key="entry.id"
        :class="{ 'logs__row--selected': selectedId === entry.id }"
      >
        <button
          class="logs__entry"
          type="button"
          :aria-expanded="selectedId === entry.id"
          :aria-controls="`${filterId}-details-${entry.id}`"
          @click="selectedId = selectedId === entry.id ? '' : entry.id"
        >
          <span class="logs__metadata"><span
            class="logs__severity"
            :class="`logs__severity--${entry.level}`"
            role="img"
            :aria-label="t(`diagnostics.${entry.level}`)"
          >{{ entry.level === 'error' ? '×' : entry.level === 'warning' ? '!' : 'i' }}</span><time :datetime="new Date(entry.lastSeenAt).toISOString()">{{ new Date(entry.lastSeenAt).toLocaleTimeString() }}</time><span class="logs__source">{{ source ? entry.entityId : `${t(`diagnostics.sources.${entry.source}`)}${entry.entityId ? ` / ${entry.entityId}` : ''}` }}</span><span
            v-if="entry.repeatCount > 1"
            class="logs__count"
            :aria-label="t('diagnostics.repeated', { count: entry.repeatCount })"
          >×{{ entry.repeatCount }}</span><IconChevronDown
            v-if="selectedId === entry.id"
            aria-hidden="true"
          /><IconChevronRight
            v-else
            aria-hidden="true"
          /></span>
          <span class="logs__message">{{ entry.message }}</span>
        </button>
        <div
          v-show="selectedId === entry.id"
          :id="`${filterId}-details-${entry.id}`"
          class="logs__selected"
        >
          <small>{{ entry.code }} · {{ new Date(entry.firstSeenAt).toLocaleString() }}</small>
          <small v-if="entry.repeatCount > 1">{{ t('diagnostics.last') }}: {{ new Date(entry.lastSeenAt).toLocaleString() }}</small>
          <pre v-if="entry.details && entry.details.trim() !== entry.message.trim()">{{ entry.details }}</pre>
          <BaseButton
            size="sm"
            @click="copy(formatDiagnostics([entry]))"
          >
            <template #icon>
              <IconCopy />
            </template>{{ t('diagnostics.copyEntry') }}
          </BaseButton>
        </div>
      </li>
    </ol>
    <BaseButton
      v-if="filtered.length > limit"
      @click="limit += 100"
    >
      {{ t('diagnostics.loadMore') }}
    </BaseButton>
  </section>
</template>
<style scoped>
.logs { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); overflow: auto; min-height: 0; color: var(--color-text); font-size: var(--font-size-md); }
.logs p { margin: 0; }
.logs__toolbar { display: flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; }
.logs__search { flex: 1 1 160px; min-width: 100px; }
.logs__tools,.logs__filter-actions,.logs__chips { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
.logs__tools { margin-left: auto; }
.logs__filter-button .logs__count { margin-inline-start: var(--space-2); }
.logs__filters { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-4); padding: var(--space-4); border: var(--border-width) solid var(--color-border-subtle); background: var(--color-bg-app); }
.logs__filter { display: grid; gap: var(--space-2); color: var(--color-text-muted); }
.logs__filter-actions { grid-column: 1 / -1; justify-content: space-between; }
.logs__chip { display: inline-flex; align-items: center; gap: var(--space-2); padding: var(--space-2) var(--space-3); max-width: 100%; overflow-wrap: anywhere; border: var(--border-width) solid var(--color-border); background: var(--color-bg-panel-header); color: var(--color-text); font: inherit; cursor: pointer; }
.logs__chip svg { flex: none; }
.logs__chip:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 1px; }
.logs__summary { display: flex; justify-content: space-between; gap: var(--space-3); flex-wrap: wrap; padding: var(--space-2) 0; font-size: var(--font-size-sm); color: var(--color-text-muted); }
.logs__muted,.logs small,.logs__metadata { color: var(--color-text-muted); }
.logs__empty { display: grid; justify-items: center; gap: var(--space-4); padding: var(--space-7); color: var(--color-text-muted); text-align: center; }
.logs__hidden-selection { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); }
.logs__list { list-style: none; padding: 0; margin: 0; border-top: var(--border-width) solid var(--color-border-subtle); }
.logs__list li { border-bottom: var(--border-width) solid var(--color-border-subtle); }
.logs__list li:nth-child(even) { background: var(--color-bg-app); }
.logs__entry { display: flex; flex-direction: column; text-align: left; width: 100%; gap: var(--space-2); padding: var(--space-4); border: 0; background: transparent; color: inherit; font: inherit; overflow-wrap: anywhere; cursor: pointer; }
.logs__entry:hover { background: var(--color-bg-hover); }
.logs__entry:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: -2px; }
.logs__row--selected { border-left: 2px solid var(--color-border-selected); background: var(--color-bg-app); }
.logs__metadata { display: flex; align-items: center; gap: var(--space-3); font-size: var(--font-size-sm); }
.logs__metadata > svg { flex: none; }
.logs__source { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.logs__severity { display: inline-grid; place-items: center; flex: none; width: 16px; height: 16px; border: 1px solid currentColor; border-radius: 50%; font-weight: bold; }
.logs__severity--error { color: var(--color-text-danger); }
.logs__severity--warning { color: var(--color-warning); }
.logs__count { padding: 0 var(--space-2); background: var(--color-bg-field); font-size: var(--font-size-sm); font-variant-numeric: tabular-nums; }
.logs__message { white-space: pre-wrap; overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.logs__row--selected .logs__message { display: block; }
.logs__selected { display: flex; flex-direction: column; align-items: flex-start; gap: var(--space-3); padding: 0 var(--space-4) var(--space-4); overflow-wrap: anywhere; }
.logs pre { max-width: 100%; margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font: var(--font-size-sm) var(--font-family-mono); }
.logs textarea { box-sizing: border-box; width: 100%; min-height: 120px; padding: var(--space-4); border: var(--border-width) solid var(--color-border); color: var(--color-text); background: var(--color-bg-field); font: var(--font-size-sm) var(--font-family-mono); }
.logs textarea:focus-visible { outline: 2px solid var(--color-border-focus); }
@media (width < 600px) { .logs__search { flex-basis: 100%; } .logs__filters { grid-template-columns: minmax(0, 1fr); } .logs__chip { min-height: 44px; } }
</style>
