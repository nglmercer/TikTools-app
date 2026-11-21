import { appStorage } from '@/shared/lib/app-storage.ts';
import type { DisplayEvent } from '@/types.ts';

const STORAGE_KEY = 'tiktok-live-feed-history';
const MAX_HISTORY_EVENTS = 200;
const MAX_HISTORY_AGE_MS = 7 * 24 * 60 * 60 * 1000;

export interface FeedHistoryRecord {
  /** `JSON.stringify([creatorId, roomId])` the events belong to. */
  scope: string;
  savedAt: number;
  events: DisplayEvent[];
}

export interface FeedHistoryAdapter {
  load(): FeedHistoryRecord | null;
  save(record: FeedHistoryRecord): void;
  clear(): void;
}

const KINDS = new Set(['chat', 'gift', 'like', 'member', 'social']);

function isSavedEvent(value: unknown): value is DisplayEvent {
  if (!value || typeof value !== 'object') return false;
  const event = value as Partial<DisplayEvent>;
  return typeof event.id === 'number' && Number.isSafeInteger(event.id) &&
    typeof event.receivedAt === 'number' && Number.isFinite(event.receivedAt) &&
    typeof event.author === 'string' && typeof event.kind === 'string' && KINDS.has(event.kind);
}

/** A bounded, validated snapshot of the feed so previous events survive a restart. */
export function createFeedHistory(): FeedHistoryAdapter {
  return {
    load() {
      try {
        const raw = appStorage().getItem(STORAGE_KEY);
        if (!raw) return null;
        const parsed: unknown = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return null;
        const record = parsed as Partial<FeedHistoryRecord>;
        if (!Array.isArray(record.events) || typeof record.scope !== 'string' ||
            typeof record.savedAt !== 'number' || Date.now() - record.savedAt > MAX_HISTORY_AGE_MS) {
          return null;
        }
        return {
          scope: record.scope,
          savedAt: record.savedAt,
          events: record.events.filter(isSavedEvent).slice(-MAX_HISTORY_EVENTS),
        };
      } catch {
        return null;
      }
    },
    save(record) {
      try {
        appStorage().setItem(STORAGE_KEY, JSON.stringify({
          scope: record.scope,
          savedAt: Date.now(),
          events: record.events.slice(-MAX_HISTORY_EVENTS),
        }));
      } catch { /* Storage can be disabled or full; the live feed keeps working. */ }
    },
    clear() {
      try { appStorage().removeItem(STORAGE_KEY); } catch { /* Ignore disabled storage. */ }
    },
  };
}
