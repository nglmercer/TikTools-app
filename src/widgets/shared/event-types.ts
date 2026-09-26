/**
 * Widget-side event types. Automation payloads reuse the generated TikTools
 * contracts; widgets never define their own user/gift/social shapes.
 */

import type {
  AutomationUser,
  ChatAutomationData,
  ConnectionAutomationData,
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

/** Structural subset of the generated AutomationEvent widgets rely on. */
export interface AutomationEventLike {
  id: string;
  type: string;
  timestamp: number;
  user?: AutomationUser | null;
  data: unknown;
}

export interface FollowAutomationEvent {
  id: string;
  type: 'tiktok.follow';
  timestamp: number;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface GiftAutomationEvent {
  id: string;
  type: 'tiktok.gift';
  timestamp: number;
  user?: AutomationUser | null;
  data: GiftAutomationData;
}

export interface ChatAutomationEvent {
  id: string;
  type: 'tiktok.chat';
  timestamp: number;
  user?: AutomationUser | null;
  data: ChatAutomationData;
}

export interface ShareAutomationEvent {
  id: string;
  type: 'tiktok.share';
  timestamp: number;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface JoinAutomationEvent {
  id: string;
  type: 'tiktok.join';
  timestamp: number;
  user?: AutomationUser | null;
  data: MemberAutomationData;
}

export interface LikeAutomationEvent {
  id: string;
  type: 'tiktok.like';
  timestamp: number;
  user?: AutomationUser | null;
  data: LikeAutomationData;
}

export interface SocialAutomationEvent {
  id: string;
  type: 'tiktok.social';
  timestamp: number;
  user?: AutomationUser | null;
  data: SocialAutomationData;
}

export interface RoomStatsAutomationEvent {
  id: string;
  type: 'tiktok.room_stats';
  timestamp: number;
  user?: AutomationUser | null;
  data: RoomStatsAutomationData;
}

export interface ConnectionAutomationEvent {
  id: string;
  type: 'tiktok.connected' | 'tiktok.disconnected';
  timestamp: number;
  user?: AutomationUser | null;
  data: ConnectionAutomationData;
}

export interface PointsAwardedAutomationEvent {
  id: string;
  type: 'points.awarded';
  timestamp: number;
  user?: AutomationUser | null;
  data: PointsAwardedAutomationData;
}

export interface PluginEmitAutomationEvent {
  id: string;
  type: 'plugin.emit';
  timestamp: number;
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
