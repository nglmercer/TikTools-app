import type { DisplayEvent } from '@/types.ts';

/** Likes and joins are reactions: they live briefly in the feed, not in its history. */
export const EPHEMERAL_EVENT_MS = 8_000;
export const MAX_EPHEMERAL_EVENTS = 8;

export function isEphemeralEvent(event: DisplayEvent): boolean {
  return event.kind === 'like' || event.i18nKey === 'joinedLive';
}

export function visibleEphemeralEvents(
  events: readonly DisplayEvent[],
  now: number,
  limit = MAX_EPHEMERAL_EVENTS,
): DisplayEvent[] {
  return events
    .filter((event) => isEphemeralEvent(event) && now - event.receivedAt < EPHEMERAL_EVENT_MS)
    .slice(-limit);
}
