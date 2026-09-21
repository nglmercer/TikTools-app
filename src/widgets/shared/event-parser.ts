/**
 * Runtime validation for gateway frames. Network data arrives as `unknown`
 * and is rejected (never throws) when malformed.
 */

import { EVENT_GAP_TOPIC, LIVE_EVENT_TOPIC } from './config.ts';
import type {
  AutomationEventLike,
  AutomationUser,
  DomainEventEnvelope,
  EventGapData,
  FollowAutomationEvent,
  GatewayControlMessage,
  GiftAutomationData,
  GiftAutomationEvent,
  LiveEventPayload,
  SocialAutomationData,
} from './event-types.ts';

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

export function isDomainEventEnvelope(value: unknown): value is DomainEventEnvelope {
  if (!isRecord(value)) return false;
  if (typeof value['topic'] !== 'string') return false;
  return 'data' in value;
}

export function isGatewayControlMessage(value: unknown): value is GatewayControlMessage {
  if (!isRecord(value)) return false;
  switch (value['type']) {
    case 'authenticated':
    case 'pong':
      return true;
    case 'subscribed':
      return (
        Array.isArray(value['topics']) &&
        value['topics'].every((topic): topic is string => typeof topic === 'string')
      );
    case 'error':
      return typeof value['error'] === 'string';
    default:
      return false;
  }
}

function isAutomationUser(value: unknown): value is AutomationUser {
  if (!isRecord(value)) return false;
  return (
    typeof value['uniqueId'] === 'string' &&
    typeof value['nickname'] === 'string' &&
    typeof value['secUid'] === 'string' &&
    (value['userId'] === undefined || value['userId'] === null || typeof value['userId'] === 'string')
  );
}

function isAutomationEventLike(value: unknown): value is AutomationEventLike {
  if (!isRecord(value)) return false;
  if (!isNonEmptyString(value['id'])) return false;
  if (!isNonEmptyString(value['type'])) return false;
  if (!isNumber(value['timestamp'])) return false;
  const user = value['user'];
  if (user !== undefined && user !== null && !isAutomationUser(user)) return false;
  return 'data' in value;
}

function isSocialAutomationData(value: unknown): value is SocialAutomationData {
  if (!isRecord(value)) return false;
  return (
    isNumber(value['action']) &&
    isNumber(value['followCount']) &&
    isNumber(value['shareCount']) &&
    typeof value['method'] === 'string' &&
    typeof value['msgId'] === 'string' &&
    isBoolean(value['isHistory'])
  );
}

function isGiftAutomationData(value: unknown): value is GiftAutomationData {
  if (!isRecord(value)) return false;
  const icon = value['giftIconUrl'];
  return (
    typeof value['giftId'] === 'string' &&
    typeof value['giftName'] === 'string' &&
    isNumber(value['diamondCount']) &&
    isNumber(value['repeatCount']) &&
    isNumber(value['comboCount']) &&
    typeof value['groupId'] === 'string' &&
    isBoolean(value['repeatEnd']) &&
    isBoolean(value['streakable']) &&
    (icon === undefined || icon === null || typeof icon === 'string') &&
    typeof value['method'] === 'string' &&
    typeof value['msgId'] === 'string' &&
    isBoolean(value['isHistory'])
  );
}

/**
 * Validates a `live.event` envelope into its canonical payload. Returns null
 * for any other topic or malformed shape.
 */
export function parseLiveEvent(envelope: DomainEventEnvelope): LiveEventPayload | null {
  if (envelope.topic !== LIVE_EVENT_TOPIC) return null;
  if (!isRecord(envelope.data)) return null;
  const { eventType, event } = envelope.data;
  if (!isNonEmptyString(eventType)) return null;
  if (!isAutomationEventLike(event)) return null;
  return { eventType, event };
}

export function isFollowEvent(event: AutomationEventLike): event is FollowAutomationEvent {
  return event.type === 'tiktok.follow' && isSocialAutomationData(event.data);
}

export function isGiftEvent(event: AutomationEventLike): event is GiftAutomationEvent {
  return event.type === 'tiktok.gift' && isGiftAutomationData(event.data);
}

export function isHistoryEvent(event: AutomationEventLike): boolean {
  return isRecord(event.data) && event.data['isHistory'] === true;
}

export function parseEventGap(envelope: DomainEventEnvelope): EventGapData | null {
  if (envelope.topic !== EVENT_GAP_TOPIC) return null;
  if (!isRecord(envelope.data)) return null;
  const { lost, resync } = envelope.data;
  if (!isNumber(lost) || !isBoolean(resync)) return null;
  return { lost, resync };
}
