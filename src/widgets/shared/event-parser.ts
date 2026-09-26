/**
 * Runtime validation for gateway frames. Network data arrives as `unknown`
 * and is rejected (never throws) when malformed.
 */

import { EVENT_GAP_TOPIC, LIVE_EVENT_TOPIC } from './config.ts';
import type {
  AutomationEventLike,
  AutomationUser,
  ChatAutomationData,
  ChatAutomationEvent,
  ConnectionAutomationData,
  ConnectionAutomationEvent,
  DomainEventEnvelope,
  EventGapData,
  FollowAutomationEvent,
  GatewayControlMessage,
  GiftAutomationData,
  GiftAutomationEvent,
  JoinAutomationEvent,
  LikeAutomationData,
  LikeAutomationEvent,
  LiveEventPayload,
  MemberAutomationData,
  PluginEmitAutomationData,
  PluginEmitAutomationEvent,
  PointsAwardedAutomationData,
  PointsAwardedAutomationEvent,
  RoomStatsAutomationData,
  RoomStatsAutomationEvent,
  ShareAutomationEvent,
  SocialAutomationData,
  SocialAutomationEvent,
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

function isChatAutomationData(value: unknown): value is ChatAutomationData {
  if (!isRecord(value)) return false;
  return (
    typeof value['comment'] === 'string' &&
    typeof value['method'] === 'string' &&
    typeof value['msgId'] === 'string' &&
    isBoolean(value['isHistory'])
  );
}

function isMemberAutomationData(value: unknown): value is MemberAutomationData {
  if (!isRecord(value)) return false;
  return (
    isNumber(value['memberCount']) &&
    isNumber(value['action']) &&
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

function isLikeAutomationData(value: unknown): value is LikeAutomationData {
  if (!isRecord(value)) return false;
  return (
    isNumber(value['count']) &&
    isNumber(value['total']) &&
    typeof value['method'] === 'string' &&
    typeof value['msgId'] === 'string' &&
    isBoolean(value['isHistory'])
  );
}

function isRoomStatsAutomationData(value: unknown): value is RoomStatsAutomationData {
  if (!isRecord(value)) return false;
  return (
    isNumber(value['viewers']) &&
    isNumber(value['totalUsers']) &&
    isNumber(value['popularity']) &&
    isNumber(value['anonymous']) &&
    typeof value['method'] === 'string' &&
    typeof value['msgId'] === 'string' &&
    isBoolean(value['isHistory'])
  );
}

function isConnectionAutomationData(value: unknown): value is ConnectionAutomationData {
  if (!isRecord(value)) return false;
  return typeof value['uniqueId'] === 'string' && typeof value['roomId'] === 'string';
}

function isPointsAwardedAutomationData(value: unknown): value is PointsAwardedAutomationData {
  if (!isRecord(value)) return false;
  return (
    typeof value['uniqueId'] === 'string' &&
    isNumber(value['delta']) &&
    isNumber(value['totalPoints']) &&
    isNumber(value['level']) &&
    typeof value['currencyName'] === 'string' &&
    typeof value['reason'] === 'string'
  );
}

function isPluginEmitAutomationData(value: unknown): value is PluginEmitAutomationData {
  if (!isRecord(value)) return false;
  return (
    typeof value['emitType'] === 'string' && isNumber(value['depth']) && 'payload' in value
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

export function isChatEvent(event: AutomationEventLike): event is ChatAutomationEvent {
  return event.type === 'tiktok.chat' && isChatAutomationData(event.data);
}

export function isShareEvent(event: AutomationEventLike): event is ShareAutomationEvent {
  return event.type === 'tiktok.share' && isSocialAutomationData(event.data);
}

export function isJoinEvent(event: AutomationEventLike): event is JoinAutomationEvent {
  return event.type === 'tiktok.join' && isMemberAutomationData(event.data);
}

export function isLikeEvent(event: AutomationEventLike): event is LikeAutomationEvent {
  return event.type === 'tiktok.like' && isLikeAutomationData(event.data);
}

export function isSocialEvent(event: AutomationEventLike): event is SocialAutomationEvent {
  return event.type === 'tiktok.social' && isSocialAutomationData(event.data);
}

export function isRoomStatsEvent(event: AutomationEventLike): event is RoomStatsAutomationEvent {
  return event.type === 'tiktok.room_stats' && isRoomStatsAutomationData(event.data);
}

export function isConnectionEvent(event: AutomationEventLike): event is ConnectionAutomationEvent {
  return (
    (event.type === 'tiktok.connected' || event.type === 'tiktok.disconnected') &&
    isConnectionAutomationData(event.data)
  );
}

export function isPointsAwardedEvent(
  event: AutomationEventLike,
): event is PointsAwardedAutomationEvent {
  return event.type === 'points.awarded' && isPointsAwardedAutomationData(event.data);
}

export function isPluginEmitEvent(event: AutomationEventLike): event is PluginEmitAutomationEvent {
  return event.type === 'plugin.emit' && isPluginEmitAutomationData(event.data);
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
