import { expect, test } from 'vitest';
import { asEnvelope } from '../../../../../../src/web/entities/automation/model/automation-execution.ts';

test('wraps a bare custom data object in an event envelope', () => {
  expect(asEnvelope({ comment: '!play vaundy', isHistory: false }, 'tiktok.chat')).toEqual({
    type: 'tiktok.chat',
    data: { comment: '!play vaundy', isHistory: false },
  });
});

test('keeps a full custom envelope untouched', () => {
  const envelope = { type: 'tiktok.gift', data: { giftName: 'Rose' }, user: { uniqueId: 'a' } };
  expect(asEnvelope(envelope, 'tiktok.chat')).toBe(envelope);
});
