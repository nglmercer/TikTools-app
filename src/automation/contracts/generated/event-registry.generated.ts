// THIS FILE IS GENERATED. Run bun scripts/task.ts contracts.

export const EVENT_REGISTRY_VERSION = 9 as const;

const SHARED_REGISTRY_1 = {
  "allCaps": false,
  "digits": 1,
  "elongated": false,
  "emojiCount": 1,
  "emojiOnly": false,
  "emojiRatio": 1,
  "letters": 1,
  "mentions": 1,
  "repetitionScore": 1,
  "urls": 1
} as const;

const SHARED_REGISTRY_2 = {
  "confusables": false,
  "detected": false,
  "flags": [],
  "leetspeak": false,
  "mixedScripts": false,
  "punctuationFlood": false,
  "repetition": false,
  "score": 1
} as const;

const SHARED_REGISTRY_3 = {
  "confidence": 1,
  "language": "sample",
  "pronunciation": {
    "confidence": 1,
    "dialect": "sample",
    "ipa": "sample",
    "language": "sample"
  },
  "reason": "sample",
  "source": "sample",
  "speak": false,
  "text": "sample"
} as const;

const SHARED_REGISTRY_4 = {
  "casefolded": "sample",
  "composition": SHARED_REGISTRY_1,
  "language": {
    "candidates": [],
    "confidence": 1,
    "top": "sample"
  },
  "nfc": "sample",
  "nfkc": "sample",
  "normalized": "sample",
  "obfuscation": SHARED_REGISTRY_2,
  "rebus": {
    "candidate": "sample",
    "confidence": 1,
    "score": 1,
    "strong": false
  },
  "spam": {
    "calibrated": false,
    "detected": false,
    "model": "sample",
    "reasons": [],
    "score": 1
  },
  "truncated": false,
  "tts": SHARED_REGISTRY_3,
  "unicode": {
    "bidirectional": 1,
    "confusables": 1,
    "invisible": 1,
    "mixedScripts": false,
    "score": 1,
    "suspicious": false
  }
} as const;

const SHARED_REGISTRY_5 = {
  "language": {
    "candidates": [],
    "confidence": 1,
    "top": "sample"
  },
  "normalized": "sample",
  "tts": SHARED_REGISTRY_3
} as const;

const SHARED_REGISTRY_6 = {
  "composition": SHARED_REGISTRY_1,
  "language": {
    "candidates": [],
    "confidence": 1,
    "top": "sample"
  }
} as const;

const SHARED_REGISTRY_7 = {
  "nickname": SHARED_REGISTRY_5,
  "uniqueId": SHARED_REGISTRY_6
} as const;

const SHARED_REGISTRY_8 = {
  "comment": SHARED_REGISTRY_4,
  "processing": {
    "status": "sample"
  },
  "providers": {},
  "user": SHARED_REGISTRY_7
} as const;

