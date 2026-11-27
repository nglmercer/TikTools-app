import { expect, test } from 'vitest';
import { asEnvelope } from '../../../../../../src/web/entities/automation/model/automation-execution.ts';
import { EVENT_KIND } from '../../../../../shared/index.ts';

test('wraps a bare custom data object in an event envelope', () => {
  expect(asEnvelope({ comment: '!play vaundy', isHistory: false }, EVENT_KIND.tiktokChat)).toEqual({
    type: EVENT_KIND.tiktokChat,
    data: { comment: '!play vaundy', isHistory: false },
  });
});

test('keeps a full custom envelope untouched', () => {
  const envelope = { type: EVENT_KIND.tiktokGift, data: { giftName: 'Rose' }, user: { uniqueId: 'a' } };
  expect(asEnvelope(envelope, EVENT_KIND.tiktokChat)).toBe(envelope);
});
