import { liveMessageIdentity } from '../../shared/live-identity.ts';
import type { LiveActivityAutomationData, LiveGiftAutomationData, LiveSubscriptionAutomationData, LikeAggregation, PlatformId } from '../../automation/contracts/generated/automation-events.ts';
/**
 * Widget-side event types. Automation payloads reuse the generated TikTools
 * contracts; widgets never define their own user/gift/social shapes.
 */

import type {
  AutomationUser,
  ChatAutomationData,
  ConnectionAutomationData,
  EmoteChatAutomationData,
  GiftAutomationData,
  LikeAutomationData,
  MemberAutomationData,
  PluginEmitAutomationData,
  PointsAwardedAutomationData,
  RoomStatsAutomationData,
  SocialAutomationData,
} from '../../automation/contracts/generated/automation-events.ts';

export type {
  AutomationUser,
  ChatAutomationData,
  ConnectionAutomationData,
  EmoteChatAutomationData,
  GiftAutomationData,
  LikeAutomationData,
  MemberAutomationData,
  PluginEmitAutomationData,
  PointsAwardedAutomationData,
  RoomStatsAutomationData,
  SocialAutomationData,
};

/** Stable gateway transport envelope: `{ topic, data }`. */
export interface DomainEventEnvelope {
  topic: string;
  data: unknown;
}

/** `live.event` payload: canonical type plus the automation event. */
export interface LiveEventPayload {
  eventType: string;
  event: AutomationEventLike;
}

/** Structural subset of the generated `AutomationCreator`. */
export interface AutomationCreatorLike {
  uniqueId: string;
  roomId: string;
}

/** Structural subset of the generated AutomationEvent widgets rely on. */
export interface AutomationEventLike {
  platform?: PlatformId;
  id: string;
  type: string;
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: unknown;
}

/**
 * The identity the dedup caches key on.
 *
 * The envelope `id` is generated fresh by the host for every emission, so if
 * TikTok replays a message the host publishes it under a *new* id and a cache
 * keyed on `id` never matches. `data.msgId` is the upstream message id and is
 * stable across replays; the room id keeps two rooms' messages from colliding.
 *
 * Falls back to `id` when there is no usable upstream id (`msgId` absent or
 * `"0"`, which is the proto default rather than a real message id).
 */
export function eventIdentity(event: AutomationEventLike): string {
  const data = event.data;
  const msgId =
    typeof data === 'object' && data !== null && !Array.isArray(data)
      ? (data as Record<string, unknown>)['msgId']
      : undefined;
  return liveMessageIdentity(event.type, msgId, event.creator?.roomId, event.creator?.uniqueId) ?? event.id;
}

export interface FollowAutomationEvent {
  id: string;
  type: 'tiktok.follow';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface GiftAutomationEvent {
  id: string;
  type: 'tiktok.gift';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: GiftAutomationData;
}

export interface ChatAutomationEvent {
  id: string;
  type: 'tiktok.chat' | 'live.chat';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: ChatAutomationData;
}

export interface SharedActivityEvent extends AutomationEventLike {
  type: 'live.follow' | 'live.share' | 'live.like';
  data: LiveActivityAutomationData & { action?: never; aggregation?: LikeAggregation };
}
export interface SharedSubscriptionEvent extends AutomationEventLike {
  type: 'live.subscription';
  data: LiveSubscriptionAutomationData & { action?: never };
}
export interface SharedGiftEvent extends AutomationEventLike {
  type: 'live.gift';
  data: LiveGiftAutomationData;
}

export interface EmoteChatAutomationEvent {
  id: string;
  type: 'tiktok.emote_chat';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: EmoteChatAutomationData;
}

export interface ShareAutomationEvent {
  id: string;
  type: 'tiktok.share';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface JoinAutomationEvent {
  id: string;
  type: 'tiktok.join';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: MemberAutomationData;
}

export interface LikeAutomationEvent {
  id: string;
  type: 'tiktok.like';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: LikeAutomationData;
}

export interface SocialAutomationEvent {
  id: string;
  type: 'tiktok.social';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface RoomStatsAutomationEvent {
  id: string;
  type: 'tiktok.room_stats';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: RoomStatsAutomationData;
}

export interface ConnectionAutomationEvent {
  id: string;
  type: 'tiktok.connected' | 'tiktok.disconnected';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: ConnectionAutomationData;
}

export interface PointsAwardedAutomationEvent {
  id: string;
  type: 'points.awarded';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: PointsAwardedAutomationData;
}

export interface PluginEmitAutomationEvent {
  id: string;
  type: 'plugin.emit';
  timestamp: number;
  creator?: AutomationCreatorLike | null;
  user?: AutomationUser | null;
  data: PluginEmitAutomationData;
}

/** `event.gap` payload: broadcast messages were skipped, do not resync. */
export interface EventGapData {
  lost: number;
  resync: boolean;
}

export type GatewayControlMessage =
  | { type: 'authenticated' }
  | { type: 'subscribed'; topics: string[] }
  | { type: 'pong' }
  | { type: 'error'; error: string };

/** Primary display name with the canonical fallback chain. */
export function displayNameFor(user: AutomationUser | null | undefined): string {
  const nickname = user?.nickname?.trim();
  if (nickname) return nickname;
  const uniqueId = user?.uniqueId?.trim();
  if (uniqueId) return uniqueId;
  return 'Viewer';
}

export function handleFor(user: AutomationUser | null | undefined): string {
  const uniqueId = user?.uniqueId?.trim();
  return uniqueId ? `@${uniqueId}` : '';
}
