import { describe, expect, it } from 'vitest';
import { liveFieldsFrom } from './cloud-overlay-fields.ts';

const gift = (overrides: Record<string, unknown> = {}) => ({
  topic: 'live.event',
  data: {
    eventType: 'tiktok.gift',
    event: {
      id: 'e1',
      type: 'tiktok.gift',
      timestamp: 1,
      user: { uniqueId: 'ana', nickname: 'Ana', secUid: 'sec-ana' },
      data: { giftId: 'g1', giftName: 'Rose', repeatCount: 3, comboCount: 2, diamondCount: 1, groupId: 'grp', repeatEnd: true, streakable: false, method: 'gift', msgId: 'm1', isHistory: false, ...overrides },
    },
  },
});

describe('cloud overlay live fields', () => {
  it('maps a gift to the viewer and gift fields the overlay allows', () => {
    expect(liveFieldsFrom(gift())).toEqual({
      'viewer.name': 'Ana',
      'viewer.handle': '@ana',
      'gift.name': 'Rose',
      'gift.count': '3',
      'gift.diamonds': '1',
    });
  });

  it('reports the largest count and never a negative diamond total', () => {
    const values = liveFieldsFrom(gift({ repeatCount: 0, comboCount: 5, diamondCount: -4 }));
    expect(values['gift.count']).toBe('5');
    expect(values['gift.diamonds']).toBe('0');
  });

  it('returns nothing for an envelope it does not understand', () => {
    expect(liveFieldsFrom({ topic: 'event.gap', data: {} })).toEqual({});
  });
});
