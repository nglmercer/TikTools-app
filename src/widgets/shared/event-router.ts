/**
 * Transport-to-widget routing. Keeps WebSocket concerns separate from alert
 * presentation: envelopes in, typed widget events out.
 */

import {
  CHAT_EVENT_TYPE,
  FOLLOW_EVENT_TYPE,
  GIFT_EVENT_TYPE,
  JOIN_EVENT_TYPE,
  SHARE_EVENT_TYPE,
  SUBSCRIBE_MEMBER_ACTION,
} from './config.ts';
import {
  isChatEvent,
  isDomainEventEnvelope,
  isFollowEvent,
  isGiftEvent,
  isJoinEvent,
  isShareEvent,
  parseLiveEvent,
} from './event-parser.ts';
import type {
  ChatAutomationEvent,
  FollowAutomationEvent,
  GiftAutomationEvent,
  JoinAutomationEvent,
  ShareAutomationEvent,
} from './event-types.ts';

export type RoutedLiveEvent =
  | { kind: 'follow'; event: FollowAutomationEvent }
  | { kind: 'gift'; event: GiftAutomationEvent }
  | { kind: 'chat'; event: ChatAutomationEvent }
  | { kind: 'share'; event: ShareAutomationEvent }
  | { kind: 'subscribe'; event: JoinAutomationEvent };

/**
 * Validates one gateway frame into a typed widget event. Returns null for
 * non-live topics, unknown event types, and malformed payloads. History and
 * duplicate filtering stay in the controllers so each widget owns its policy.
 */
export function routeLiveEvent(value: unknown): RoutedLiveEvent | null {
  if (!isDomainEventEnvelope(value)) return null;
  const payload = parseLiveEvent(value);
  if (!payload) return null;
  const { event } = payload;
  if (event.type === FOLLOW_EVENT_TYPE && isFollowEvent(event)) {
    return { kind: 'follow', event };
  }
  if (event.type === GIFT_EVENT_TYPE && isGiftEvent(event)) {
    return { kind: 'gift', event };
  }
  if (event.type === CHAT_EVENT_TYPE && isChatEvent(event)) {
    return { kind: 'chat', event };
  }
  if (event.type === SHARE_EVENT_TYPE && isShareEvent(event)) {
    return { kind: 'share', event };
  }
  // Plain joins stay unrouted: no widget renders them, and the subscribe
  // widget only wants member action 3.
  if (event.type === JOIN_EVENT_TYPE && isJoinEvent(event)) {
    if (event.data.action === SUBSCRIBE_MEMBER_ACTION) {
      return { kind: 'subscribe', event };
    }
    return null;
  }
  return null;
}