const SHARED_REGISTRY_9 = {
  "path": "event.user.avatarUrl",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Avatar Url",
    "es": "Avatar Url"
  },
  "hint": {
    "en": "AutomationUser.avatarUrl",
    "es": "AutomationUser.avatarUrl"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_10 = {
  "path": "event.user.nickname",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Nickname",
    "es": "Nickname"
  },
  "hint": {
    "en": "AutomationUser.nickname",
    "es": "AutomationUser.nickname"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_11 = {
  "path": "event.user.secUid",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Sec Uid",
    "es": "Sec Uid"
  },
  "hint": {
    "en": "AutomationUser.secUid",
    "es": "AutomationUser.secUid"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_12 = {
  "path": "event.user.uniqueId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Unique Id",
    "es": "Unique Id"
  },
  "hint": {
    "en": "AutomationUser.uniqueId",
    "es": "AutomationUser.uniqueId"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_13 = {
  "path": "event.user.userId",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "User Id",
    "es": "User Id"
  },
  "hint": {
    "en": "AutomationUser.userId",
    "es": "AutomationUser.userId"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_14 = {
  "path": "event.platform",
  "tsType": "PlatformId",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Platform",
    "es": "Plataforma"
  },
  "hint": {
    "en": "Source platform",
    "es": "Plataforma de origen"
  },
  "sample": "twitch"
} as const;

const SHARED_REGISTRY_15 = {
  "path": "event.intel.comment.casefolded",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Casefolded",
    "es": "Casefolded"
  },
  "hint": {
    "en": "EventIntel.comment.casefolded",
    "es": "EventIntel.comment.casefolded"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_16 = {
  "path": "event.intel.comment.composition.allCaps",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "All Caps",
    "es": "All Caps"
  },
  "hint": {
    "en": "EventIntel.comment.composition.allCaps",
    "es": "EventIntel.comment.composition.allCaps"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_17 = {
  "path": "event.intel.comment.composition.digits",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Digits",
    "es": "Digits"
  },
  "hint": {
    "en": "EventIntel.comment.composition.digits",
    "es": "EventIntel.comment.composition.digits"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_18 = {
  "path": "event.intel.comment.composition.elongated",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Elongated",
    "es": "Elongated"
  },
  "hint": {
    "en": "EventIntel.comment.composition.elongated",
    "es": "EventIntel.comment.composition.elongated"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_19 = {
  "path": "event.intel.comment.composition.emojiCount",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Emoji Count",
    "es": "Emoji Count"
  },
  "hint": {
    "en": "EventIntel.comment.composition.emojiCount",
    "es": "EventIntel.comment.composition.emojiCount"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_20 = {
  "path": "event.intel.comment.composition.emojiOnly",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Emoji Only",
    "es": "Emoji Only"
  },
  "hint": {
    "en": "EventIntel.comment.composition.emojiOnly",
    "es": "EventIntel.comment.composition.emojiOnly"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_21 = {
  "path": "event.intel.comment.composition.emojiRatio",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Emoji Ratio",
    "es": "Emoji Ratio"
  },
  "hint": {
    "en": "EventIntel.comment.composition.emojiRatio",
    "es": "EventIntel.comment.composition.emojiRatio"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_22 = {
  "path": "event.intel.comment.composition.letters",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Letters",
    "es": "Letters"
  },
  "hint": {
    "en": "EventIntel.comment.composition.letters",
    "es": "EventIntel.comment.composition.letters"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_23 = {
  "path": "event.intel.comment.composition.mentions",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Mentions",
    "es": "Mentions"
  },
  "hint": {
    "en": "EventIntel.comment.composition.mentions",
    "es": "EventIntel.comment.composition.mentions"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_24 = {
  "path": "event.intel.comment.composition.repetitionScore",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Repetition Score",
    "es": "Repetition Score"
  },
  "hint": {
    "en": "EventIntel.comment.composition.repetitionScore",
    "es": "EventIntel.comment.composition.repetitionScore"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_25 = {
  "path": "event.intel.comment.composition.urls",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Urls",
    "es": "Urls"
  },
  "hint": {
    "en": "EventIntel.comment.composition.urls",
    "es": "EventIntel.comment.composition.urls"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_26 = {
  "path": "event.intel.comment.language.candidates",
  "tsType": "(JsonValue)[] | null",
  "kind": "array",
  "optional": true,
  "label": {
    "en": "Candidates",
    "es": "Candidates"
  },
  "hint": {
    "en": "EventIntel.comment.language.candidates",
    "es": "EventIntel.comment.language.candidates"
  },
  "sample": []
} as const;

const SHARED_REGISTRY_27 = {
  "path": "event.intel.comment.language.confidence",
  "tsType": "number",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.comment.language.confidence",
    "es": "EventIntel.comment.language.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_28 = {
  "path": "event.intel.comment.language.top",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Top",
    "es": "Top"
  },
  "hint": {
    "en": "EventIntel.comment.language.top",
    "es": "EventIntel.comment.language.top"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_29 = {
  "path": "event.intel.comment.nfc",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Nfc",
    "es": "Nfc"
  },
  "hint": {
    "en": "EventIntel.comment.nfc",
    "es": "EventIntel.comment.nfc"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_30 = {
  "path": "event.intel.comment.nfkc",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Nfkc",
    "es": "Nfkc"
  },
  "hint": {
    "en": "EventIntel.comment.nfkc",
    "es": "EventIntel.comment.nfkc"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_31 = {
  "path": "event.intel.comment.normalized",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Normalized",
    "es": "Normalized"
  },
  "hint": {
    "en": "EventIntel.comment.normalized",
    "es": "EventIntel.comment.normalized"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_32 = {
  "path": "event.intel.comment.obfuscation.confusables",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Confusables",
    "es": "Confusables"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.confusables",
    "es": "EventIntel.comment.obfuscation.confusables"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_33 = {
  "path": "event.intel.comment.obfuscation.detected",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Detected",
    "es": "Detected"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.detected",
    "es": "EventIntel.comment.obfuscation.detected"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_34 = {
  "path": "event.intel.comment.obfuscation.flags",
  "tsType": "(JsonValue)[] | null",
  "kind": "array",
  "optional": true,
  "label": {
    "en": "Flags",
    "es": "Flags"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.flags",
    "es": "EventIntel.comment.obfuscation.flags"
  },
  "sample": []
} as const;

const SHARED_REGISTRY_35 = {
  "path": "event.intel.comment.obfuscation.leetspeak",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Leetspeak",
    "es": "Leetspeak"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.leetspeak",
    "es": "EventIntel.comment.obfuscation.leetspeak"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_36 = {
  "path": "event.intel.comment.obfuscation.mixedScripts",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Mixed Scripts",
    "es": "Mixed Scripts"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.mixedScripts",
    "es": "EventIntel.comment.obfuscation.mixedScripts"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_37 = {
  "path": "event.intel.comment.obfuscation.punctuationFlood",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Punctuation Flood",
    "es": "Punctuation Flood"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.punctuationFlood",
    "es": "EventIntel.comment.obfuscation.punctuationFlood"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_38 = {
  "path": "event.intel.comment.obfuscation.repetition",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Repetition",
    "es": "Repetition"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.repetition",
    "es": "EventIntel.comment.obfuscation.repetition"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_39 = {
  "path": "event.intel.comment.obfuscation.score",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Score",
    "es": "Score"
  },
  "hint": {
    "en": "EventIntel.comment.obfuscation.score",
    "es": "EventIntel.comment.obfuscation.score"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_40 = {
  "path": "event.intel.comment.rebus.candidate",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Candidate",
    "es": "Candidate"
  },
  "hint": {
    "en": "EventIntel.comment.rebus.candidate",
    "es": "EventIntel.comment.rebus.candidate"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_41 = {
  "path": "event.intel.comment.rebus.confidence",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.comment.rebus.confidence",
    "es": "EventIntel.comment.rebus.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_42 = {
  "path": "event.intel.comment.rebus.score",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Score",
    "es": "Score"
  },
  "hint": {
    "en": "EventIntel.comment.rebus.score",
    "es": "EventIntel.comment.rebus.score"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_43 = {
  "path": "event.intel.comment.rebus.strong",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Strong",
    "es": "Strong"
  },
  "hint": {
    "en": "EventIntel.comment.rebus.strong",
    "es": "EventIntel.comment.rebus.strong"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_44 = {
  "path": "event.intel.comment.spam.calibrated",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Calibrated",
    "es": "Calibrated"
  },
  "hint": {
    "en": "EventIntel.comment.spam.calibrated",
    "es": "EventIntel.comment.spam.calibrated"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_45 = {
  "path": "event.intel.comment.spam.detected",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Detected",
    "es": "Detected"
  },
  "hint": {
    "en": "EventIntel.comment.spam.detected",
    "es": "EventIntel.comment.spam.detected"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_46 = {
  "path": "event.intel.comment.spam.model",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Model",
    "es": "Model"
  },
  "hint": {
    "en": "EventIntel.comment.spam.model",
    "es": "EventIntel.comment.spam.model"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_47 = {
  "path": "event.intel.comment.spam.reasons",
  "tsType": "(JsonValue)[] | null",
  "kind": "array",
  "optional": true,
  "label": {
    "en": "Reasons",
    "es": "Reasons"
  },
  "hint": {
    "en": "EventIntel.comment.spam.reasons",
    "es": "EventIntel.comment.spam.reasons"
  },
  "sample": []
} as const;

const SHARED_REGISTRY_48 = {
  "path": "event.intel.comment.spam.score",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Score",
    "es": "Score"
  },
  "hint": {
    "en": "EventIntel.comment.spam.score",
    "es": "EventIntel.comment.spam.score"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_49 = {
  "path": "event.intel.comment.truncated",
  "tsType": "boolean",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Truncated",
    "es": "Truncated"
  },
  "hint": {
    "en": "EventIntel.comment.truncated",
    "es": "EventIntel.comment.truncated"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_50 = {
  "path": "event.intel.comment.tts.confidence",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.comment.tts.confidence",
    "es": "EventIntel.comment.tts.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_51 = {
  "path": "event.intel.comment.tts.language",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Language",
    "es": "Language"
  },
  "hint": {
    "en": "EventIntel.comment.tts.language",
    "es": "EventIntel.comment.tts.language"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_52 = {
  "path": "event.intel.comment.tts.pronunciation.confidence",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.comment.tts.pronunciation.confidence",
    "es": "EventIntel.comment.tts.pronunciation.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_53 = {
  "path": "event.intel.comment.tts.pronunciation.dialect",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Dialect",
    "es": "Dialect"
  },
  "hint": {
    "en": "EventIntel.comment.tts.pronunciation.dialect",
    "es": "EventIntel.comment.tts.pronunciation.dialect"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_54 = {
  "path": "event.intel.comment.tts.pronunciation.ipa",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Ipa",
    "es": "Ipa"
  },
  "hint": {
    "en": "EventIntel.comment.tts.pronunciation.ipa",
    "es": "EventIntel.comment.tts.pronunciation.ipa"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_55 = {
  "path": "event.intel.comment.tts.pronunciation.language",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Language",
    "es": "Language"
  },
  "hint": {
    "en": "EventIntel.comment.tts.pronunciation.language",
    "es": "EventIntel.comment.tts.pronunciation.language"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_56 = {
  "path": "event.intel.comment.tts.reason",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Reason",
    "es": "Reason"
  },
  "hint": {
    "en": "EventIntel.comment.tts.reason",
    "es": "EventIntel.comment.tts.reason"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_57 = {
  "path": "event.intel.comment.tts.source",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Source",
    "es": "Source"
  },
  "hint": {
    "en": "EventIntel.comment.tts.source",
    "es": "EventIntel.comment.tts.source"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_58 = {
  "path": "event.intel.comment.tts.speak",
  "tsType": "boolean",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Speak",
    "es": "Speak"
  },
  "hint": {
    "en": "EventIntel.comment.tts.speak",
    "es": "EventIntel.comment.tts.speak"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_59 = {
  "path": "event.intel.comment.tts.text",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Text",
    "es": "Text"
  },
  "hint": {
    "en": "EventIntel.comment.tts.text",
    "es": "EventIntel.comment.tts.text"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_60 = {
  "path": "event.intel.comment.unicode.bidirectional",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Bidirectional",
    "es": "Bidirectional"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.bidirectional",
    "es": "EventIntel.comment.unicode.bidirectional"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_61 = {
  "path": "event.intel.comment.unicode.confusables",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confusables",
    "es": "Confusables"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.confusables",
    "es": "EventIntel.comment.unicode.confusables"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_62 = {
  "path": "event.intel.comment.unicode.invisible",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Invisible",
    "es": "Invisible"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.invisible",
    "es": "EventIntel.comment.unicode.invisible"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_63 = {
  "path": "event.intel.comment.unicode.mixedScripts",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Mixed Scripts",
    "es": "Mixed Scripts"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.mixedScripts",
    "es": "EventIntel.comment.unicode.mixedScripts"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_64 = {
  "path": "event.intel.comment.unicode.score",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Score",
    "es": "Score"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.score",
    "es": "EventIntel.comment.unicode.score"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_65 = {
  "path": "event.intel.comment.unicode.suspicious",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Suspicious",
    "es": "Suspicious"
  },
  "hint": {
    "en": "EventIntel.comment.unicode.suspicious",
    "es": "EventIntel.comment.unicode.suspicious"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_66 = {
  "path": "event.intel.processing.status",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Status",
    "es": "Status"
  },
  "hint": {
    "en": "EventIntel.processing.status",
    "es": "EventIntel.processing.status"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_67 = {
  "path": "event.intel.user.nickname.language.candidates",
  "tsType": "(JsonValue)[] | null",
  "kind": "array",
  "optional": true,
  "label": {
    "en": "Candidates",
    "es": "Candidates"
  },
  "hint": {
    "en": "EventIntel.user.nickname.language.candidates",
    "es": "EventIntel.user.nickname.language.candidates"
  },
  "sample": []
} as const;

const SHARED_REGISTRY_68 = {
  "path": "event.intel.user.nickname.language.confidence",
  "tsType": "number",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.user.nickname.language.confidence",
    "es": "EventIntel.user.nickname.language.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_69 = {
  "path": "event.intel.user.nickname.language.top",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Top",
    "es": "Top"
  },
  "hint": {
    "en": "EventIntel.user.nickname.language.top",
    "es": "EventIntel.user.nickname.language.top"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_70 = {
  "path": "event.intel.user.nickname.normalized",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Normalized",
    "es": "Normalized"
  },
  "hint": {
    "en": "EventIntel.user.nickname.normalized",
    "es": "EventIntel.user.nickname.normalized"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_71 = {
  "path": "event.intel.user.nickname.tts.confidence",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.confidence",
    "es": "EventIntel.user.nickname.tts.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_72 = {
  "path": "event.intel.user.nickname.tts.language",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Language",
    "es": "Language"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.language",
    "es": "EventIntel.user.nickname.tts.language"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_73 = {
  "en": "EventIntel.user.nickname.tts.pronunciation.confidence",
  "es": "EventIntel.user.nickname.tts.pronunciation.confidence"
} as const;

const SHARED_REGISTRY_74 = {
  "path": "event.intel.user.nickname.tts.pronunciation.confidence",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": SHARED_REGISTRY_73,
  "sample": 1
} as const;

const SHARED_REGISTRY_75 = {
  "path": "event.intel.user.nickname.tts.pronunciation.dialect",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Dialect",
    "es": "Dialect"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.pronunciation.dialect",
    "es": "EventIntel.user.nickname.tts.pronunciation.dialect"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_76 = {
  "path": "event.intel.user.nickname.tts.pronunciation.ipa",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Ipa",
    "es": "Ipa"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.pronunciation.ipa",
    "es": "EventIntel.user.nickname.tts.pronunciation.ipa"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_77 = {
  "path": "event.intel.user.nickname.tts.pronunciation.language",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Language",
    "es": "Language"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.pronunciation.language",
    "es": "EventIntel.user.nickname.tts.pronunciation.language"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_78 = {
  "path": "event.intel.user.nickname.tts.reason",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Reason",
    "es": "Reason"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.reason",
    "es": "EventIntel.user.nickname.tts.reason"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_79 = {
  "path": "event.intel.user.nickname.tts.source",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Source",
    "es": "Source"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.source",
    "es": "EventIntel.user.nickname.tts.source"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_80 = {
  "path": "event.intel.user.nickname.tts.speak",
  "tsType": "boolean",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Speak",
    "es": "Speak"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.speak",
    "es": "EventIntel.user.nickname.tts.speak"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_81 = {
  "path": "event.intel.user.nickname.tts.text",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Text",
    "es": "Text"
  },
  "hint": {
    "en": "EventIntel.user.nickname.tts.text",
    "es": "EventIntel.user.nickname.tts.text"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_82 = {
  "path": "event.intel.user.uniqueId.composition.allCaps",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "All Caps",
    "es": "All Caps"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.allCaps",
    "es": "EventIntel.user.uniqueId.composition.allCaps"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_83 = {
  "path": "event.intel.user.uniqueId.composition.digits",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Digits",
    "es": "Digits"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.digits",
    "es": "EventIntel.user.uniqueId.composition.digits"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_84 = {
  "path": "event.intel.user.uniqueId.composition.elongated",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Elongated",
    "es": "Elongated"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.elongated",
    "es": "EventIntel.user.uniqueId.composition.elongated"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_85 = {
  "path": "event.intel.user.uniqueId.composition.emojiCount",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Emoji Count",
    "es": "Emoji Count"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.emojiCount",
    "es": "EventIntel.user.uniqueId.composition.emojiCount"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_86 = {
  "path": "event.intel.user.uniqueId.composition.emojiOnly",
  "tsType": "boolean | null",
  "kind": "boolean",
  "optional": true,
  "label": {
    "en": "Emoji Only",
    "es": "Emoji Only"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.emojiOnly",
    "es": "EventIntel.user.uniqueId.composition.emojiOnly"
  },
  "sample": false
} as const;

const SHARED_REGISTRY_87 = {
  "path": "event.intel.user.uniqueId.composition.emojiRatio",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Emoji Ratio",
    "es": "Emoji Ratio"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.emojiRatio",
    "es": "EventIntel.user.uniqueId.composition.emojiRatio"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_88 = {
  "path": "event.intel.user.uniqueId.composition.letters",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Letters",
    "es": "Letters"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.letters",
    "es": "EventIntel.user.uniqueId.composition.letters"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_89 = {
  "path": "event.intel.user.uniqueId.composition.mentions",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Mentions",
    "es": "Mentions"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.mentions",
    "es": "EventIntel.user.uniqueId.composition.mentions"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_90 = {
  "en": "EventIntel.user.uniqueId.composition.repetitionScore",
  "es": "EventIntel.user.uniqueId.composition.repetitionScore"
} as const;

const SHARED_REGISTRY_91 = {
  "path": "event.intel.user.uniqueId.composition.repetitionScore",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Repetition Score",
    "es": "Repetition Score"
  },
  "hint": SHARED_REGISTRY_90,
  "sample": 1
} as const;

const SHARED_REGISTRY_92 = {
  "path": "event.intel.user.uniqueId.composition.urls",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Urls",
    "es": "Urls"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.composition.urls",
    "es": "EventIntel.user.uniqueId.composition.urls"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_93 = {
  "path": "event.intel.user.uniqueId.language.candidates",
  "tsType": "(JsonValue)[] | null",
  "kind": "array",
  "optional": true,
  "label": {
    "en": "Candidates",
    "es": "Candidates"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.language.candidates",
    "es": "EventIntel.user.uniqueId.language.candidates"
  },
  "sample": []
} as const;

const SHARED_REGISTRY_94 = {
  "path": "event.intel.user.uniqueId.language.confidence",
  "tsType": "number",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Confidence",
    "es": "Confidence"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.language.confidence",
    "es": "EventIntel.user.uniqueId.language.confidence"
  },
  "sample": 1
} as const;

const SHARED_REGISTRY_95 = {
  "path": "event.intel.user.uniqueId.language.top",
  "tsType": "string",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Top",
    "es": "Top"
  },
  "hint": {
    "en": "EventIntel.user.uniqueId.language.top",
    "es": "EventIntel.user.uniqueId.language.top"
  },
  "sample": "sample"
} as const;

const SHARED_REGISTRY_96 = {
  "name": "emotes",
  "tsType": "({ \"emoteType\"?: number | null; \"id\": string; \"imageUrl\"?: string | null; \"index\"?: number | null; \"packageId\"?: string | null; \"shortcode\"?: string | null })[]",
  "optional": true
} as const;

const SHARED_REGISTRY_97 = [
  {
    "name": "comment",
    "tsType": "string",
    "optional": false
  },
  SHARED_REGISTRY_96,
  {
    "name": "isHistory",
    "tsType": "boolean",
    "optional": false
  },
  {
    "name": "method",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "msgId",
    "tsType": "string",
    "optional": false
  }
] as const;

const SHARED_REGISTRY_98 = {
  "path": "event.data.count",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Count",
    "es": "Count"
  },
  "hint": {
    "en": "LiveActivityAutomationData.count",
    "es": "LiveActivityAutomationData.count"
  },
  "sample": 1,
  "sourceField": "count"
} as const;

const SHARED_REGISTRY_99 = {
  "path": "event.data.isHistory",
  "tsType": "boolean",
  "kind": "boolean",
  "optional": false,
  "label": {
    "en": "Is History",
    "es": "Is History"
  },
  "hint": {
    "en": "LiveActivityAutomationData.isHistory",
    "es": "LiveActivityAutomationData.isHistory"
  },
  "sample": false,
  "sourceField": "isHistory"
} as const;

const SHARED_REGISTRY_100 = {
  "path": "event.data.msgId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Msg Id",
    "es": "Msg Id"
  },
  "hint": {
    "en": "LiveActivityAutomationData.msgId",
    "es": "LiveActivityAutomationData.msgId"
  },
  "sample": "1",
  "sourceField": "msgId"
} as const;

const SHARED_REGISTRY_101 = {
  "path": "event.data.total",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Total",
    "es": "Total"
  },
  "hint": {
    "en": "LiveActivityAutomationData.total",
    "es": "LiveActivityAutomationData.total"
  },
  "sample": 1,
  "sourceField": "total"
} as const;

const SHARED_REGISTRY_102 = [
  {
    "name": "count",
    "tsType": "number | null",
    "optional": true
  },
  {
    "name": "isHistory",
    "tsType": "boolean",
    "optional": false
  },
  {
    "name": "method",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "msgId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "total",
    "tsType": "number | null",
    "optional": true
  }
] as const;

const SHARED_REGISTRY_103 = {
  "path": "event.data.followCount",
  "tsType": "number",
  "kind": "number",
  "optional": false,
  "label": {
    "en": "Follow Count",
    "es": "Follow Count"
  },
  "hint": {
    "en": "SocialAutomationData.followCount",
    "es": "SocialAutomationData.followCount"
  },
  "sample": 1,
  "sourceField": "followCount",
  "sourceMethod": "WebcastSocialMessage",
  "sourcePath": "follow_count",
  "sourceTransform": "normalized-unsigned",
  "sourceJsonPath": "followCount",
  "sourceProtoType": "int64",
  "sourceCardinality": "optional"
} as const;

const SHARED_REGISTRY_104 = {
  "path": "event.data.isHistory",
  "tsType": "boolean",
  "kind": "boolean",
  "optional": false,
  "label": {
    "en": "Is History",
    "es": "Is History"
  },
  "hint": {
    "en": "SocialAutomationData.isHistory",
    "es": "SocialAutomationData.isHistory"
  },
  "sample": false,
  "sourceField": "isHistory"
} as const;

const SHARED_REGISTRY_105 = {
  "path": "event.data.method",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Method",
    "es": "Method"
  },
  "hint": {
    "en": "SocialAutomationData.method",
    "es": "SocialAutomationData.method"
  },
  "sample": "WebcastSampleMessage",
  "sourceField": "method"
} as const;

const SHARED_REGISTRY_106 = {
  "path": "event.data.msgId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Msg Id",
    "es": "Msg Id"
  },
  "hint": {
    "en": "SocialAutomationData.msgId",
    "es": "SocialAutomationData.msgId"
  },
  "sample": "1",
  "sourceField": "msgId"
} as const;

const SHARED_REGISTRY_107 = {
  "path": "event.data.shareCount",
  "tsType": "number",
  "kind": "number",
  "optional": false,
  "label": {
    "en": "Share Count",
    "es": "Share Count"
  },
  "hint": {
    "en": "SocialAutomationData.shareCount",
    "es": "SocialAutomationData.shareCount"
  },
  "sample": 1,
  "sourceField": "shareCount",
  "sourceMethod": "WebcastSocialMessage",
  "sourcePath": "share_count",
  "sourceTransform": "normalized-unsigned",
  "sourceJsonPath": "shareCount",
  "sourceProtoType": "int32",
  "sourceCardinality": "optional"
} as const;

const SHARED_REGISTRY_108 = [
  {
    "name": "action",
    "tsType": "number",
    "optional": false
  },
  {
    "name": "followCount",
    "tsType": "number",
    "optional": false
  },
  {
    "name": "isHistory",
    "tsType": "boolean",
    "optional": false
  },
  {
    "name": "method",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "msgId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "shareCount",
    "tsType": "number",
    "optional": false
  }
] as const;

const SHARED_REGISTRY_109 = {
  "path": "event.data.roomId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Room Id",
    "es": "Room Id"
  },
  "hint": {
    "en": "ConnectionAutomationData.roomId",
    "es": "ConnectionAutomationData.roomId"
  },
  "sample": "sample",
  "sourceField": "roomId"
} as const;

const SHARED_REGISTRY_110 = {
  "path": "event.data.uniqueId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Unique Id",
    "es": "Unique Id"
  },
  "hint": {
    "en": "ConnectionAutomationData.uniqueId",
    "es": "ConnectionAutomationData.uniqueId"
  },
  "sample": "sample",
  "sourceField": "uniqueId"
} as const;

const SHARED_REGISTRY_111 = [
  SHARED_REGISTRY_109,
  SHARED_REGISTRY_110,
  SHARED_REGISTRY_66
] as const;

const SHARED_REGISTRY_112 = {
  "actionId": "sample",
  "actionStatus": "pending",
  "competitionId": "sample",
  "creatorId": "sample",
  "generation": 1,
  "optionId": "sample",
  "rankingId": "sample",
  "roomId": "sample",
  "selectionId": "sample",
  "selectionMode": "highest_score",
  "winnerId": "sample",
  "winnerScore": 1
} as const;

const SHARED_REGISTRY_113 = {
  "path": "event.data.actionId",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Action Id",
    "es": "Action Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.actionId",
    "es": "CompetitionAutomationData.actionId"
  },
  "sample": "sample",
  "sourceField": "actionId"
} as const;

const SHARED_REGISTRY_114 = {
  "path": "event.data.actionStatus",
  "tsType": "\"pending\" | \"running\" | \"completed\" | \"failed\" | \"uncertain\" | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Action Status",
    "es": "Action Status"
  },
  "hint": {
    "en": "CompetitionAutomationData.actionStatus",
    "es": "CompetitionAutomationData.actionStatus"
  },
  "sample": "pending",
  "sourceField": "actionStatus"
} as const;

const SHARED_REGISTRY_115 = {
  "path": "event.data.competitionId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Competition Id",
    "es": "Competition Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.competitionId",
    "es": "CompetitionAutomationData.competitionId"
  },
  "sample": "sample",
  "sourceField": "competitionId"
} as const;

const SHARED_REGISTRY_116 = {
  "path": "event.data.creatorId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Creator Id",
    "es": "Creator Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.creatorId",
    "es": "CompetitionAutomationData.creatorId"
  },
  "sample": "sample",
  "sourceField": "creatorId"
} as const;

const SHARED_REGISTRY_117 = {
  "path": "event.data.generation",
  "tsType": "number",
  "kind": "number",
  "optional": false,
  "label": {
    "en": "Generation",
    "es": "Generation"
  },
  "hint": {
    "en": "CompetitionAutomationData.generation",
    "es": "CompetitionAutomationData.generation"
  },
  "sample": 1,
  "sourceField": "generation"
} as const;

const SHARED_REGISTRY_118 = {
  "path": "event.data.optionId",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Option Id",
    "es": "Option Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.optionId",
    "es": "CompetitionAutomationData.optionId"
  },
  "sample": "sample",
  "sourceField": "optionId"
} as const;

const SHARED_REGISTRY_119 = {
  "path": "event.data.rankingId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Ranking Id",
    "es": "Ranking Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.rankingId",
    "es": "CompetitionAutomationData.rankingId"
  },
  "sample": "sample",
  "sourceField": "rankingId"
} as const;

const SHARED_REGISTRY_120 = {
  "path": "event.data.roomId",
  "tsType": "string",
  "kind": "string",
  "optional": false,
  "label": {
    "en": "Room Id",
    "es": "Room Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.roomId",
    "es": "CompetitionAutomationData.roomId"
  },
  "sample": "sample",
  "sourceField": "roomId"
} as const;

const SHARED_REGISTRY_121 = {
  "path": "event.data.selectionId",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Selection Id",
    "es": "Selection Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.selectionId",
    "es": "CompetitionAutomationData.selectionId"
  },
  "sample": "sample",
  "sourceField": "selectionId"
} as const;

const SHARED_REGISTRY_122 = {
  "path": "event.data.selectionMode",
  "tsType": "\"highest_score\" | \"uniform_random\" | \"weighted_random\" | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Selection Mode",
    "es": "Selection Mode"
  },
  "hint": {
    "en": "CompetitionAutomationData.selectionMode",
    "es": "CompetitionAutomationData.selectionMode"
  },
  "sample": "highest_score",
  "sourceField": "selectionMode"
} as const;

const SHARED_REGISTRY_123 = {
  "path": "event.data.winnerId",
  "tsType": "string | null",
  "kind": "string",
  "optional": true,
  "label": {
    "en": "Winner Id",
    "es": "Winner Id"
  },
  "hint": {
    "en": "CompetitionAutomationData.winnerId",
    "es": "CompetitionAutomationData.winnerId"
  },
  "sample": "sample",
  "sourceField": "winnerId"
} as const;

const SHARED_REGISTRY_124 = {
  "path": "event.data.winnerScore",
  "tsType": "number | null",
  "kind": "number",
  "optional": true,
  "label": {
    "en": "Winner Score",
    "es": "Winner Score"
  },
  "hint": {
    "en": "CompetitionAutomationData.winnerScore",
    "es": "CompetitionAutomationData.winnerScore"
  },
  "sample": 1,
  "sourceField": "winnerScore"
} as const;

const SHARED_REGISTRY_125 = [
  SHARED_REGISTRY_113,
  SHARED_REGISTRY_114,
  SHARED_REGISTRY_115,
  SHARED_REGISTRY_116,
  SHARED_REGISTRY_117,
  SHARED_REGISTRY_118,
  SHARED_REGISTRY_119,
  SHARED_REGISTRY_120,
  SHARED_REGISTRY_121,
  SHARED_REGISTRY_122,
  SHARED_REGISTRY_123,
  SHARED_REGISTRY_124,
  SHARED_REGISTRY_66
] as const;

const SHARED_REGISTRY_126 = {
  "name": "actionStatus",
  "tsType": "\"pending\" | \"running\" | \"completed\" | \"failed\" | \"uncertain\" | null",
  "optional": true
} as const;

const SHARED_REGISTRY_127 = [
  {
    "name": "actionId",
    "tsType": "string | null",
    "optional": true
  },
  SHARED_REGISTRY_126,
  {
    "name": "competitionId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "creatorId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "generation",
    "tsType": "number",
    "optional": false
  },
  {
    "name": "optionId",
    "tsType": "string | null",
    "optional": true
  },
  {
    "name": "rankingId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "roomId",
    "tsType": "string",
    "optional": false
  },
  {
    "name": "selectionId",
    "tsType": "string | null",
    "optional": true
  },
  {
    "name": "selectionMode",
    "tsType": "\"highest_score\" | \"uniform_random\" | \"weighted_random\" | null",
    "optional": true
  },
  {
    "name": "winnerId",
    "tsType": "string | null",
    "optional": true
  },
  {
    "name": "winnerScore",
    "tsType": "number | null",
    "optional": true
  }
] as const;

export const GENERATED_EVENT_REGISTRY = {
  "version": 9,
  "generatedBy": "tiktools-core automation contracts",
  "generatedFrom": [
    "crates/tiktools-core/src/contracts",
    "src/automation/contracts/generated/automation-events.schema.json"
  ],
  "events": {
    "live.chat": {
      "dataInterface": "LiveChatAutomationData",
      "sourceInterface": "LiveChatAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.chat",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "comment": "Hello there",
          "emotes": [],
          "isHistory": false,
          "method": "live.chat",
          "msgId": "1"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        {
          "path": "event.data.comment",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Comment",
            "es": "Comment"
          },
          "hint": {
            "en": "LiveChatAutomationData.comment",
            "es": "LiveChatAutomationData.comment"
          },
          "sample": "Hello there",
          "sourceField": "comment"
        },
        {
          "path": "event.data.emotes",
          "tsType": "({ \"emoteType\"?: number | null; \"id\": string; \"imageUrl\"?: string | null; \"index\"?: number | null; \"packageId\"?: string | null; \"shortcode\"?: string | null })[]",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Emotes",
            "es": "Emotes"
          },
          "hint": {
            "en": "LiveChatAutomationData.emotes",
            "es": "LiveChatAutomationData.emotes"
          },
          "sample": [],
          "sourceField": "emotes"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "LiveChatAutomationData.isHistory",
            "es": "LiveChatAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveChatAutomationData.method",
            "es": "LiveChatAutomationData.method"
          },
          "sample": "live.chat",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "LiveChatAutomationData.msgId",
            "es": "LiveChatAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        SHARED_REGISTRY_15,
        SHARED_REGISTRY_16,
        SHARED_REGISTRY_17,
        SHARED_REGISTRY_18,
        SHARED_REGISTRY_19,
        SHARED_REGISTRY_20,
        SHARED_REGISTRY_21,
        SHARED_REGISTRY_22,
        SHARED_REGISTRY_23,
        SHARED_REGISTRY_24,
        SHARED_REGISTRY_25,
        SHARED_REGISTRY_26,
        SHARED_REGISTRY_27,
        SHARED_REGISTRY_28,
        SHARED_REGISTRY_29,
        SHARED_REGISTRY_30,
        SHARED_REGISTRY_31,
        SHARED_REGISTRY_32,
        SHARED_REGISTRY_33,
        SHARED_REGISTRY_34,
        SHARED_REGISTRY_35,
        SHARED_REGISTRY_36,
        SHARED_REGISTRY_37,
        SHARED_REGISTRY_38,
        SHARED_REGISTRY_39,
        SHARED_REGISTRY_40,
        SHARED_REGISTRY_41,
        SHARED_REGISTRY_42,
        SHARED_REGISTRY_43,
        SHARED_REGISTRY_44,
        SHARED_REGISTRY_45,
        SHARED_REGISTRY_46,
        SHARED_REGISTRY_47,
        SHARED_REGISTRY_48,
        SHARED_REGISTRY_49,
        SHARED_REGISTRY_50,
        SHARED_REGISTRY_51,
        SHARED_REGISTRY_52,
        SHARED_REGISTRY_53,
        SHARED_REGISTRY_54,
        SHARED_REGISTRY_55,
        SHARED_REGISTRY_56,
        SHARED_REGISTRY_57,
        SHARED_REGISTRY_58,
        SHARED_REGISTRY_59,
        SHARED_REGISTRY_60,
        SHARED_REGISTRY_61,
        SHARED_REGISTRY_62,
        SHARED_REGISTRY_63,
        SHARED_REGISTRY_64,
        SHARED_REGISTRY_65,
        SHARED_REGISTRY_66,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_82,
        SHARED_REGISTRY_83,
        SHARED_REGISTRY_84,
        SHARED_REGISTRY_85,
        SHARED_REGISTRY_86,
        SHARED_REGISTRY_87,
        SHARED_REGISTRY_88,
        SHARED_REGISTRY_89,
        SHARED_REGISTRY_91,
        SHARED_REGISTRY_92,
        SHARED_REGISTRY_93,
        SHARED_REGISTRY_94,
        SHARED_REGISTRY_95
      ],
      "sourceFields": SHARED_REGISTRY_97,
      "note": "Generated from LiveChatAutomationData JSON Schema."
    },
    "live.follow": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.follow",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.follow",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.follow",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "live.subscription": {
      "dataInterface": "LiveSubscriptionAutomationData",
      "sourceInterface": "LiveSubscriptionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.subscription",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "gifted": false,
          "isHistory": false,
          "method": "live.subscription",
          "months": 1,
          "msgId": "1",
          "tier": "sample"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        {
          "path": "event.data.gifted",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Gifted",
            "es": "Gifted"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.gifted",
            "es": "LiveSubscriptionAutomationData.gifted"
          },
          "sample": false,
          "sourceField": "gifted"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.isHistory",
            "es": "LiveSubscriptionAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.method",
            "es": "LiveSubscriptionAutomationData.method"
          },
          "sample": "live.subscription",
          "sourceField": "method"
        },
        {
          "path": "event.data.months",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Months",
            "es": "Months"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.months",
            "es": "LiveSubscriptionAutomationData.months"
          },
          "sample": 1,
          "sourceField": "months"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.msgId",
            "es": "LiveSubscriptionAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.tier",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Tier",
            "es": "Tier"
          },
          "hint": {
            "en": "LiveSubscriptionAutomationData.tier",
            "es": "LiveSubscriptionAutomationData.tier"
          },
          "sample": "sample",
          "sourceField": "tier"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "gifted",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "months",
          "tsType": "number | null",
          "optional": true
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "tier",
          "tsType": "string | null",
          "optional": true
        }
      ],
      "note": "Generated from LiveSubscriptionAutomationData JSON Schema."
    },
    "live.gift": {
      "dataInterface": "LiveGiftAutomationData",
      "sourceInterface": "LiveGiftAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.gift",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "amount": "100",
          "count": 1,
          "id": "sample",
          "isHistory": false,
          "method": "live.gift",
          "msgId": "1",
          "name": "sample",
          "unit": "twitch_bits"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        {
          "path": "event.data.amount",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Amount",
            "es": "Amount"
          },
          "hint": {
            "en": "LiveGiftAutomationData.amount",
            "es": "LiveGiftAutomationData.amount"
          },
          "sample": "100",
          "sourceField": "amount"
        },
        {
          "path": "event.data.count",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Count",
            "es": "Count"
          },
          "hint": {
            "en": "LiveGiftAutomationData.count",
            "es": "LiveGiftAutomationData.count"
          },
          "sample": 1,
          "sourceField": "count"
        },
        {
          "path": "event.data.id",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Id",
            "es": "Id"
          },
          "hint": {
            "en": "LiveGiftAutomationData.id",
            "es": "LiveGiftAutomationData.id"
          },
          "sample": "sample",
          "sourceField": "id"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "LiveGiftAutomationData.isHistory",
            "es": "LiveGiftAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveGiftAutomationData.method",
            "es": "LiveGiftAutomationData.method"
          },
          "sample": "live.gift",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "LiveGiftAutomationData.msgId",
            "es": "LiveGiftAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.name",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Name",
            "es": "Name"
          },
          "hint": {
            "en": "LiveGiftAutomationData.name",
            "es": "LiveGiftAutomationData.name"
          },
          "sample": "sample",
          "sourceField": "name"
        },
        {
          "path": "event.data.unit",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Unit",
            "es": "Unit"
          },
          "hint": {
            "en": "LiveGiftAutomationData.unit",
            "es": "LiveGiftAutomationData.unit"
          },
          "sample": "twitch_bits",
          "sourceField": "unit"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "amount",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "count",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "id",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "name",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "unit",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from LiveGiftAutomationData JSON Schema."
    },
    "live.like": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.like",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.like",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.like",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "live.share": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.share",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.share",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.share",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "live.join": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.join",
        "timestamp": 0,
        "platform": "twitch",
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.join",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.join",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "live.room_stats": {
      "dataInterface": "LiveRoomStatsAutomationData",
      "sourceInterface": "LiveRoomStatsAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.room_stats",
        "timestamp": 0,
        "platform": "twitch",
        "data": {
          "isHistory": false,
          "method": "live.room_stats",
          "msgId": "1",
          "totalUsers": 1,
          "viewers": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_14,
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "LiveRoomStatsAutomationData.isHistory",
            "es": "LiveRoomStatsAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveRoomStatsAutomationData.method",
            "es": "LiveRoomStatsAutomationData.method"
          },
          "sample": "live.room_stats",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "LiveRoomStatsAutomationData.msgId",
            "es": "LiveRoomStatsAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.totalUsers",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Total Users",
            "es": "Total Users"
          },
          "hint": {
            "en": "LiveRoomStatsAutomationData.totalUsers",
            "es": "LiveRoomStatsAutomationData.totalUsers"
          },
          "sample": 1,
          "sourceField": "totalUsers"
        },
        {
          "path": "event.data.viewers",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Viewers",
            "es": "Viewers"
          },
          "hint": {
            "en": "LiveRoomStatsAutomationData.viewers",
            "es": "LiveRoomStatsAutomationData.viewers"
          },
          "sample": 1,
          "sourceField": "viewers"
        },
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "totalUsers",
          "tsType": "number | null",
          "optional": true
        },
        {
          "name": "viewers",
          "tsType": "number",
          "optional": false
        }
      ],
      "note": "Generated from LiveRoomStatsAutomationData JSON Schema."
    },
    "live.connected": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.connected",
        "timestamp": 0,
        "platform": "twitch",
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.connected",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.connected",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "live.disconnected": {
      "dataInterface": "LiveActivityAutomationData",
      "sourceInterface": "LiveActivityAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "live.disconnected",
        "timestamp": 0,
        "platform": "twitch",
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "live.disconnected",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_14,
        SHARED_REGISTRY_98,
        SHARED_REGISTRY_99,
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LiveActivityAutomationData.method",
            "es": "LiveActivityAutomationData.method"
          },
          "sample": "live.disconnected",
          "sourceField": "method"
        },
        SHARED_REGISTRY_100,
        SHARED_REGISTRY_101,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_102,
      "note": "Generated from LiveActivityAutomationData JSON Schema."
    },
    "tiktok.chat": {
      "dataInterface": "ChatAutomationData",
      "sourceInterface": "WebcastChatMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.chat",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "comment": "Hello there",
          "emotes": [],
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1"
        },
        "intel": {
          "comment": {
            "casefolded": "sample",
            "composition": SHARED_REGISTRY_1,
            "language": {
              "candidates": [],
              "confidence": 0.9,
              "top": "en"
            },
            "nfc": "sample",
            "nfkc": "sample",
            "normalized": "hello there",
            "obfuscation": SHARED_REGISTRY_2,
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "score": 1,
              "strong": false
            },
            "spam": {
              "calibrated": false,
              "detected": false,
              "model": "sample",
              "reasons": [],
              "score": 0.04
            },
            "truncated": false,
            "tts": {
              "confidence": 0.9,
              "language": "en",
              "pronunciation": {
                "confidence": 1,
                "dialect": "sample",
                "ipa": "sample",
                "language": "sample"
              },
              "reason": "sample",
              "source": "sample",
              "speak": false,
              "text": "hello there"
            },
            "unicode": {
              "bidirectional": 1,
              "confusables": 1,
              "invisible": 1,
              "mixedScripts": false,
              "score": 1,
              "suspicious": false
            }
          },
          "processing": {
            "status": "sample"
          },
          "providers": {},
          "user": {
            "nickname": {
              "language": {
                "candidates": [],
                "confidence": 1,
                "top": "sample"
              },
              "normalized": "Viewer Demo",
              "tts": {
                "confidence": 1,
                "language": "sample",
                "pronunciation": {
                  "confidence": 1,
                  "dialect": "sample",
                  "ipa": "sample",
                  "language": "sample"
                },
                "reason": "sample",
                "source": "sample",
                "speak": false,
                "text": "Viewer Demo"
              }
            },
            "uniqueId": SHARED_REGISTRY_6
          }
        }
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.comment",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Comment",
            "es": "Comment"
          },
          "hint": {
            "en": "ChatAutomationData.comment",
            "es": "ChatAutomationData.comment"
          },
          "sample": "Hello there",
          "sourceField": "comment",
          "sourceMethod": "WebcastChatMessage",
          "sourcePath": "content",
          "sourceTransform": "native",
          "sourceJsonPath": "content",
          "sourceProtoType": "string",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.emotes",
          "tsType": "({ \"emoteType\"?: number | null; \"id\": string; \"imageUrl\"?: string | null; \"index\"?: number | null; \"packageId\"?: string | null; \"shortcode\"?: string | null })[]",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Emotes",
            "es": "Emotes"
          },
          "hint": {
            "en": "ChatAutomationData.emotes",
            "es": "ChatAutomationData.emotes"
          },
          "sample": [],
          "sourceField": "emotes",
          "sourceMethod": "WebcastChatMessage",
          "sourcePath": "emotes",
          "sourceTransform": "normalized-message-array",
          "sourceJsonPath": "emotes",
          "sourceProtoType": "message",
          "sourceCardinality": "repeated"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "ChatAutomationData.isHistory",
            "es": "ChatAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "ChatAutomationData.method",
            "es": "ChatAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "ChatAutomationData.msgId",
            "es": "ChatAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        SHARED_REGISTRY_15,
        SHARED_REGISTRY_16,
        SHARED_REGISTRY_17,
        SHARED_REGISTRY_18,
        SHARED_REGISTRY_19,
        SHARED_REGISTRY_20,
        SHARED_REGISTRY_21,
        SHARED_REGISTRY_22,
        SHARED_REGISTRY_23,
        SHARED_REGISTRY_24,
        SHARED_REGISTRY_25,
        SHARED_REGISTRY_26,
        SHARED_REGISTRY_27,
        SHARED_REGISTRY_28,
        SHARED_REGISTRY_29,
        SHARED_REGISTRY_30,
        SHARED_REGISTRY_31,
        SHARED_REGISTRY_32,
        SHARED_REGISTRY_33,
        SHARED_REGISTRY_34,
        SHARED_REGISTRY_35,
        SHARED_REGISTRY_36,
        SHARED_REGISTRY_37,
        SHARED_REGISTRY_38,
        SHARED_REGISTRY_39,
        SHARED_REGISTRY_40,
        SHARED_REGISTRY_41,
        SHARED_REGISTRY_42,
        SHARED_REGISTRY_43,
        SHARED_REGISTRY_44,
        SHARED_REGISTRY_45,
        SHARED_REGISTRY_46,
        SHARED_REGISTRY_47,
        SHARED_REGISTRY_48,
        SHARED_REGISTRY_49,
        SHARED_REGISTRY_50,
        SHARED_REGISTRY_51,
        SHARED_REGISTRY_52,
        SHARED_REGISTRY_53,
        SHARED_REGISTRY_54,
        SHARED_REGISTRY_55,
        SHARED_REGISTRY_56,
        SHARED_REGISTRY_57,
        SHARED_REGISTRY_58,
        SHARED_REGISTRY_59,
        SHARED_REGISTRY_60,
        SHARED_REGISTRY_61,
        SHARED_REGISTRY_62,
        SHARED_REGISTRY_63,
        SHARED_REGISTRY_64,
        SHARED_REGISTRY_65,
        SHARED_REGISTRY_66,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_82,
        SHARED_REGISTRY_83,
        SHARED_REGISTRY_84,
        SHARED_REGISTRY_85,
        SHARED_REGISTRY_86,
        SHARED_REGISTRY_87,
        SHARED_REGISTRY_88,
        SHARED_REGISTRY_89,
        SHARED_REGISTRY_91,
        SHARED_REGISTRY_92,
        SHARED_REGISTRY_93,
        SHARED_REGISTRY_94,
        SHARED_REGISTRY_95
      ],
      "sourceFields": SHARED_REGISTRY_97,
      "note": "Generated from ChatAutomationData JSON Schema."
    },
    "tiktok.emote_chat": {
      "dataInterface": "EmoteChatAutomationData",
      "sourceInterface": "WebcastEmoteChatMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.emote_chat",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "emotes": [],
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.emotes",
          "tsType": "({ \"emoteType\"?: number | null; \"id\": string; \"imageUrl\"?: string | null; \"index\"?: number | null; \"packageId\"?: string | null; \"shortcode\"?: string | null })[]",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Emotes",
            "es": "Emotes"
          },
          "hint": {
            "en": "EmoteChatAutomationData.emotes",
            "es": "EmoteChatAutomationData.emotes"
          },
          "sample": [],
          "sourceField": "emotes",
          "sourceMethod": "WebcastEmoteChatMessage",
          "sourcePath": "emote_list",
          "sourceTransform": "normalized-message-array",
          "sourceJsonPath": "emoteList",
          "sourceProtoType": "message",
          "sourceCardinality": "repeated"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "EmoteChatAutomationData.isHistory",
            "es": "EmoteChatAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "EmoteChatAutomationData.method",
            "es": "EmoteChatAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "EmoteChatAutomationData.msgId",
            "es": "EmoteChatAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        SHARED_REGISTRY_96,
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from EmoteChatAutomationData JSON Schema."
    },
    "tiktok.gift": {
      "dataInterface": "GiftAutomationData",
      "sourceInterface": "WebcastGiftMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.gift",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "aggregation": {
            "cumulative": 1,
            "diamonds": 1,
            "increment": 1
          },
          "comboCount": 1,
          "diamondCount": 1,
          "giftIconUrl": "sample",
          "giftId": "5655",
          "giftName": "Rose",
          "groupId": "sample",
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "repeatCount": 1,
          "repeatEnd": false,
          "streakable": false
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.aggregation",
          "tsType": "{ \"cumulative\": number; \"diamonds\": number; \"increment\": number } | null",
          "kind": "object",
          "optional": true,
          "label": {
            "en": "Aggregation",
            "es": "Aggregation"
          },
          "hint": {
            "en": "GiftAutomationData.aggregation",
            "es": "GiftAutomationData.aggregation"
          },
          "sample": {
            "cumulative": 1,
            "diamonds": 1,
            "increment": 1
          },
          "sourceField": "aggregation"
        },
        {
          "path": "event.data.comboCount",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Combo Count",
            "es": "Combo Count"
          },
          "hint": {
            "en": "GiftAutomationData.comboCount",
            "es": "GiftAutomationData.comboCount"
          },
          "sample": 1,
          "sourceField": "comboCount",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "combo_count",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "comboCount",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.diamondCount",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Diamond Count",
            "es": "Diamond Count"
          },
          "hint": {
            "en": "GiftAutomationData.diamondCount",
            "es": "GiftAutomationData.diamondCount"
          },
          "sample": 1,
          "sourceField": "diamondCount",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "gift.diamond_count",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "gift.diamondCount",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.giftIconUrl",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Gift Icon Url",
            "es": "Gift Icon Url"
          },
          "hint": {
            "en": "GiftAutomationData.giftIconUrl",
            "es": "GiftAutomationData.giftIconUrl"
          },
          "sample": "sample",
          "sourceField": "giftIconUrl"
        },
        {
          "path": "event.data.giftId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Gift Id",
            "es": "Gift Id"
          },
          "hint": {
            "en": "GiftAutomationData.giftId",
            "es": "GiftAutomationData.giftId"
          },
          "sample": "5655",
          "sourceField": "giftId",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "gift_id",
          "sourceTransform": "u64-to-string",
          "sourceJsonPath": "giftId",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.giftName",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Gift Name",
            "es": "Gift Name"
          },
          "hint": {
            "en": "GiftAutomationData.giftName",
            "es": "GiftAutomationData.giftName"
          },
          "sample": "Rose",
          "sourceField": "giftName",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "gift.name",
          "sourceTransform": "native",
          "sourceJsonPath": "gift.name",
          "sourceProtoType": "string",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.groupId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Group Id",
            "es": "Group Id"
          },
          "hint": {
            "en": "GiftAutomationData.groupId",
            "es": "GiftAutomationData.groupId"
          },
          "sample": "sample",
          "sourceField": "groupId",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "group_id",
          "sourceTransform": "integer-to-string",
          "sourceJsonPath": "groupId",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "GiftAutomationData.isHistory",
            "es": "GiftAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "GiftAutomationData.method",
            "es": "GiftAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "GiftAutomationData.msgId",
            "es": "GiftAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.repeatCount",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Repeat Count",
            "es": "Repeat Count"
          },
          "hint": {
            "en": "GiftAutomationData.repeatCount",
            "es": "GiftAutomationData.repeatCount"
          },
          "sample": 1,
          "sourceField": "repeatCount",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "repeat_count",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "repeatCount",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.repeatEnd",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Repeat End",
            "es": "Repeat End"
          },
          "hint": {
            "en": "GiftAutomationData.repeatEnd",
            "es": "GiftAutomationData.repeatEnd"
          },
          "sample": false,
          "sourceField": "repeatEnd",
          "sourceMethod": "WebcastGiftMessage",
          "sourcePath": "repeat_end",
          "sourceTransform": "nonzero-to-boolean",
          "sourceJsonPath": "repeatEnd",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.streakable",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Streakable",
            "es": "Streakable"
          },
          "hint": {
            "en": "GiftAutomationData.streakable",
            "es": "GiftAutomationData.streakable"
          },
          "sample": false,
          "sourceField": "streakable"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "aggregation",
          "tsType": "{ \"cumulative\": number; \"diamonds\": number; \"increment\": number } | null",
          "optional": true
        },
        {
          "name": "comboCount",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "diamondCount",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "giftIconUrl",
          "tsType": "string | null",
          "optional": true
        },
        {
          "name": "giftId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "giftName",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "groupId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "repeatCount",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "repeatEnd",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "streakable",
          "tsType": "boolean",
          "optional": false
        }
      ],
      "note": "Generated from GiftAutomationData JSON Schema."
    },
    "tiktok.like": {
      "dataInterface": "LikeAutomationData",
      "sourceInterface": "WebcastLikeMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.like",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "aggregation": {
            "global": {
              "previous": 1,
              "total": 1
            },
            "user": {
              "previous": 1,
              "total": 1
            }
          },
          "count": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "total": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.aggregation",
          "tsType": "{ \"global\"?: { \"previous\": number; \"total\": number } | null; \"user\"?: { \"previous\": number; \"total\": number } | null } | null",
          "kind": "object",
          "optional": true,
          "label": {
            "en": "Aggregation",
            "es": "Aggregation"
          },
          "hint": {
            "en": "LikeAutomationData.aggregation",
            "es": "LikeAutomationData.aggregation"
          },
          "sample": {
            "global": {
              "previous": 1,
              "total": 1
            },
            "user": {
              "previous": 1,
              "total": 1
            }
          },
          "sourceField": "aggregation"
        },
        {
          "path": "event.data.count",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Count",
            "es": "Count"
          },
          "hint": {
            "en": "LikeAutomationData.count",
            "es": "LikeAutomationData.count"
          },
          "sample": 1,
          "sourceField": "count",
          "sourceMethod": "WebcastLikeMessage",
          "sourcePath": "count",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "count",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "LikeAutomationData.isHistory",
            "es": "LikeAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "LikeAutomationData.method",
            "es": "LikeAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "LikeAutomationData.msgId",
            "es": "LikeAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.total",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Total",
            "es": "Total"
          },
          "hint": {
            "en": "LikeAutomationData.total",
            "es": "LikeAutomationData.total"
          },
          "sample": 1,
          "sourceField": "total",
          "sourceMethod": "WebcastLikeMessage",
          "sourcePath": "total",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "total",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "aggregation",
          "tsType": "{ \"global\"?: { \"previous\": number; \"total\": number } | null; \"user\"?: { \"previous\": number; \"total\": number } | null } | null",
          "optional": true
        },
        {
          "name": "count",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "total",
          "tsType": "number",
          "optional": false
        }
      ],
      "note": "Generated from LikeAutomationData JSON Schema."
    },
    "tiktok.follow": {
      "dataInterface": "SocialAutomationData",
      "sourceInterface": "WebcastSocialMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.follow",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "action": 1,
          "followCount": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "shareCount": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.action",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Action",
            "es": "Action"
          },
          "hint": {
            "en": "SocialAutomationData.action",
            "es": "SocialAutomationData.action"
          },
          "sample": 1,
          "sourceField": "action",
          "sourceMethod": "WebcastSocialMessage",
          "sourcePath": "action",
          "sourceTransform": "native",
          "sourceJsonPath": "action",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        SHARED_REGISTRY_103,
        SHARED_REGISTRY_104,
        SHARED_REGISTRY_105,
        SHARED_REGISTRY_106,
        SHARED_REGISTRY_107,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_108,
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.share": {
      "dataInterface": "SocialAutomationData",
      "sourceInterface": "WebcastSocialMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.share",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "action": 3,
          "followCount": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "shareCount": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.action",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Action",
            "es": "Action"
          },
          "hint": {
            "en": "SocialAutomationData.action",
            "es": "SocialAutomationData.action"
          },
          "sample": 3,
          "sourceField": "action",
          "sourceMethod": "WebcastSocialMessage",
          "sourcePath": "action",
          "sourceTransform": "native",
          "sourceJsonPath": "action",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        SHARED_REGISTRY_103,
        SHARED_REGISTRY_104,
        SHARED_REGISTRY_105,
        SHARED_REGISTRY_106,
        SHARED_REGISTRY_107,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_108,
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.join": {
      "dataInterface": "MemberAutomationData",
      "sourceInterface": "WebcastMemberMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.join",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "action": 0,
          "isHistory": false,
          "memberCount": 1,
          "method": "WebcastSampleMessage",
          "msgId": "1"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.action",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Action",
            "es": "Action"
          },
          "hint": {
            "en": "MemberAutomationData.action",
            "es": "MemberAutomationData.action"
          },
          "sample": 0,
          "sourceField": "action",
          "sourceMethod": "WebcastMemberMessage",
          "sourcePath": "action",
          "sourceTransform": "native",
          "sourceJsonPath": "action",
          "sourceProtoType": "enum",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "MemberAutomationData.isHistory",
            "es": "MemberAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.memberCount",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Member Count",
            "es": "Member Count"
          },
          "hint": {
            "en": "MemberAutomationData.memberCount",
            "es": "MemberAutomationData.memberCount"
          },
          "sample": 1,
          "sourceField": "memberCount",
          "sourceMethod": "WebcastMemberMessage",
          "sourcePath": "member_count",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "memberCount",
          "sourceProtoType": "int32",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "MemberAutomationData.method",
            "es": "MemberAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "MemberAutomationData.msgId",
            "es": "MemberAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "action",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "memberCount",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from MemberAutomationData JSON Schema."
    },
    "tiktok.social": {
      "dataInterface": "SocialAutomationData",
      "sourceInterface": "WebcastSocialMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.social",
        "timestamp": 0,
        "user": {
          "avatarUrl": "sample",
          "nickname": "Viewer Demo",
          "secUid": "",
          "uniqueId": "usuario_demo",
          "userId": "1"
        },
        "data": {
          "action": 0,
          "followCount": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "shareCount": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        SHARED_REGISTRY_9,
        SHARED_REGISTRY_10,
        SHARED_REGISTRY_11,
        SHARED_REGISTRY_12,
        SHARED_REGISTRY_13,
        {
          "path": "event.data.action",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Action",
            "es": "Action"
          },
          "hint": {
            "en": "SocialAutomationData.action",
            "es": "SocialAutomationData.action"
          },
          "sample": 0,
          "sourceField": "action",
          "sourceMethod": "WebcastSocialMessage",
          "sourcePath": "action",
          "sourceTransform": "native",
          "sourceJsonPath": "action",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        SHARED_REGISTRY_103,
        SHARED_REGISTRY_104,
        SHARED_REGISTRY_105,
        SHARED_REGISTRY_106,
        SHARED_REGISTRY_107,
        SHARED_REGISTRY_67,
        SHARED_REGISTRY_68,
        SHARED_REGISTRY_69,
        SHARED_REGISTRY_70,
        SHARED_REGISTRY_71,
        SHARED_REGISTRY_72,
        SHARED_REGISTRY_74,
        SHARED_REGISTRY_75,
        SHARED_REGISTRY_76,
        SHARED_REGISTRY_77,
        SHARED_REGISTRY_78,
        SHARED_REGISTRY_79,
        SHARED_REGISTRY_80,
        SHARED_REGISTRY_81,
        SHARED_REGISTRY_66
      ],
      "sourceFields": SHARED_REGISTRY_108,
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.room_stats": {
      "dataInterface": "RoomStatsAutomationData",
      "sourceInterface": "WebcastRoomUserSeqMessage",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.room_stats",
        "timestamp": 0,
        "data": {
          "anonymous": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "popularity": 1,
          "totalUsers": 1,
          "viewers": 1
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        {
          "path": "event.data.anonymous",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Anonymous",
            "es": "Anonymous"
          },
          "hint": {
            "en": "RoomStatsAutomationData.anonymous",
            "es": "RoomStatsAutomationData.anonymous"
          },
          "sample": 1,
          "sourceField": "anonymous",
          "sourceMethod": "WebcastRoomUserSeqMessage",
          "sourcePath": "anonymous",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "anonymous",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.isHistory",
          "tsType": "boolean",
          "kind": "boolean",
          "optional": false,
          "label": {
            "en": "Is History",
            "es": "Is History"
          },
          "hint": {
            "en": "RoomStatsAutomationData.isHistory",
            "es": "RoomStatsAutomationData.isHistory"
          },
          "sample": false,
          "sourceField": "isHistory"
        },
        {
          "path": "event.data.method",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Method",
            "es": "Method"
          },
          "hint": {
            "en": "RoomStatsAutomationData.method",
            "es": "RoomStatsAutomationData.method"
          },
          "sample": "WebcastSampleMessage",
          "sourceField": "method"
        },
        {
          "path": "event.data.msgId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Msg Id",
            "es": "Msg Id"
          },
          "hint": {
            "en": "RoomStatsAutomationData.msgId",
            "es": "RoomStatsAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
          "path": "event.data.popularity",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Popularity",
            "es": "Popularity"
          },
          "hint": {
            "en": "RoomStatsAutomationData.popularity",
            "es": "RoomStatsAutomationData.popularity"
          },
          "sample": 1,
          "sourceField": "popularity",
          "sourceMethod": "WebcastRoomUserSeqMessage",
          "sourcePath": "popularity",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "popularity",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.totalUsers",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Total Users",
            "es": "Total Users"
          },
          "hint": {
            "en": "RoomStatsAutomationData.totalUsers",
            "es": "RoomStatsAutomationData.totalUsers"
          },
          "sample": 1,
          "sourceField": "totalUsers",
          "sourceMethod": "WebcastRoomUserSeqMessage",
          "sourcePath": "total_user",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "totalUser",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        {
          "path": "event.data.viewers",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Viewers",
            "es": "Viewers"
          },
          "hint": {
            "en": "RoomStatsAutomationData.viewers",
            "es": "RoomStatsAutomationData.viewers"
          },
          "sample": 1,
          "sourceField": "viewers",
          "sourceMethod": "WebcastRoomUserSeqMessage",
          "sourcePath": "total",
          "sourceTransform": "normalized-unsigned",
          "sourceJsonPath": "total",
          "sourceProtoType": "int64",
          "sourceCardinality": "optional"
        },
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "anonymous",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "isHistory",
          "tsType": "boolean",
          "optional": false
        },
        {
          "name": "method",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "msgId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "popularity",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "totalUsers",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "viewers",
          "tsType": "number",
          "optional": false
        }
      ],
      "note": "Generated from RoomStatsAutomationData JSON Schema."
    },
    "tiktok.connected": {
      "dataInterface": "ConnectionAutomationData",
      "sourceInterface": "ConnectionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.connected",
        "timestamp": 0,
        "data": {
          "roomId": "sample",
          "uniqueId": "sample"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_111,
      "sourceFields": [
        {
          "name": "roomId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "uniqueId",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from ConnectionAutomationData JSON Schema."
    },
    "tiktok.disconnected": {
      "dataInterface": "ConnectionAutomationData",
      "sourceInterface": "ConnectionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.disconnected",
        "timestamp": 0,
        "data": {
          "roomId": "sample",
          "uniqueId": "sample"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_111,
      "sourceFields": [
        {
          "name": "roomId",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "uniqueId",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from ConnectionAutomationData JSON Schema."
    },
    "points.awarded": {
      "dataInterface": "PointsAwardedAutomationData",
      "sourceInterface": "PointsAwardedAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "points.awarded",
        "timestamp": 0,
        "data": {
          "currencyName": "sample",
          "delta": 1,
          "level": 1,
          "reason": "sample",
          "totalPoints": 1,
          "uniqueId": "sample"
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        {
          "path": "event.data.currencyName",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Currency Name",
            "es": "Currency Name"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.currencyName",
            "es": "PointsAwardedAutomationData.currencyName"
          },
          "sample": "sample",
          "sourceField": "currencyName"
        },
        {
          "path": "event.data.delta",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Delta",
            "es": "Delta"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.delta",
            "es": "PointsAwardedAutomationData.delta"
          },
          "sample": 1,
          "sourceField": "delta"
        },
        {
          "path": "event.data.level",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Level",
            "es": "Level"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.level",
            "es": "PointsAwardedAutomationData.level"
          },
          "sample": 1,
          "sourceField": "level"
        },
        {
          "path": "event.data.reason",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Reason",
            "es": "Reason"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.reason",
            "es": "PointsAwardedAutomationData.reason"
          },
          "sample": "sample",
          "sourceField": "reason"
        },
        {
          "path": "event.data.totalPoints",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Total Points",
            "es": "Total Points"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.totalPoints",
            "es": "PointsAwardedAutomationData.totalPoints"
          },
          "sample": 1,
          "sourceField": "totalPoints"
        },
        {
          "path": "event.data.uniqueId",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Unique Id",
            "es": "Unique Id"
          },
          "hint": {
            "en": "PointsAwardedAutomationData.uniqueId",
            "es": "PointsAwardedAutomationData.uniqueId"
          },
          "sample": "sample",
          "sourceField": "uniqueId"
        },
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "currencyName",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "delta",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "level",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "reason",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "totalPoints",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "uniqueId",
          "tsType": "string",
          "optional": false
        }
      ],
      "note": "Generated from PointsAwardedAutomationData JSON Schema."
    },
    "plugin.emit": {
      "dataInterface": "PluginEmitAutomationData",
      "sourceInterface": "PluginEmitAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "plugin.emit",
        "timestamp": 0,
        "data": {
          "depth": 0,
          "emitType": "plugin.sample",
          "payload": {}
        },
        "intel": SHARED_REGISTRY_8
      },
      "fields": [
        {
          "path": "event.data.depth",
          "tsType": "number",
          "kind": "number",
          "optional": false,
          "label": {
            "en": "Depth",
            "es": "Depth"
          },
          "hint": {
            "en": "PluginEmitAutomationData.depth",
            "es": "PluginEmitAutomationData.depth"
          },
          "sample": 0,
          "sourceField": "depth"
        },
        {
          "path": "event.data.emitType",
          "tsType": "string",
          "kind": "string",
          "optional": false,
          "label": {
            "en": "Emit Type",
            "es": "Emit Type"
          },
          "hint": {
            "en": "PluginEmitAutomationData.emitType",
            "es": "PluginEmitAutomationData.emitType"
          },
          "sample": "plugin.sample",
          "sourceField": "emitType"
        },
        {
          "path": "event.data.payload",
          "tsType": "JsonValue",
          "kind": "unknown",
          "optional": false,
          "label": {
            "en": "Payload",
            "es": "Payload"
          },
          "hint": {
            "en": "PluginEmitAutomationData.payload",
            "es": "PluginEmitAutomationData.payload"
          },
          "sample": {},
          "sourceField": "payload"
        },
        SHARED_REGISTRY_66
      ],
      "sourceFields": [
        {
          "name": "depth",
          "tsType": "number",
          "optional": false
        },
        {
          "name": "emitType",
          "tsType": "string",
          "optional": false
        },
        {
          "name": "payload",
          "tsType": "JsonValue",
          "optional": false
        }
      ],
      "note": "Generated from PluginEmitAutomationData JSON Schema."
    },
    "competition.started": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.started",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.paused": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.paused",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.resumed": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.resumed",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.cancelled": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.cancelled",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.finished": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.finished",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.winner.selected": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.winner.selected",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.action.completed": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.action.completed",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    },
    "competition.action.failed": {
      "dataInterface": "CompetitionAutomationData",
      "sourceInterface": "CompetitionAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "competition.action.failed",
        "timestamp": 0,
        "data": SHARED_REGISTRY_112,
        "intel": SHARED_REGISTRY_8
      },
      "fields": SHARED_REGISTRY_125,
      "sourceFields": SHARED_REGISTRY_127,
      "note": "Generated from CompetitionAutomationData JSON Schema."
    }
  }
} as const satisfies Record<string, unknown>;
