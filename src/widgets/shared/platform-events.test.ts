import { expect, test } from 'bun:test';
import { ChatController } from './chat-controller.ts';
import { GiftController } from './gift-controller.ts';
import { routeLiveEvent } from './event-router.ts';

function envelope(type: string, data: Record<string, unknown>, platform = 'twitch') {
  return { topic: 'live.event', data: { eventType: type, event: { id: `${platform}-${type}`, type, platform, timestamp: 1,
    creator: { uniqueId: `${platform}:channel`, roomId: 'same-room' }, user: { userId: '42', uniqueId: `${platform}:2:42`, nickname: 'Alice', secUid: '' },
    data: { msgId: 'same-id', method: type, isHistory: false, ...data } } } };
}
test('chat widget displays shared events and separates provider identities', () => {
  const chat = new ChatController({ onMessages: () => {} });
  expect(chat.handleEnvelope(envelope('live.chat', { comment: 'hello', emotes: [] }))).toBe('added');
  expect(chat.handleEnvelope(envelope('live.chat', { comment: 'hello', emotes: [] }))).toBe('duplicate');
  expect(chat.handleEnvelope(envelope('live.chat', { comment: 'other', emotes: [] }, 'youtube'))).toBe('added');
  expect(chat.snapshot.map((m) => m.text)).toEqual(['hello', 'other']);
  chat.dispose();
});
test('Bits retain their unit in gift widgets and never acquire a diamond value', () => {
  const controller = new GiftController({ onShow: () => {}, onHide: () => {}, onUpdate: () => {} });
  expect(controller.handleEnvelope(envelope('live.gift', { id: 'bits', name: 'Bits', count: 1, unit: 'twitch_bits', amount: '100' }))).toBe('shown');
  expect(controller.currentAlert?.unit).toBe('twitch_bits');
  expect(controller.currentAlert?.amount).toBe('100');
  expect(controller.currentAlert?.totalDiamonds).toBe(0);
  expect(routeLiveEvent(envelope('live.gift', { id: 'bits', name: 'Bits', count: 1, unit: 'twitch_bits', amount: 'NaN' }))).toBeNull();
  expect(routeLiveEvent(envelope('live.subscription', { tier: '1000', months: 1, gifted: false }))?.kind).toBe('subscribe');
  controller.dispose();
});
