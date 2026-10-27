import { describe, expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import type { DisplayEvent } from '@/types.ts';
import { cleanHandle, displayName, eventText, formatEventStamp, hasRichText } from './event-text.ts';
import { filterEvents, windowEvents } from './event-filter.ts';

function event(overrides: Partial<DisplayEvent>): DisplayEvent {
  return { kind: 'chat', author: 'flor', text: 'hello', id: 1, receivedAt: 1_700_000_000_000, ...overrides };
}

const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

describe('eventText', () => {
  it('localizes host keys with params and event fallbacks', () => {
    expect(eventText(event({ kind: 'gift', i18nKey: 'giftSent', giftDetails: { name: 'Rose', count: 3, diamonds: 1 } }), t)).toBe(
      'sent 3× Rose',
    );
    expect(eventText(event({ kind: 'like', i18nKey: 'likeSent', likeCount: 1 }), t)).toBe('sent 1 like');
    expect(eventText(event({ kind: 'like', i18nKey: 'likeSent', i18nParams: { count: 15 } }), t)).toBe('sent 15 likes');
    expect(eventText(event({ kind: 'like', i18nKey: 'likeSent', likeCount: 5, likeUserTotal: 35, i18nParams: { count: 5 } }), t)).toBe(
      'sent 35 likes',
    );
    expect(eventText(event({ kind: 'member', i18nKey: 'joinedLive' }), t)).toBe('joined the LIVE');
    expect(eventText(event({ i18nKey: 'chatMessage', i18nParams: { comment: 'hola' } }), t)).toBe('hola');
    expect(eventText(event({ i18nKey: 'unknown', text: 'raw' }), t)).toBe('raw');
  });

  it('detects comments that need emote rendering', () => {
    expect(hasRichText(event({ text: 'hi [smile]' }), 'hi [smile]')).toBe(true);
    expect(hasRichText(event({ kind: 'gift', text: '[x]' }), '[x]')).toBe(false);
    expect(hasRichText(event({}), 'plain')).toBe(false);
  });

  it('prefers distinct nicknames over handles', () => {
    expect(cleanHandle('@@luna')).toBe('luna');
    expect(displayName('luna', 'Luna ✨')).toBe('Luna ✨');
    expect(displayName('luna', 'luna')).toBe('@luna');
  });

  it('formats full timestamps per locale', () => {
    expect(formatEventStamp(Date.UTC(2026, 9, 19, 12), 'en')).toContain('2026');
  });
});

describe('event filtering', () => {
  const events = [
    event({ id: 1, kind: 'chat', author: 'luna', text: 'hola' }),
    event({ id: 2, kind: 'gift', author: 'mango', text: 'Rose' }),
    event({ id: 3, kind: 'member', author: 'kiwi', text: 'joined', i18nKey: 'joinedLive' }),
    event({ id: 4, kind: 'social', author: 'ana', text: 'followed', i18nKey: 'followedCreator' }),
    event({ id: 5, kind: 'social', author: 'ben', text: 'shared', i18nKey: 'sharedLive' }),
  ];

  it('separates follows and joins from the rest of social and searches author and text', () => {
    expect(filterEvents(events, 'follow', '').map((entry) => entry.id)).toEqual([4]);
    expect(filterEvents(events, 'join', '').map((entry) => entry.id)).toEqual([3]);
    expect(filterEvents(events, 'social', '').map((entry) => entry.id)).toEqual([5]);
    expect(filterEvents(events, 'all', 'ROSE').map((entry) => entry.id)).toEqual([2]);
  });

  it('reuses the previous scan for the same buffer and key', () => {
    expect(filterEvents(events, 'chat', '')).toBe(filterEvents(events, 'chat', ''));
  });

  it('windows only the pinned, unfiltered stream', () => {
    expect(windowEvents(events, { autoScroll: true, filter: 'all', query: '', limit: 2 })).toEqual({
      visible: events.slice(-2),
      windowed: true,
    });
    expect(windowEvents(events, { autoScroll: false, filter: 'all', query: '', limit: 2 }).windowed).toBe(false);
    expect(windowEvents(events, { autoScroll: true, filter: 'chat', query: '', limit: 2 }).windowed).toBe(false);
  });
});
