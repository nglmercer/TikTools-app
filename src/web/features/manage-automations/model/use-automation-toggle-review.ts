import { computed, ref, shallowRef } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { AutomationHost, AutomationKind, AutomationRevision } from '@/entities/automation/model/record-versions.ts';
import type { AutomationControls } from './use-automation-controls.ts';

export interface ToggleTarget {
  readonly kind: AutomationKind;
  readonly id: string;
  readonly name: string;
  readonly enabled: boolean;
  /** The revision and host the owner saw when asking; the write is made against exactly these. */
  readonly revision: AutomationRevision;
  readonly host: AutomationHost | null;
}

type ToggleControls = Pick<AutomationControls, 'revisionFor' | 'readRecord' | 'setRecordEnabled' | 'sourceHost'>;

/**
 * The review behind enabling or disabling one automation. It captures the record, its
 * revision and the host when the owner asks, writes against exactly those, and after a
 * failure or a recovery it requires a fresh read before another confirmation. A record
 * that already has the requested state is reported as applied and is never written again.
 */
export function useAutomationToggleReview(options: {
  controls: ToggleControls;
  /** The record as the page lists it now, or `undefined` when it is not there. */
  findRecord: (kind: AutomationKind, id: string) => { readonly name: string } | undefined;
  t: (key: string) => string;
}) {
  const { controls, findRecord, t } = options;
  const open = ref(false);
  const busy = ref(false);
  const needsReview = ref(false);
  const satisfied = ref(false);
  const error = ref('');
  const target = shallowRef<ToggleTarget | null>(null);
  /** Confirming is blocked while a write runs, while a fresh read is required and once it is already applied. */
  const confirmBlocked = computed(() => busy.value || needsReview.value || satisfied.value);

  /** Opens the confirmation for a record, or reports the missing revision without opening anything. */
  function request(kind: AutomationKind, id: string, enabled: boolean): void {
    if (busy.value || open.value) return;
    const record = findRecord(kind, id);
    const revision = controls.revisionFor(kind, id);
    error.value = '';
    if (!record || revision === undefined) {
      error.value = t('automations.conflict.missingRevision');
      return;
    }
    target.value = { kind, id, name: record.name, enabled, revision, host: controls.sourceHost.value };
    needsReview.value = false;
    satisfied.value = false;
    open.value = true;
  }

  async function confirm(): Promise<void> {
    const current = target.value;
    if (!current || confirmBlocked.value) return;
    busy.value = true;
    try {
      await controls.setRecordEnabled(current.kind, current.id, current.enabled, current.revision, current.host);
      open.value = false;
    } catch (failure) {
      error.value = errorMessage(failure);
      needsReview.value = true;
    } finally {
      busy.value = false;
    }
  }

  /** Reads the record again and adopts its revision and host; the owner confirms again afterwards. */
  async function review(): Promise<void> {
    const current = target.value;
    if (!current || busy.value) return;
    busy.value = true;
    try {
      const latest = await controls.readRecord(current.kind, current.id);
      if (!latest.record) {
        error.value = t('automations.conflict.deleted');
        return;
      }
      target.value = { ...current, name: latest.record.name, revision: latest.revision, host: latest.host ?? null };
      satisfied.value = latest.record.enabled === current.enabled;
      needsReview.value = false;
      error.value = satisfied.value ? t('automations.conflict.alreadyApplied') : '';
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      busy.value = false;
    }
  }

  /** The library changed behind the review: the next confirmation needs a fresh read. */
  function markNeedsReview(): void {
    needsReview.value = true;
  }

  /** A recovery changed the host's definitions: an open review must read again before it can be confirmed. */
  function recovered(message: string): void {
    if (!open.value || !target.value) return;
    needsReview.value = true;
    satisfied.value = false;
    error.value = message;
  }

  return { open, busy, needsReview, satisfied, error, target, confirmBlocked, request, confirm, review, markNeedsReview, recovered };
}
