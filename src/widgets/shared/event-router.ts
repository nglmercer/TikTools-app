/**
 * Transport-to-widget routing. Keeps WebSocket concerns separate from alert
 * presentation: envelopes in, typed widget events out.
 */

import {
  CHAT_EVENT_TYPE,
  EMOTE_CHAT_EVENT_TYPE,
  FOLLOW_EVENT_TYPE,
  GIFT_EVENT_TYPE,
  JOIN_EVENT_TYPE,
  LIKE_EVENT_TYPE,
  SHARE_EVENT_TYPE,
  SUBSCRIBE_MEMBER_ACTION,
} from './config.ts';
import {
  isChatEvent,
  isEmoteChatEvent,
  isDomainEventEnvelope,
  isFollowEvent,
  isGiftEvent,
  isJoinEvent,
  isLikeEvent,
  isShareEvent,
  parseLiveEvent,
  isRecord,
} from './event-parser.ts';
import type {
  ChatAutomationEvent,
  EmoteChatAutomationEvent,
  FollowAutomationEvent,
  GiftAutomationEvent,
  JoinAutomationEvent,
  LikeAutomationEvent,
  ShareAutomationEvent,
  SharedActivityEvent,
  SharedSubscriptionEvent,
  SharedGiftEvent,
} from './event-types.ts';

export type RoutedLiveEvent =
  | { kind: 'follow'; event: FollowAutomationEvent | SharedActivityEvent }
  | { kind: 'gift'; event: GiftAutomationEvent }
  | { kind: 'platform-gift'; event: SharedGiftEvent }
  | { kind: 'chat'; event: ChatAutomationEvent | EmoteChatAutomationEvent }
  | { kind: 'share'; event: ShareAutomationEvent | SharedActivityEvent }
  | { kind: 'subscribe'; event: JoinAutomationEvent | SharedSubscriptionEvent }
  | { kind: 'like'; event: LikeAutomationEvent | SharedActivityEvent };

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
  if (event.type.startsWith('live.') && isRecord(event.data) && typeof event.data['isHistory'] === 'boolean' && typeof event.data['msgId'] === 'string' && typeof event.data['method'] === 'string') {
    const sharedData = event.data;
    if (event.type === 'live.follow' || event.type === 'live.share' || event.type === 'live.like') {
      if (event.type === 'live.like' && (typeof event.data['count'] !== 'number' || !Number.isFinite(event.data['count']) || event.data['count'] < 0)) return null;
      return { kind: event.type === 'live.follow' ? 'follow' : event.type === 'live.share' ? 'share' : 'like', event: event as SharedActivityEvent };
    }
    if (event.type === 'live.subscription' && typeof event.data['gifted'] === 'boolean' && (event.data['tier'] == null || typeof event.data['tier'] === 'string') && (event.data['months'] == null || (typeof event.data['months'] === 'number' && Number.isFinite(event.data['months']) && event.data['months'] >= 0))) return { kind: 'subscribe', event: event as SharedSubscriptionEvent };
    if (event.type === 'live.gift' && ['id', 'name', 'unit', 'amount'].every((key) => typeof sharedData[key] === 'string') && typeof event.data['count'] === 'number' && Number.isFinite(event.data['count']) && event.data['count'] >= 0 && /^\d+(\.\d+)?$/.test(String(event.data['amount']))) return { kind: 'platform-gift', event: event as SharedGiftEvent };
  }
  if (event.type === FOLLOW_EVENT_TYPE && isFollowEvent(event)) {
    return { kind: 'follow', event };
  }
  if (event.type === GIFT_EVENT_TYPE && isGiftEvent(event)) {
    return { kind: 'gift', event };
  }
  if ((event.type === CHAT_EVENT_TYPE || event.type === 'live.chat') && isChatEvent(event)) {
    return { kind: 'chat', event };
  }
  if (event.type === EMOTE_CHAT_EVENT_TYPE && isEmoteChatEvent(event)) {
    return { kind: 'chat', event };
  }
  if (event.type === SHARE_EVENT_TYPE && isShareEvent(event)) {
    return { kind: 'share', event };
  }
  if (event.type === LIKE_EVENT_TYPE && isLikeEvent(event)) {
    return { kind: 'like', event };
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
