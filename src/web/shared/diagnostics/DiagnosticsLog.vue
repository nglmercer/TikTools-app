<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { diagnosticLog, diagnosticView, formatDiagnostics } from './log.ts';
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
const visible = computed(() => filtered.value.slice(-limit.value).toReversed());
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
watch(() => [source.value, entityId.value, diagnosticView.request.value], () => { search.value = ''; level.value = ''; limit.value = 100; });
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
    <div class="logs__tools">
      <input
        v-model="search"
        type="search"
        :placeholder="t('diagnostics.search')"
        :aria-label="t('diagnostics.search')"
      >
      <select
        v-model="source"
        :aria-label="t('diagnostics.allSources')"
      >
        <option value="">
          {{ t('diagnostics.allSources') }}
        </option><option
          v-for="item in sources"
          :key="item"
          :value="item"
        >
          {{ t(`diagnostics.sources.${item}`) }}
        </option>
      </select>
      <select
        v-model="level"
        :aria-label="t('diagnostics.allLevels')"
      >
        <option value="">
          {{ t('diagnostics.allLevels') }}
        </option><option
          v-for="item in (['info', 'warning', 'error'] as const)"
          :key="item"
          :value="item"
        >
          {{ t(`diagnostics.${item}`) }}
        </option>
      </select>
      <button @click="reset">
        {{ t('diagnostics.resetFilters') }}
      </button>
    </div>
    <p v-if="entityId">
      {{ t('diagnostics.filterEntity', { entity: entityId }) }}
    </p>
    <div class="logs__tools">
      <button @click="copy(formatDiagnostics(entries))">
        {{ t('diagnostics.copyAll') }}
      </button>
      <button @click="copy(formatDiagnostics(filtered))">
        {{ t('diagnostics.copyFiltered') }}
      </button>
      <button @click="exportText">
        {{ t('diagnostics.export') }}
      </button>
      <button @click="diagnosticLog.clear(); notice = t('diagnostics.cleared')">
        {{ t('diagnostics.clear') }}
      </button>
    </div>
    <p role="status">
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
    <article
      v-if="selected"
      class="logs__selected"
    >
      <small>{{ selected.code }} · {{ new Date(selected.lastSeenAt).toLocaleString() }}</small><br>
      <strong>{{ selected.message }}</strong><pre>{{ selected.details }}</pre>
      <button @click="copy(formatDiagnostics([selected]))">
        {{ t('diagnostics.copyEntry') }}
      </button>
    </article>
    <p v-if="!filtered.length">
      {{ t('diagnostics.empty') }}
    </p>
    <ol class="logs__list">
      <li
        v-for="entry in visible"
        :key="entry.id"
      >
        <button
          class="logs__entry"
          :aria-pressed="selectedId === entry.id"
          @click="selectedId = entry.id"
        >
          <small>{{ new Date(entry.lastSeenAt).toLocaleTimeString() }} · {{ t(`diagnostics.${entry.level}`) }} · {{ t(`diagnostics.sources.${entry.source}`) }}{{ entry.entityId ? ` / ${entry.entityId}` : '' }}</small>
          <span>{{ entry.message }}</span><small v-if="entry.repeatCount > 1">{{ t('diagnostics.repeated', { count: entry.repeatCount }) }}</small>
        </button>
      </li>
    </ol>
    <button
      v-if="filtered.length > limit"
      @click="limit += 100"
    >
      +100
    </button>
  </section>
</template>
<style scoped>
.logs { padding: 16px; overflow: auto; min-height: 0; color: var(--color-text); }
.logs__tools { display:flex; gap:8px; flex-wrap:wrap; }
.logs button,.logs input,.logs select,.logs textarea { font:inherit; color:inherit; background:var(--color-bg-secondary, #20242d); border:1px solid var(--color-border, #454955); border-radius:6px; padding:8px; min-height:36px; }
.logs input { flex:1; min-width:140px; }
.logs textarea { width:100%; min-height:120px; }
.logs__muted,.logs small { color:var(--color-text-muted); }
.logs__list { list-style:none; padding:0; }
.logs__list li { margin-top:8px; }
.logs__entry { display:flex; flex-direction:column; text-align:left; width:100%; gap:6px; overflow-wrap:anywhere; }
.logs__entry[aria-pressed=true],.logs__selected { border:1px solid var(--color-border-focus, #8378fa); }
.logs__selected { padding:12px; border-radius:8px; overflow-wrap:anywhere; }
.logs pre { white-space:pre-wrap; overflow-wrap:anywhere; font-size:12px; }
.logs button { cursor:pointer; }
.logs :focus-visible { outline:2px solid #a69bff; outline-offset:2px; }
@media (pointer:coarse) { .logs button,.logs select { min-height:44px; } }
</style>
