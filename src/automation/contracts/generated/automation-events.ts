// THIS FILE IS GENERATED. Run bun run contracts:generate.

import type { JsonValue } from './json-value.ts';

export interface AutomationCreator {
  "roomId": string;
  "uniqueId": string;
}

export interface AutomationEvent {
  "connectionId"?: string | null;
  "creator"?: { "roomId": string; "uniqueId": string } | null;
  "data": JsonValue;
  "id": string;
  "points"?: { "delta": number; "level": number; "total": number } | null;
  "sourceEventId"?: string | null;
  "timestamp": number;
  "type": string;
  "user"?: { "nickname": string; "secUid": string; "uniqueId": string; "userId": string | null } | null;
}

export interface AutomationPoints {
  "delta": number;
  "level": number;
  "total": number;
}

export interface AutomationUser {
  "nickname": string;
  "secUid": string;
  "uniqueId": string;
  "userId"?: string | null;
}

export interface ChatAutomationData {
  "comment": string;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
}

export interface ConnectionAutomationData {
  "roomId": string;
  "uniqueId": string;
}

export interface GiftAutomationData {
  "comboCount": number;
  "diamondCount": number;
  "giftIconUrl"?: string | null;
  "giftId": string;
  "giftName": string;
  "groupId": string;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "repeatCount": number;
  "repeatEnd": boolean;
  "streakable": boolean;
}

export interface LikeAutomationData {
  "count": number;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "total": number;
}

export interface MemberAutomationData {
  "action": number;
  "isHistory": boolean;
  "memberCount": number;
  "method": string;
  "msgId": string;
}

export interface PluginEmitAutomationData {
  "depth": number;
  "emitType": string;
  "payload": JsonValue;
}

export interface PointsAwardedAutomationData {
  "currencyName": string;
  "delta": number;
  "level": number;
  "reason": string;
  "totalPoints": number;
  "uniqueId": string;
}

export interface RoomStatsAutomationData {
  "anonymous": number;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "popularity": number;
  "totalUsers": number;
  "viewers": number;
}

export interface SocialAutomationData {
  "action": number;
  "followCount": number;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "shareCount": number;
}
