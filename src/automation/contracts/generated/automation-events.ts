// THIS FILE IS GENERATED. Run bun run contracts:generate.

import type { JsonValue } from './json-value.ts';

export interface AutomationCreator {
  "uniqueId": string;
  "roomId": string;
}

export interface AutomationEvent {
  "id": string;
  "type": string;
  "timestamp": number;
  "connectionId"?: string | null;
  "creator"?: { "uniqueId": string; "roomId": string } | null;
  "user"?: { "userId": string | null; "uniqueId": string; "nickname": string; "secUid": string; "avatarUrl": string | null } | null;
  "data": JsonValue;
  "points"?: { "delta": number; "total": number; "level": number } | null;
  "sourceEventId"?: string | null;
  "intel"?: { "comment": { "normalized": string | null; "nfc": string | null; "nfkc": string | null; "casefolded": string | null; "truncated": boolean; "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "composition": { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null; "unicode": { "mixedScripts": boolean | null; "suspicious": boolean | null; "score": number | null; "invisible": number | null; "bidirectional": number | null; "confusables": number | null } | null; "obfuscation": { "detected": boolean | null; "score": number | null; "leetspeak": boolean | null; "repetition": boolean | null; "punctuationFlood": boolean | null; "mixedScripts": boolean | null; "confusables": boolean | null; "flags": JsonValue[] | null } | null; "spam": { "score": number | null; "detected": boolean | null; "reasons": JsonValue[] | null; "model": string | null; "calibrated": boolean | null } | null; "rebus": { "candidate": string | null; "confidence": number | null; "score": number | null; "strong": boolean | null } | null; "tts": { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null } | null; "user": { "nickname": { "normalized": string | null; "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "tts": { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null } | null; "uniqueId": { "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "composition": { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null } | null } | null; "processing": { "status": string } | null; "providers": JsonValue } | null;
}

export interface AutomationPoints {
  "delta": number;
  "total": number;
  "level": number;
}

export interface AutomationUser {
  "userId"?: string | null;
  "uniqueId": string;
  "nickname": string;
  "secUid": string;
  "avatarUrl"?: string | null;
}

export interface ChatAutomationData {
  "comment": string;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}

export interface ConnectionAutomationData {
  "uniqueId": string;
  "roomId": string;
}

export interface EventIntel {
  "comment"?: { "normalized": string | null; "nfc": string | null; "nfkc": string | null; "casefolded": string | null; "truncated": boolean; "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "composition": { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null; "unicode": { "mixedScripts": boolean | null; "suspicious": boolean | null; "score": number | null; "invisible": number | null; "bidirectional": number | null; "confusables": number | null } | null; "obfuscation": { "detected": boolean | null; "score": number | null; "leetspeak": boolean | null; "repetition": boolean | null; "punctuationFlood": boolean | null; "mixedScripts": boolean | null; "confusables": boolean | null; "flags": JsonValue[] | null } | null; "spam": { "score": number | null; "detected": boolean | null; "reasons": JsonValue[] | null; "model": string | null; "calibrated": boolean | null } | null; "rebus": { "candidate": string | null; "confidence": number | null; "score": number | null; "strong": boolean | null } | null; "tts": { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null } | null;
  "user"?: { "nickname": { "normalized": string | null; "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "tts": { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null } | null; "uniqueId": { "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "composition": { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null } | null } | null;
  "processing"?: { "status": string } | null;
  "providers"?: JsonValue;
}

export interface GiftAutomationData {
  "giftId": string;
  "giftName": string;
  "diamondCount": number;
  "repeatCount": number;
  "comboCount": number;
  "groupId": string;
  "repeatEnd": boolean;
  "streakable": boolean;
  "giftIconUrl"?: string | null;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}

export interface IntelComment {
  "normalized"?: string | null;
  "nfc"?: string | null;
  "nfkc"?: string | null;
  "casefolded"?: string | null;
  "truncated"?: boolean;
  "language"?: { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null;
  "composition"?: { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null;
  "unicode"?: { "mixedScripts": boolean | null; "suspicious": boolean | null; "score": number | null; "invisible": number | null; "bidirectional": number | null; "confusables": number | null } | null;
  "obfuscation"?: { "detected": boolean | null; "score": number | null; "leetspeak": boolean | null; "repetition": boolean | null; "punctuationFlood": boolean | null; "mixedScripts": boolean | null; "confusables": boolean | null; "flags": JsonValue[] | null } | null;
  "spam"?: { "score": number | null; "detected": boolean | null; "reasons": JsonValue[] | null; "model": string | null; "calibrated": boolean | null } | null;
  "rebus"?: { "candidate": string | null; "confidence": number | null; "score": number | null; "strong": boolean | null } | null;
  "tts"?: { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null;
}

export interface IntelComposition {
  "emojiOnly"?: boolean | null;
  "emojiCount"?: number | null;
  "emojiRatio"?: number | null;
  "letters"?: number | null;
  "digits"?: number | null;
  "allCaps"?: boolean | null;
  "elongated"?: boolean | null;
  "repetitionScore"?: number | null;
  "urls"?: number | null;
  "mentions"?: number | null;
}

export interface IntelHandle {
  "language"?: { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null;
  "composition"?: { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null;
}

export interface IntelLanguage {
  "top": string;
  "confidence": number;
  "candidates"?: JsonValue[] | null;
}

export interface IntelLanguageCandidate {
  "language": string;
  "confidence": number;
}

export interface IntelNickname {
  "normalized"?: string | null;
  "language"?: { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null;
  "tts"?: { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null;
}

export interface IntelObfuscation {
  "detected"?: boolean | null;
  "score"?: number | null;
  "leetspeak"?: boolean | null;
  "repetition"?: boolean | null;
  "punctuationFlood"?: boolean | null;
  "mixedScripts"?: boolean | null;
  "confusables"?: boolean | null;
  "flags"?: JsonValue[] | null;
}

export interface IntelProcessing {
  "status": string;
}

export interface IntelPronunciation {
  "ipa"?: string | null;
  "language"?: string | null;
  "dialect"?: string | null;
  "confidence"?: number | null;
}

export interface IntelRebus {
  "candidate"?: string | null;
  "confidence"?: number | null;
  "score"?: number | null;
  "strong"?: boolean | null;
}

export interface IntelSpam {
  "score"?: number | null;
  "detected"?: boolean | null;
  "reasons"?: JsonValue[] | null;
  "model"?: string | null;
  "calibrated"?: boolean | null;
}

export interface IntelTts {
  "text": string;
  "language"?: string | null;
  "confidence"?: number | null;
  "source"?: string | null;
  "speak"?: boolean;
  "reason"?: string | null;
  "pronunciation"?: { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null;
}

export interface IntelUnicode {
  "mixedScripts"?: boolean | null;
  "suspicious"?: boolean | null;
  "score"?: number | null;
  "invisible"?: number | null;
  "bidirectional"?: number | null;
  "confusables"?: number | null;
}

export interface IntelUser {
  "nickname"?: { "normalized": string | null; "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "tts": { "text": string; "language": string | null; "confidence": number | null; "source": string | null; "speak": boolean; "reason": string | null; "pronunciation": { "ipa": string | null; "language": string | null; "dialect": string | null; "confidence": number | null } | null } | null } | null;
  "uniqueId"?: { "language": { "top": string; "confidence": number; "candidates": JsonValue[] | null } | null; "composition": { "emojiOnly": boolean | null; "emojiCount": number | null; "emojiRatio": number | null; "letters": number | null; "digits": number | null; "allCaps": boolean | null; "elongated": boolean | null; "repetitionScore": number | null; "urls": number | null; "mentions": number | null } | null } | null;
}

export interface LikeAutomationData {
  "count": number;
  "total": number;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}

export interface MemberAutomationData {
  "memberCount": number;
  "action": number;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}

export interface PluginEmitAutomationData {
  "emitType": string;
  "depth": number;
  "payload": JsonValue;
}

export interface PointsAwardedAutomationData {
  "uniqueId": string;
  "delta": number;
  "totalPoints": number;
  "level": number;
  "currencyName": string;
  "reason": string;
}

export interface RoomStatsAutomationData {
  "viewers": number;
  "totalUsers": number;
  "popularity": number;
  "anonymous": number;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}

export interface SocialAutomationData {
  "action": number;
  "followCount": number;
  "shareCount": number;
  "method": string;
  "msgId": string;
  "isHistory": boolean;
}
