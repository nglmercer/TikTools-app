import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { buildFieldChoices, mergeChoices, type FieldSide } from '@/entities/cloud-sync/lib/sync-merge.ts';
import type { JsonValue, SyncAdapter, SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { diffJson } from '@/shared/ui/json/json-diff.ts';
import type { SyncPanelState } from './use-sync-panel.ts';

export type SyncDiffPanel = Pick<SyncPanelState, 'view' | 'current' | 'historyChange' | 'operation' | 'resolve'>;

/** Presentation and field choices only; mutation policy remains in useSyncPanel. */
export function useSyncDiff(adapter: SyncAdapter, panel: SyncDiffPanel) {
  const { t } = useI18n();
  const caps = adapter.capabilities;
  const layout = ref<'unified' | 'side-by-side'>('unified');
  /** The two sides the diff compares, and how they are labelled. */
  function sides(change: SyncChange): { before: JsonValue | undefined; after: JsonValue | undefined; beforeLabel: string; afterLabel: string } {
    if (change.side === 'remote') return { before: change.before, after: change.after, beforeLabel: t('syncChanges.local'), afterLabel: t('syncChanges.cloudSide') };
    if (change.side === 'conflict') return { before: change.after, after: change.remote, beforeLabel: t('syncChanges.local'), afterLabel: t('syncChanges.cloudSide') };
    return { before: change.before, after: change.after, beforeLabel: t('syncChanges.synced'), afterLabel: t('syncChanges.current') };
  }

  const shown = computed(() => (panel.view.value === 'changes' ? panel.current.value : panel.historyChange.value));
  const shownSides = computed(() => (shown.value ? sides(shown.value) : null));
  const operationStats = computed(() => {
    let added = 0;
    let removed = 0;
    for (const change of panel.operation.value?.changes ?? []) {
      const { stats } = diffJson(change.before, change.after);
      added += stats.added + stats.changed;
      removed += stats.removed + stats.changed;
    }
    return { added, removed };
  });

  // Field-level conflict choice, rebuilt whenever another element is shown.
  const picks = ref<Record<string, FieldSide>>({});
  const conflict = computed(() => {
    const change = panel.current.value;
    if (panel.view.value !== 'changes' || !change || change.side !== 'conflict' || !caps.resolve) return null;
    // Elements whose values never reach the page (changedKeys) cannot be merged field by field.
    const merge = caps.merge && !change.changedKeys;
    return { change, merge, fields: merge ? buildFieldChoices(change.base, change.after, change.remote) : [] };
  });
  watch(conflict, (next) => {
    picks.value = Object.fromEntries((next?.fields ?? []).map((field) => [field.key, field.choice]));
  }, { immediate: true });

  function preview(value: JsonValue | undefined): string {
    if (value === undefined) return '—';
    const text = JSON.stringify(value);
    return text.length > 60 ? `${text.slice(0, 57)}…` : text;
  }

  function setAll(side: FieldSide): void {
    picks.value = Object.fromEntries((conflict.value?.fields ?? []).map((field) => [field.key, side]));
  }

  function applyMerge(): void {
    const current = conflict.value;
    if (!current) return;
    const value = mergeChoices(current.change.after, current.change.remote, current.fields, picks.value);
    if (value !== undefined) void panel.resolve(current.change.id, { choice: 'merged', value });
  }

  const canReset = computed(() => caps.reset && panel.current.value !== null && panel.current.value.side !== 'remote' && panel.current.value.side !== 'conflict');

  return { layout, shown, shownSides, operationStats, picks, conflict, preview, setAll, applyMerge, canReset };
}

export type SyncDiffState = ReturnType<typeof useSyncDiff>;
