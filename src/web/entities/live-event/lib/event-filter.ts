import type { DisplayEvent, EventFilter } from '@/types.ts';

export const eventFilters: readonly EventFilter[] = ['all', 'chat', 'gift', 'like', 'follow', 'join', 'social'];

export type EventCategory = Exclude<EventFilter, 'all'>;
export const MAX_PER_CATEGORY = 100;

/** Bucket used for retention: follows and joins get their own share so a
 *  flood of likes or joins cannot push chat out of the buffer. */
export function eventCategory(event: DisplayEvent): EventCategory {
  if (event.kind === 'social') return event.i18nKey === 'followedCreator' ? 'follow' : 'social';
  if (event.kind === 'member') return 'join';
  return event.kind;
}

/** Keeps the newest `limit` events of every category, preserving arrival order. */
export function retainPerCategory(
  events: readonly DisplayEvent[],
  limit = MAX_PER_CATEGORY,
): DisplayEvent[] {
  const counts = new Map<EventCategory, number>();
  const kept: DisplayEvent[] = [];
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const event = events[i]!;
    const category = eventCategory(event);
    const count = counts.get(category) ?? 0;
    if (count >= limit) continue;
    counts.set(category, count + 1);
    kept.push(event);
  }
  return kept.reverse();
}

const cache = new WeakMap<readonly DisplayEvent[], { key: string; result: DisplayEvent[] }>();

function matchesKind(event: DisplayEvent, filter: EventFilter): boolean {
  if (filter === 'all') return true;
  if (filter === 'follow') return event.kind === 'social' && event.i18nKey === 'followedCreator';
  if (filter === 'join') return event.kind === 'member';
  if (filter === 'social') return event.kind === 'social' && event.i18nKey !== 'followedCreator';
  return event.kind === filter;
}

export function filterEvents(events: readonly DisplayEvent[], filter: EventFilter, query: string): readonly DisplayEvent[] {
  const needle = query.trim().toLowerCase();
  // The default feed view is already the complete buffer. Returning it
  // directly avoids an O(n) copy before the windowing pass on every packet.
  if (filter === 'all' && !needle) return events;
  const key = `${filter}::${needle}`;
  const cached = cache.get(events);
  if (cached?.key === key) return cached.result;
  const result = events.filter(
    (event) =>
      matchesKind(event, filter) &&
      (!needle || event.author.toLowerCase().includes(needle) || event.text.toLowerCase().includes(needle) || event.giftDetails?.name.toLowerCase().includes(needle) || event.giftDetails?.giftId === needle),
  );
  cache.set(events, { key, result });
  return result;
}

function exceedsPerCategory(events: readonly DisplayEvent[], limit: number): boolean {
  const counts = new Map<EventCategory, number>();
  for (const event of events) {
    const category = eventCategory(event);
    const count = (counts.get(category) ?? 0) + 1;
    if (count > limit) return true;
    counts.set(category, count);
  }
  return false;
}

export function windowEvents(
  events: readonly DisplayEvent[],
  options: { autoScroll: boolean; filter: EventFilter; query: string; limit: number },
): { visible: readonly DisplayEvent[]; windowed: boolean } {
  const windowed = options.autoScroll && options.filter === 'all' && !options.query.trim() && events.length > options.limit;
  // Cap per category, not globally: a flood of likes/joins must not hide chat.
  // The live buffer already applies the default retention cap, so the common
  // path only verifies that invariant and reuses the input instead of building
  // another reversed copy for every packet.
  return { visible: windowed && exceedsPerCategory(events, options.limit) ? retainPerCategory(events, options.limit) : events, windowed };
}
