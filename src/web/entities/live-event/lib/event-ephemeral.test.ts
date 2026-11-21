import { expect, it } from 'vitest';
import type { DisplayEvent } from '@/types.ts';
import { EPHEMERAL_EVENT_MS, isEphemeralEvent, visibleEphemeralEvents } from './event-ephemeral.ts';

const now = 1_000_000;
function event(overrides: Partial<DisplayEvent>): DisplayEvent {
  return { id: 1, receivedAt: now - 1_000, kind: 'chat', author: 'luna', text: 'hola', ...overrides } as DisplayEvent;
}

it('treats likes and joins as reactions but keeps chat and gifts permanent', () => {
  expect(isEphemeralEvent(event({ kind: 'like' }))).toBe(true);
  expect(isEphemeralEvent(event({ kind: 'member', i18nKey: 'joinedLive' }))).toBe(true);
  expect(isEphemeralEvent(event({ kind: 'chat' }))).toBe(false);
  expect(isEphemeralEvent(event({ kind: 'gift' }))).toBe(false);
  expect(isEphemeralEvent(event({ kind: 'member' }))).toBe(false);
});

it('expires reactions after the display window and keeps only the latest ones', () => {
  const events = [
    event({ id: 1, kind: 'like', receivedAt: now - EPHEMERAL_EVENT_MS - 1 }),
    event({ id: 2, kind: 'member', i18nKey: 'joinedLive', receivedAt: now - 2_000 }),
    event({ id: 3, kind: 'like', receivedAt: now - 1_000 }),
    event({ id: 4, kind: 'chat', receivedAt: now - 500 }),
  ];
  expect(visibleEphemeralEvents(events, now).map((entry) => entry.id)).toEqual([2, 3]);
  expect(visibleEphemeralEvents(events, now, 1).map((entry) => entry.id)).toEqual([3]);
  expect(visibleEphemeralEvents(events, now + EPHEMERAL_EVENT_MS)).toEqual([]);
});
