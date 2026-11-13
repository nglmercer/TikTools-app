// THIS FILE IS GENERATED. Run bun scripts/task.ts contracts.

import type { JsonObject, JsonValue } from './json-value.ts';

export type ActionStatus = "pending" | "running" | "completed" | "failed" | "uncertain";

export interface AutomationCreator {
  "roomId": string;
  "uniqueId": string;
}

export interface AutomationEvent {
  "connectionId"?: string | null;
  "creator"?: { "roomId": string; "uniqueId": string } | null;
  "data": JsonValue;
  "id": string;
  "intel"?: { "comment"?: { "casefolded"?: string | null; "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null; "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null; "nfc"?: string | null; "nfkc"?: string | null; "normalized"?: string | null; "obfuscation"?: { "confusables"?: boolean | null; "detected"?: boolean | null; "flags"?: (JsonValue)[] | null; "leetspeak"?: boolean | null; "mixedScripts"?: boolean | null; "punctuationFlood"?: boolean | null; "repetition"?: boolean | null; "score"?: number | null } | null; "rebus"?: { "candidate"?: string | null; "confidence"?: number | null; "score"?: number | null; "strong"?: boolean | null } | null; "spam"?: { "calibrated"?: boolean | null; "detected"?: boolean | null; "model"?: string | null; "reasons"?: (JsonValue)[] | null; "score"?: number | null } | null; "truncated"?: boolean; "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null; "unicode"?: { "bidirectional"?: number | null; "confusables"?: number | null; "invisible"?: number | null; "mixedScripts"?: boolean | null; "score"?: number | null; "suspicious"?: boolean | null } | null } | null; "processing"?: { "status": string } | null; "providers"?: JsonValue; "user"?: { "nickname"?: { "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null; "normalized"?: string | null; "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null } | null; "uniqueId"?: { "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null; "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null } | null } | null } | null;
  "platform"?: "tiktok" | "twitch" | "youtube" | "facebook" | "kick" | null;
  "points"?: { "delta": number; "level": number; "total": number } | null;
  "sourceEventId"?: string | null;
  "timestamp": number;
  "type": string;
  "user"?: { "avatarUrl"?: string | null; "nickname": string; "secUid": string; "uniqueId": string; "userId"?: string | null } | null;
}

export interface AutomationPoints {
  "delta": number;
  "level": number;
  "total": number;
}

export interface AutomationUser {
  "avatarUrl"?: string | null;
  "nickname": string;
  "secUid": string;
  "uniqueId": string;
  "userId"?: string | null;
}

export interface Capabilities {
  "chat": boolean;
  "discovery": boolean;
  "follows": boolean;
  "gifts": boolean;
  "joins": boolean;
  "likes": boolean;
  "moderation": boolean;
  "roomStats": boolean;
  "sendChat": boolean;
  "shares": boolean;
  "subscriptions": boolean;
}

export interface ChannelIdentity {
  "handle": string;
  "id": string;
  "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick";
  "roomId": string;
}

export interface ChatAutomationData {
  "comment": string;
  "emotes"?: ({ "emoteType"?: number | null; "id": string; "imageUrl"?: string | null; "index"?: number | null; "packageId"?: string | null; "shortcode"?: string | null })[];
  "isHistory": boolean;
  "method": string;
  "msgId": string;
}

export interface ChatAutomationEmote {
  "emoteType"?: number | null;
  "id": string;
  "imageUrl"?: string | null;
  "index"?: number | null;
  "packageId"?: string | null;
  "shortcode"?: string | null;
}

export interface CompetitionAutomationData {
  "actionId"?: string | null;
  "actionStatus"?: "pending" | "running" | "completed" | "failed" | "uncertain" | null;
  "competitionId": string;
  "creatorId": string;
  "generation": number;
  "optionId"?: string | null;
  "rankingId": string;
  "roomId": string;
  "selectionId"?: string | null;
  "selectionMode"?: "highest_score" | "uniform_random" | "weighted_random" | null;
  "winnerId"?: string | null;
  "winnerScore"?: number | null;
}

export interface ConnectionAutomationData {
  "roomId": string;
  "uniqueId": string;
}

export interface ConnectionSnapshot {
  "capabilities": { "chat": boolean; "discovery": boolean; "follows": boolean; "gifts": boolean; "joins": boolean; "likes": boolean; "moderation": boolean; "roomStats": boolean; "sendChat": boolean; "shares": boolean; "subscriptions": boolean };
  "channel"?: { "handle": string; "id": string; "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick"; "roomId": string } | null;
  "connectionId": string;
  "error"?: { "code": string; "message": string; "retryAfterMs"?: number | null } | null;
  "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick";
  "reconnectAttempt": number;
  "retryAfterMs"?: number | null;
  "state": "connecting" | "connected" | "reconnecting" | "disconnected" | "error";
  "target": string;
}

export type ConnectionState = "connecting" | "connected" | "reconnecting" | "disconnected" | "error";

export interface CounterTransition {
  "previous": number;
  "total": number;
}

export interface EmoteChatAutomationData {
  "emotes"?: ({ "emoteType"?: number | null; "id": string; "imageUrl"?: string | null; "index"?: number | null; "packageId"?: string | null; "shortcode"?: string | null })[];
  "isHistory": boolean;
  "method": string;
  "msgId": string;
}

export interface EventIntel {
  "comment"?: { "casefolded"?: string | null; "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null; "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null; "nfc"?: string | null; "nfkc"?: string | null; "normalized"?: string | null; "obfuscation"?: { "confusables"?: boolean | null; "detected"?: boolean | null; "flags"?: (JsonValue)[] | null; "leetspeak"?: boolean | null; "mixedScripts"?: boolean | null; "punctuationFlood"?: boolean | null; "repetition"?: boolean | null; "score"?: number | null } | null; "rebus"?: { "candidate"?: string | null; "confidence"?: number | null; "score"?: number | null; "strong"?: boolean | null } | null; "spam"?: { "calibrated"?: boolean | null; "detected"?: boolean | null; "model"?: string | null; "reasons"?: (JsonValue)[] | null; "score"?: number | null } | null; "truncated"?: boolean; "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null; "unicode"?: { "bidirectional"?: number | null; "confusables"?: number | null; "invisible"?: number | null; "mixedScripts"?: boolean | null; "score"?: number | null; "suspicious"?: boolean | null } | null } | null;
  "processing"?: { "status": string } | null;
  "providers"?: JsonValue;
  "user"?: { "nickname"?: { "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null; "normalized"?: string | null; "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null } | null; "uniqueId"?: { "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null; "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null } | null } | null;
}

export interface GiftAggregation {
  "cumulative": number;
  "diamonds": number;
  "increment": number;
}

export interface GiftAutomationData {
  "aggregation"?: { "cumulative": number; "diamonds": number; "increment": number } | null;
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

export interface IntelComment {
  "casefolded"?: string | null;
  "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null;
  "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null;
  "nfc"?: string | null;
  "nfkc"?: string | null;
  "normalized"?: string | null;
  "obfuscation"?: { "confusables"?: boolean | null; "detected"?: boolean | null; "flags"?: (JsonValue)[] | null; "leetspeak"?: boolean | null; "mixedScripts"?: boolean | null; "punctuationFlood"?: boolean | null; "repetition"?: boolean | null; "score"?: number | null } | null;
  "rebus"?: { "candidate"?: string | null; "confidence"?: number | null; "score"?: number | null; "strong"?: boolean | null } | null;
  "spam"?: { "calibrated"?: boolean | null; "detected"?: boolean | null; "model"?: string | null; "reasons"?: (JsonValue)[] | null; "score"?: number | null } | null;
  "truncated"?: boolean;
  "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null;
  "unicode"?: { "bidirectional"?: number | null; "confusables"?: number | null; "invisible"?: number | null; "mixedScripts"?: boolean | null; "score"?: number | null; "suspicious"?: boolean | null } | null;
}

export interface IntelComposition {
  "allCaps"?: boolean | null;
  "digits"?: number | null;
  "elongated"?: boolean | null;
  "emojiCount"?: number | null;
  "emojiOnly"?: boolean | null;
  "emojiRatio"?: number | null;
  "letters"?: number | null;
  "mentions"?: number | null;
  "repetitionScore"?: number | null;
  "urls"?: number | null;
}

export interface IntelHandle {
  "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null;
  "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null;
}

export interface IntelLanguage {
  "candidates"?: (JsonValue)[] | null;
  "confidence": number;
  "top": string;
}

export interface IntelLanguageCandidate {
  "confidence": number;
  "language": string;
}

export interface IntelNickname {
  "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null;
  "normalized"?: string | null;
  "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null;
}

export interface IntelObfuscation {
  "confusables"?: boolean | null;
  "detected"?: boolean | null;
  "flags"?: (JsonValue)[] | null;
  "leetspeak"?: boolean | null;
  "mixedScripts"?: boolean | null;
  "punctuationFlood"?: boolean | null;
  "repetition"?: boolean | null;
  "score"?: number | null;
}

export interface IntelProcessing {
  "status": string;
}

export interface IntelPronunciation {
  "confidence"?: number | null;
  "dialect"?: string | null;
  "ipa"?: string | null;
  "language"?: string | null;
}

export interface IntelRebus {
  "candidate"?: string | null;
  "confidence"?: number | null;
  "score"?: number | null;
  "strong"?: boolean | null;
}

export interface IntelSpam {
  "calibrated"?: boolean | null;
  "detected"?: boolean | null;
  "model"?: string | null;
  "reasons"?: (JsonValue)[] | null;
  "score"?: number | null;
}

export interface IntelTts {
  "confidence"?: number | null;
  "language"?: string | null;
  "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null;
  "reason"?: string | null;
  "source"?: string | null;
  "speak"?: boolean;
  "text": string;
}

export interface IntelUnicode {
  "bidirectional"?: number | null;
  "confusables"?: number | null;
  "invisible"?: number | null;
  "mixedScripts"?: boolean | null;
  "score"?: number | null;
  "suspicious"?: boolean | null;
}

export interface IntelUser {
  "nickname"?: { "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null; "normalized"?: string | null; "tts"?: { "confidence"?: number | null; "language"?: string | null; "pronunciation"?: { "confidence"?: number | null; "dialect"?: string | null; "ipa"?: string | null; "language"?: string | null } | null; "reason"?: string | null; "source"?: string | null; "speak"?: boolean; "text": string } | null } | null;
  "uniqueId"?: { "composition"?: { "allCaps"?: boolean | null; "digits"?: number | null; "elongated"?: boolean | null; "emojiCount"?: number | null; "emojiOnly"?: boolean | null; "emojiRatio"?: number | null; "letters"?: number | null; "mentions"?: number | null; "repetitionScore"?: number | null; "urls"?: number | null } | null; "language"?: { "candidates"?: (JsonValue)[] | null; "confidence": number; "top": string } | null } | null;
}

export interface LikeAggregation {
  "global"?: { "previous": number; "total": number } | null;
  "user"?: { "previous": number; "total": number } | null;
}

export interface LikeAutomationData {
  "aggregation"?: { "global"?: { "previous": number; "total": number } | null; "user"?: { "previous": number; "total": number } | null } | null;
  "count": number;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "total": number;
}

export interface LiveActivityAutomationData {
  "count"?: number | null;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "total"?: number | null;
}

export interface LiveChatAutomationData {
  "comment": string;
  "emotes"?: ({ "emoteType"?: number | null; "id": string; "imageUrl"?: string | null; "index"?: number | null; "packageId"?: string | null; "shortcode"?: string | null })[];
  "isHistory": boolean;
  "method": string;
  "msgId": string;
}

export interface LiveEnvelope {
  "channel": { "handle": string; "id": string; "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick"; "roomId": string };
  "connectionId": string;
  "extensions"?: JsonObject;
  "isHistory": boolean;
  "occurredAt": number;
  "payload": { "comment": string; "emotes": (JsonValue)[]; "kind": "chat" } | { "kind": "follow" } | { "gifted": boolean; "kind": "subscription"; "months"?: number | null; "tier"?: string | null } | { "amount": string; "count": number; "id": string; "kind": "gift"; "name": string; "unit": string } | { "count": number; "kind": "like"; "total"?: number | null } | { "count": number; "kind": "share" } | { "kind": "join" } | { "kind": "room_stats"; "totalUsers"?: number | null; "viewers": number } | { "kind": "connected" } | { "kind": "disconnected" } | { "data": JsonValue; "eventType": string; "kind": "extension" };
  "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick";
  "receivedAt": number;
  "schemaVersion": number;
  "sessionId": string;
  "sourceEventId"?: string | null;
  "user"?: { "avatarUrl"?: string | null; "displayName": string; "handle": string; "id": string; "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick" } | null;
}

export interface LiveError {
  "code": string;
  "message": string;
  "retryAfterMs"?: number | null;
}

export interface LiveGiftAutomationData {
  "amount": string;
  "count": number;
  "id": string;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "name": string;
  "unit": string;
}

export type LivePayload = { "comment": string; "emotes": (JsonValue)[]; "kind": "chat" } | { "kind": "follow" } | { "gifted": boolean; "kind": "subscription"; "months"?: number | null; "tier"?: string | null } | { "amount": string; "count": number; "id": string; "kind": "gift"; "name": string; "unit": string } | { "count": number; "kind": "like"; "total"?: number | null } | { "count": number; "kind": "share" } | { "kind": "join" } | { "kind": "room_stats"; "totalUsers"?: number | null; "viewers": number } | { "kind": "connected" } | { "kind": "disconnected" } | { "data": JsonValue; "eventType": string; "kind": "extension" };

export interface LiveRoomStatsAutomationData {
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "totalUsers"?: number | null;
  "viewers": number;
}

export interface LiveSubscriptionAutomationData {
  "gifted": boolean;
  "isHistory": boolean;
  "method": string;
  "months"?: number | null;
  "msgId": string;
  "tier"?: string | null;
}

export interface MemberAutomationData {
  "action": number;
  "isHistory": boolean;
  "memberCount": number;
  "method": string;
  "msgId": string;
}

export interface PlatformDescriptor {
  "authentication": string;
  "capabilities": { "chat": boolean; "discovery": boolean; "follows": boolean; "gifts": boolean; "joins": boolean; "likes": boolean; "moderation": boolean; "roomStats": boolean; "sendChat": boolean; "shares": boolean; "subscriptions": boolean };
  "name": string;
  "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick";
}

export type PlatformId = "tiktok" | "twitch" | "youtube" | "facebook" | "kick";

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

export type SelectionMode = "highest_score" | "uniform_random" | "weighted_random";

export interface SocialAutomationData {
  "action": number;
  "followCount": number;
  "isHistory": boolean;
  "method": string;
  "msgId": string;
  "shareCount": number;
}

export interface ViewerIdentity {
  "avatarUrl"?: string | null;
  "displayName": string;
  "handle": string;
  "id": string;
  "platform": "tiktok" | "twitch" | "youtube" | "facebook" | "kick";
}
