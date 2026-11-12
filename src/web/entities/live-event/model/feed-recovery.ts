import type { Ref } from 'vue';
import type { UiEvent } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';

export interface FeedBackfillResult {
  oldestCursor: number;
  latestCursor: number;
  depth: number;
  gap: boolean;
  truncated: boolean;
  events: Array<{ cursor: number; event: UiEvent }>;
}

interface RecoveryContext {
  active: () => boolean;
  cursor: () => number | null;
  resetCursor: () => void;
  push: (event: UiEvent, cursor: number) => boolean;
  discontinuity: () => void;
  dropped: Ref<number>;
}

/** One serialized backfill loop per feed, fenced by presentation/session invalidation. */
export function createFeedRecovery(control: ControlClient, context: RecoveryContext) {
  let recoveryGeneration = 0;
  let recoveryInFlight = false;
  let recoveryRequested = false;
  let recoveryAfter: number | null = null;
  let recoveryGaps = 0;
  let recoveredEvents = 0;
  let recoveredGaps = 0;
  let unjournaledSuspended = 0;
  const recoverFeed = async (): Promise<void> => {
    if (recoveryInFlight) { recoveryRequested = true; return; }
    if (typeof (control as { call?: unknown }).call !== 'function') { unjournaledSuspended = 0; context.discontinuity(); return; }
    const generation = recoveryGeneration;
    recoveryInFlight = true;
    let recoveredAny = false;
    let after = recoveryAfter ?? context.cursor() ?? 0;
    recoveryAfter = after;
    try {
      // One host journal is bounded. Page by the last returned cursor, never
      // by latestCursor (which could skip the rest of a truncated page).
      for (let page = 0; page < 16; page += 1) {
        const result = await control.call<FeedBackfillResult>('ui.feed.backfill', { version: 1, after });
        if (!context.active() || generation !== recoveryGeneration) return;
        if (result.latestCursor < after) {
          context.resetCursor(); after = 0; recoveryAfter = 0;
          recoveryGaps += 1; context.discontinuity();
          continue;
        }
        if (unjournaledSuspended > 0) {
          unjournaledSuspended = 0; recoveryGaps += 1; context.discontinuity();
        }
        let recovered = 0;
        let next = after;
        for (const entry of result.events ?? []) {
          if (!Number.isSafeInteger(entry?.cursor) || entry.cursor <= after || !entry.event) continue;
          next = Math.max(next, entry.cursor);
          if (context.push(entry.event, entry.cursor)) recovered += 1;
        }
        if (recovered > 0) {
          recoveredEvents += recovered;
          if (!recoveredAny) recoveredGaps += 1;
          recoveredAny = true;
        }
        context.dropped.value = Math.max(0, context.dropped.value - recovered);
        if (result.gap) { recoveryGaps += 1; context.discontinuity(); }
        if (!result.truncated) { if (!recoveryRequested) recoveryAfter = null; break; }
        if (next <= after || page === 15) { recoveryGaps += 1; context.discontinuity(); break; }
        after = next;
        recoveryAfter = after;
      }
    } catch {
      if (context.active() && generation === recoveryGeneration) { recoveryGaps += 1; context.discontinuity(); }
    } finally {
      recoveryInFlight = false;
      if (recoveryRequested && context.active()) {
        recoveryRequested = false;
        void recoverFeed();
      }
    }
  };

  function invalidate(resetAfter = false): void {
    recoveryGeneration += 1;
    if (resetAfter) recoveryAfter = null;
  }
  function noteGap(previousCursor: number | null): void {
    recoveryAfter = Math.min(recoveryAfter ?? previousCursor ?? 0, previousCursor ?? 0);
  }
  return {
    recover: recoverFeed,
    invalidate,
    noteGap,
    noteUnjournaledDrop: () => { unjournaledSuspended += 1; },
    diagnostics: () => ({ recoveryGaps, recoveredEvents, recoveredGaps, recoveryInFlight }),
  };
}
