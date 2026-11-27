import { beforeEach, expect, it } from 'vitest';
import type { DisplayEvent } from '@/types.ts';
import { createFeedHistory } from '../../../../../../src/web/entities/live-event/lib/feed-history.ts';

beforeEach(() => localStorage.clear());

function event(id: number): DisplayEvent {
  return { id, receivedAt: Date.now(), kind: 'chat', author: 'luna', text: `message ${id}` };
}

it('saves and restores a scoped feed snapshot with its cursor and ids', () => {
  const history = createFeedHistory();
  const events = [event(1), { ...event(2), feedCursor: 42 }];
  history.save({ scope: '["luna","room-1"]', savedAt: Date.now(), events });
  const restored = history.load();
  expect(restored?.scope).toBe('["luna","room-1"]');
  expect(restored?.events.map((entry) => entry.id)).toEqual([1, 2]);
  expect(restored?.events[1]?.feedCursor).toBe(42);
});

it('ignores corrupt, stale and malformed snapshots', () => {
  const history = createFeedHistory();
  localStorage.setItem('tiktok-live-feed-history', '{broken');
  expect(history.load()).toBeNull();
  localStorage.setItem('tiktok-live-feed-history', JSON.stringify({ scope: '', savedAt: 0, events: [event(1)] }));
  expect(history.load()).toBeNull();
  localStorage.setItem('tiktok-live-feed-history', JSON.stringify({ scope: '', savedAt: Date.now(), events: [{ id: 'bad' }] }));
  expect(history.load()?.events).toEqual([]);
  history.clear();
  expect(history.load()).toBeNull();
});
