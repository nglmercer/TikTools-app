import { defineSyncAdapter, type JsonValue, type SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import type { useCloudOverlayStore } from '@/entities/overlay/model/cloud-overlay-store.ts';
import { jsonEqual } from '@/shared/ui/json/json-diff.ts';

export const OVERLAY_SYNC_METHODS = ['bundles.overlays.create', 'bundles.overlays.read', 'bundles.overlays.revise'] as const;

export function overlayFailureText(t: (key: string) => string, code: string): string {
  const known: Record<string, string> = {
    'overlay-not-saved': 'overlaySaveFirst',
    'overlay-review-changed': 'overlayReviewChanged',
    'overlay-draft-changed': 'overlayReviewChanged',
    'overlay-cloud-changed': 'overlayCloudChanged',
  };
  return known[code] ? t(`syncChanges.${known[code]}`) : code || t('syncChanges.actionFailed');
}

export function createOverlaySyncAdapter(deps: {
  overlay: ReturnType<typeof useCloudOverlayStore>;
  supports(method: string): boolean | undefined;
  t(key: string): string;
}) {
  const { overlay, t } = deps;
  const display = (value: unknown) => value === null ? undefined : value as JsonValue;
  async function checked(operation: () => boolean | void | Promise<boolean | void>): Promise<void> {
    try {
      if (await operation() === false) throw new Error(overlay.error ?? 'overlay-review-changed');
    } catch (failure) {
      const code = failure instanceof Error ? failure.message : '';
      throw new Error(overlayFailureText(t, code), { cause: failure });
    }
  }
  return defineSyncAdapter({
    scope: { id: 'overlays', label: t('syncChanges.scopes.overlays') },
    requiresSyncOptIn: false,
    capabilities: { push: true, fetch: true, apply: true, resolve: true },
    resolveLabels: { local: t('syncChanges.sendSavedValue'), cloud: t('syncChanges.applyCloud') },
    kindLabel: () => t('syncChanges.kinds.overlay'),
    watchSource: () => overlay.comparisonSource,
    unavailableReason: () => OVERLAY_SYNC_METHODS.every((method) => deps.supports(method) === true) ? null : t('syncChanges.unsupported'),
    notice: () => t('syncChanges.overlaysNotice'),
    async list(): Promise<SyncChange[]> {
      const row = overlay.review();
      if (!row) return [];
      const localChanged = !jsonEqual(row.local, row.base);
      const side = row.conflict ? 'conflict' : localChanged ? 'local' : 'remote';
      return [{
        id: row.id, kind: 'overlay', name: row.local.title, side,
        status: row.conflict ? 'conflict' : row.base ? 'modified' : 'added',
        before: display(side === 'remote' ? row.local : row.base),
        after: display(side === 'remote' ? row.cloud : row.local),
        base: display(row.base), remote: display(row.cloud),
        canSend: row.canSend, canApply: row.cloud !== null,
        canResolveLocal: false, canResolveCloud: row.cloud !== null,
      }];
    },
    async push(ids) {
      if (ids.length !== 1) throw new Error(t('syncChanges.overlayReviewChanged'));
      await checked(() => overlay.saveReview(ids[0]!));
    },
    fetch: () => checked(() => overlay.fetchRevision()),
    apply: (id) => checked(() => overlay.applyReview(id)),
    async resolve(id, resolution) {
      if (resolution.choice !== 'cloud') throw new Error(t('syncChanges.overlayCloudChanged'));
      await checked(() => overlay.applyReview(id));
    },
  });
}
