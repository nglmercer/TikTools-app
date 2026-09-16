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
  "intel"?: { "comment": { "composition": { "allCaps": boolean | null; "digits": number | null; "elongated": boolean | null; "emojiCount": number | null; "emojiOnly": boolean | null; "emojiRatio": number | null; "letters": number | null; "mentions": number | null; "repetitionScore": number | null; "urls": number | null } | null; "language": { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null; "normalized": string | null; "obfuscation": { "confusables": boolean | null; "detected": boolean | null; "leetspeak": boolean | null; "mixedScripts": boolean | null; "punctuationFlood": boolean | null; "repetition": boolean | null; "score": number | null } | null; "rebus": { "candidate": string | null; "confidence": number | null; "strong": boolean | null } | null; "spam": { "detected": boolean | null; "score": number | null } | null; "tts": { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null; "unicode": { "mixedScripts": boolean | null; "score": number | null; "suspicious": boolean | null } | null } | null; "processing": { "status": string } | null; "providers": JsonValue; "user": { "nickname": { "language": { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null; "normalized": string | null; "tts": { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null } | null } | null } | null;
  "points"?: { "delta": number; "level": number; "total": number } | null;
  "sourceEventId"?: string | null;
  "timestamp": number;
  "type": string;
  "user"?: { "nickname": string; "secUid": string; "uniqueId": string; "userId": string | null } | null;
}

export interface AutomationIntel {
  "comment"?: { "composition": { "allCaps": boolean | null; "digits": number | null; "elongated": boolean | null; "emojiCount": number | null; "emojiOnly": boolean | null; "emojiRatio": number | null; "letters": number | null; "mentions": number | null; "repetitionScore": number | null; "urls": number | null } | null; "language": { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null; "normalized": string | null; "obfuscation": { "confusables": boolean | null; "detected": boolean | null; "leetspeak": boolean | null; "mixedScripts": boolean | null; "punctuationFlood": boolean | null; "repetition": boolean | null; "score": number | null } | null; "rebus": { "candidate": string | null; "confidence": number | null; "strong": boolean | null } | null; "spam": { "detected": boolean | null; "score": number | null } | null; "tts": { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null; "unicode": { "mixedScripts": boolean | null; "score": number | null; "suspicious": boolean | null } | null } | null;
  "processing"?: { "status": string } | null;
  "providers"?: JsonValue;
  "user"?: { "nickname": { "language": { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null; "normalized": string | null; "tts": { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null } | null } | null;
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

export interface IntelComment {
  "composition"?: { "allCaps": boolean | null; "digits": number | null; "elongated": boolean | null; "emojiCount": number | null; "emojiOnly": boolean | null; "emojiRatio": number | null; "letters": number | null; "mentions": number | null; "repetitionScore": number | null; "urls": number | null } | null;
  "language"?: { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null;
  "normalized"?: string | null;
  "obfuscation"?: { "confusables": boolean | null; "detected": boolean | null; "leetspeak": boolean | null; "mixedScripts": boolean | null; "punctuationFlood": boolean | null; "repetition": boolean | null; "score": number | null } | null;
  "rebus"?: { "candidate": string | null; "confidence": number | null; "strong": boolean | null } | null;
  "spam"?: { "detected": boolean | null; "score": number | null } | null;
  "tts"?: { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null;
  "unicode"?: { "mixedScripts": boolean | null; "score": number | null; "suspicious": boolean | null } | null;
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

export interface IntelLanguage {
  "candidates"?: JsonValue[] | null;
  "confidence": number;
  "top": string;
}

export interface IntelLanguageCandidate {
  "confidence": number;
  "language": string;
}

export interface IntelNickname {
  "language"?: { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null;
  "normalized"?: string | null;
  "tts"?: { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null;
}

export interface IntelObfuscation {
  "confusables"?: boolean | null;
  "detected"?: boolean | null;
  "leetspeak"?: boolean | null;
  "mixedScripts"?: boolean | null;
  "punctuationFlood"?: boolean | null;
  "repetition"?: boolean | null;
  "score"?: number | null;
}

export interface IntelProcessing {
  "status": string;
}

export interface IntelRebus {
  "candidate"?: string | null;
  "confidence"?: number | null;
  "strong"?: boolean | null;
}

export interface IntelSpam {
  "detected"?: boolean | null;
  "score"?: number | null;
}

export interface IntelTts {
  "confidence"?: number | null;
  "ipa"?: string | null;
  "language"?: string | null;
  "source"?: string | null;
  "speak"?: boolean | null;
  "text": string;
}

export interface IntelUnicode {
  "mixedScripts"?: boolean | null;
  "score"?: number | null;
  "suspicious"?: boolean | null;
}

export interface IntelUser {
  "nickname"?: { "language": { "candidates": JsonValue[] | null; "confidence": number; "top": string } | null; "normalized": string | null; "tts": { "confidence": number | null; "ipa": string | null; "language": string | null; "source": string | null; "speak": boolean | null; "text": string } | null } | null;
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
