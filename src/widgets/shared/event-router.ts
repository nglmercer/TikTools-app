/**
 * Transport-to-widget routing. Keeps WebSocket concerns separate from alert
 * presentation: envelopes in, typed follow/gift events out.
 */

import { FOLLOW_EVENT_TYPE, GIFT_EVENT_TYPE } from './config.ts';
import {
  isDomainEventEnvelope,
  isFollowEvent,
  isGiftEvent,
  parseLiveEvent,
} from './event-parser.ts';
import type { FollowAutomationEvent, GiftAutomationEvent } from './event-types.ts';

export type RoutedLiveEvent =
  | { kind: 'follow'; event: FollowAutomationEvent }
  | { kind: 'gift'; event: GiftAutomationEvent };

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
  return null;
}
