// THIS FILE IS GENERATED. Run bun run contracts:generate.

export const EVENT_REGISTRY_VERSION = 4 as const;
export const GENERATED_EVENT_REGISTRY = {
  "version": 4,
  "generatedBy": "tiktools-core automation contracts",
  "generatedFrom": [
    "crates/tiktools-core/src/contracts",
    "src/automation/contracts/generated/automation-events.schema.json"
  ],
  "events": {
    "tiktok.chat": {
      "dataInterface": "ChatAutomationData",
      "sourceInterface": "ChatAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.chat",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
          "userId": "1"
        },
        "data": {
          "comment": "Hello there",
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1"
        },
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 0.9,
              "top": "en"
            },
            "normalized": "hello there",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 0.04
            },
            "tts": {
              "confidence": 0.9,
              "ipa": "sample",
              "language": "en",
              "source": "sample",
              "speak": false,
              "text": "hello there"
            },
            "unicode": {
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
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "Viewer Demo"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
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
          "sourceField": "comment"
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
        {
          "path": "event.intel.comment.composition.allCaps",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "All Caps",
            "es": "All Caps"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.allCaps",
            "es": "AutomationIntel.comment.composition.allCaps"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.composition.digits",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Digits",
            "es": "Digits"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.digits",
            "es": "AutomationIntel.comment.composition.digits"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.elongated",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Elongated",
            "es": "Elongated"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.elongated",
            "es": "AutomationIntel.comment.composition.elongated"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.composition.emojiCount",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Emoji Count",
            "es": "Emoji Count"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.emojiCount",
            "es": "AutomationIntel.comment.composition.emojiCount"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.emojiOnly",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Emoji Only",
            "es": "Emoji Only"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.emojiOnly",
            "es": "AutomationIntel.comment.composition.emojiOnly"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.composition.emojiRatio",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Emoji Ratio",
            "es": "Emoji Ratio"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.emojiRatio",
            "es": "AutomationIntel.comment.composition.emojiRatio"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.letters",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Letters",
            "es": "Letters"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.letters",
            "es": "AutomationIntel.comment.composition.letters"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.mentions",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Mentions",
            "es": "Mentions"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.mentions",
            "es": "AutomationIntel.comment.composition.mentions"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.repetitionScore",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Repetition Score",
            "es": "Repetition Score"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.repetitionScore",
            "es": "AutomationIntel.comment.composition.repetitionScore"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.composition.urls",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Urls",
            "es": "Urls"
          },
          "hint": {
            "en": "AutomationIntel.comment.composition.urls",
            "es": "AutomationIntel.comment.composition.urls"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.comment.language.candidates",
            "es": "AutomationIntel.comment.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.comment.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.comment.language.confidence",
            "es": "AutomationIntel.comment.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.comment.language.top",
            "es": "AutomationIntel.comment.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.comment.normalized",
            "es": "AutomationIntel.comment.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.obfuscation.confusables",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Confusables",
            "es": "Confusables"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.confusables",
            "es": "AutomationIntel.comment.obfuscation.confusables"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.detected",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Detected",
            "es": "Detected"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.detected",
            "es": "AutomationIntel.comment.obfuscation.detected"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.leetspeak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Leetspeak",
            "es": "Leetspeak"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.leetspeak",
            "es": "AutomationIntel.comment.obfuscation.leetspeak"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.mixedScripts",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Mixed Scripts",
            "es": "Mixed Scripts"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.mixedScripts",
            "es": "AutomationIntel.comment.obfuscation.mixedScripts"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.punctuationFlood",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Punctuation Flood",
            "es": "Punctuation Flood"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.punctuationFlood",
            "es": "AutomationIntel.comment.obfuscation.punctuationFlood"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.repetition",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Repetition",
            "es": "Repetition"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.repetition",
            "es": "AutomationIntel.comment.obfuscation.repetition"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.obfuscation.score",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Score",
            "es": "Score"
          },
          "hint": {
            "en": "AutomationIntel.comment.obfuscation.score",
            "es": "AutomationIntel.comment.obfuscation.score"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.rebus.candidate",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Candidate",
            "es": "Candidate"
          },
          "hint": {
            "en": "AutomationIntel.comment.rebus.candidate",
            "es": "AutomationIntel.comment.rebus.candidate"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.rebus.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.comment.rebus.confidence",
            "es": "AutomationIntel.comment.rebus.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.rebus.strong",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Strong",
            "es": "Strong"
          },
          "hint": {
            "en": "AutomationIntel.comment.rebus.strong",
            "es": "AutomationIntel.comment.rebus.strong"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.spam.detected",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Detected",
            "es": "Detected"
          },
          "hint": {
            "en": "AutomationIntel.comment.spam.detected",
            "es": "AutomationIntel.comment.spam.detected"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.spam.score",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Score",
            "es": "Score"
          },
          "hint": {
            "en": "AutomationIntel.comment.spam.score",
            "es": "AutomationIntel.comment.spam.score"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.confidence",
            "es": "AutomationIntel.comment.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.ipa",
            "es": "AutomationIntel.comment.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.language",
            "es": "AutomationIntel.comment.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.source",
            "es": "AutomationIntel.comment.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.speak",
            "es": "AutomationIntel.comment.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.comment.tts.text",
            "es": "AutomationIntel.comment.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.comment.unicode.mixedScripts",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Mixed Scripts",
            "es": "Mixed Scripts"
          },
          "hint": {
            "en": "AutomationIntel.comment.unicode.mixedScripts",
            "es": "AutomationIntel.comment.unicode.mixedScripts"
          },
          "sample": false
        },
        {
          "path": "event.intel.comment.unicode.score",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Score",
            "es": "Score"
          },
          "hint": {
            "en": "AutomationIntel.comment.unicode.score",
            "es": "AutomationIntel.comment.unicode.score"
          },
          "sample": 1
        },
        {
          "path": "event.intel.comment.unicode.suspicious",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Suspicious",
            "es": "Suspicious"
          },
          "hint": {
            "en": "AutomationIntel.comment.unicode.suspicious",
            "es": "AutomationIntel.comment.unicode.suspicious"
          },
          "sample": false
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
        {
          "name": "comment",
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
        }
      ],
      "note": "Generated from ChatAutomationData JSON Schema."
    },
    "tiktok.gift": {
      "dataInterface": "GiftAutomationData",
      "sourceInterface": "GiftAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.gift",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
          "userId": "1"
        },
        "data": {
          "comboCount": 1,
          "diamondCount": 1,
          "giftIconUrl": "sample",
          "giftId": "5655",
          "giftName": "Rosa",
          "groupId": "sample",
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "repeatCount": 1,
          "repeatEnd": false,
          "streakable": false
        },
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
          "sourceField": "comboCount"
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
          "sourceField": "diamondCount"
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
          "sourceField": "giftId"
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
          "sample": "Rosa",
          "sourceField": "giftName"
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
          "sourceField": "groupId"
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
          "sourceField": "repeatCount"
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
          "sourceField": "repeatEnd"
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
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
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
      "sourceInterface": "LikeAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.like",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
          "userId": "1"
        },
        "data": {
          "count": 1,
          "isHistory": false,
          "method": "WebcastSampleMessage",
          "msgId": "1",
          "total": 1
        },
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
          "sourceField": "count"
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
          "sourceField": "total"
        },
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
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
      "sourceInterface": "SocialAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.follow",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
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
          "sourceField": "action"
        },
        {
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
          "sourceField": "followCount"
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
            "en": "SocialAutomationData.isHistory",
            "es": "SocialAutomationData.isHistory"
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
            "en": "SocialAutomationData.method",
            "es": "SocialAutomationData.method"
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
            "en": "SocialAutomationData.msgId",
            "es": "SocialAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
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
          "sourceField": "shareCount"
        },
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
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
      ],
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.share": {
      "dataInterface": "SocialAutomationData",
      "sourceInterface": "SocialAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.share",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
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
          "sourceField": "action"
        },
        {
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
          "sourceField": "followCount"
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
            "en": "SocialAutomationData.isHistory",
            "es": "SocialAutomationData.isHistory"
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
            "en": "SocialAutomationData.method",
            "es": "SocialAutomationData.method"
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
            "en": "SocialAutomationData.msgId",
            "es": "SocialAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
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
          "sourceField": "shareCount"
        },
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
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
      ],
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.join": {
      "dataInterface": "MemberAutomationData",
      "sourceInterface": "MemberAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.join",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
          "userId": "1"
        },
        "data": {
          "action": 1,
          "isHistory": false,
          "memberCount": 1,
          "method": "WebcastSampleMessage",
          "msgId": "1"
        },
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
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
          "sample": 1,
          "sourceField": "action"
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
          "sourceField": "memberCount"
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
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
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
      "sourceInterface": "SocialAutomationData",
      "sampleEvent": {
        "id": "sample-event",
        "type": "tiktok.social",
        "timestamp": 0,
        "user": {
          "uniqueId": "usuario_demo",
          "nickname": "Viewer Demo",
          "secUid": "",
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
        },
        {
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
        },
        {
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
        },
        {
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
        },
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
          "sourceField": "action"
        },
        {
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
          "sourceField": "followCount"
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
            "en": "SocialAutomationData.isHistory",
            "es": "SocialAutomationData.isHistory"
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
            "en": "SocialAutomationData.method",
            "es": "SocialAutomationData.method"
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
            "en": "SocialAutomationData.msgId",
            "es": "SocialAutomationData.msgId"
          },
          "sample": "1",
          "sourceField": "msgId"
        },
        {
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
          "sourceField": "shareCount"
        },
        {
          "path": "event.intel.user.nickname.language.candidates",
          "tsType": "JsonValue[] | null",
          "kind": "array",
          "optional": true,
          "label": {
            "en": "Candidates",
            "es": "Candidates"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.candidates",
            "es": "AutomationIntel.user.nickname.language.candidates"
          },
          "sample": []
        },
        {
          "path": "event.intel.user.nickname.language.confidence",
          "tsType": "number",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.confidence",
            "es": "AutomationIntel.user.nickname.language.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.language.top",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Top",
            "es": "Top"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.language.top",
            "es": "AutomationIntel.user.nickname.language.top"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.normalized",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Normalized",
            "es": "Normalized"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.normalized",
            "es": "AutomationIntel.user.nickname.normalized"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.confidence",
          "tsType": "number | null",
          "kind": "number",
          "optional": true,
          "label": {
            "en": "Confidence",
            "es": "Confidence"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.confidence",
            "es": "AutomationIntel.user.nickname.tts.confidence"
          },
          "sample": 1
        },
        {
          "path": "event.intel.user.nickname.tts.ipa",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Ipa",
            "es": "Ipa"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.ipa",
            "es": "AutomationIntel.user.nickname.tts.ipa"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.language",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Language",
            "es": "Language"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.language",
            "es": "AutomationIntel.user.nickname.tts.language"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.source",
          "tsType": "string | null",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Source",
            "es": "Source"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.source",
            "es": "AutomationIntel.user.nickname.tts.source"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.user.nickname.tts.speak",
          "tsType": "boolean | null",
          "kind": "boolean",
          "optional": true,
          "label": {
            "en": "Speak",
            "es": "Speak"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.speak",
            "es": "AutomationIntel.user.nickname.tts.speak"
          },
          "sample": false
        },
        {
          "path": "event.intel.user.nickname.tts.text",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Text",
            "es": "Text"
          },
          "hint": {
            "en": "AutomationIntel.user.nickname.tts.text",
            "es": "AutomationIntel.user.nickname.tts.text"
          },
          "sample": "sample"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
      "sourceFields": [
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
      ],
      "note": "Generated from SocialAutomationData JSON Schema."
    },
    "tiktok.room_stats": {
      "dataInterface": "RoomStatsAutomationData",
      "sourceInterface": "RoomStatsAutomationData",
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
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
          "sourceField": "anonymous"
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
          "sourceField": "popularity"
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
            "en": "RoomStatsAutomationData.viewers",
            "es": "RoomStatsAutomationData.viewers"
          },
          "sample": 1,
          "sourceField": "viewers"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
            "en": "ConnectionAutomationData.uniqueId",
            "es": "ConnectionAutomationData.uniqueId"
          },
          "sample": "sample",
          "sourceField": "uniqueId"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
      },
      "fields": [
        {
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
            "en": "ConnectionAutomationData.uniqueId",
            "es": "ConnectionAutomationData.uniqueId"
          },
          "sample": "sample",
          "sourceField": "uniqueId"
        },
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
      ],
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
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
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
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
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
          "depth": 1,
          "emitType": "plugin.sample",
          "payload": {}
        },
        "intel": {
          "comment": {
            "composition": {
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
            },
            "language": {
              "candidates": [],
              "confidence": 1,
              "top": "sample"
            },
            "normalized": "sample",
            "obfuscation": {
              "confusables": false,
              "detected": false,
              "leetspeak": false,
              "mixedScripts": false,
              "punctuationFlood": false,
              "repetition": false,
              "score": 1
            },
            "rebus": {
              "candidate": "sample",
              "confidence": 1,
              "strong": false
            },
            "spam": {
              "detected": false,
              "score": 1
            },
            "tts": {
              "confidence": 1,
              "ipa": "sample",
              "language": "sample",
              "source": "sample",
              "speak": false,
              "text": "sample"
            },
            "unicode": {
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
              "normalized": "sample",
              "tts": {
                "confidence": 1,
                "ipa": "sample",
                "language": "sample",
                "source": "sample",
                "speak": false,
                "text": "sample"
              }
            }
          }
        }
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
          "sample": 1,
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
        {
          "path": "event.intel.processing.status",
          "tsType": "string",
          "kind": "string",
          "optional": true,
          "label": {
            "en": "Status",
            "es": "Status"
          },
          "hint": {
            "en": "AutomationIntel.processing.status",
            "es": "AutomationIntel.processing.status"
          },
          "sample": "sample"
        }
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
    }
  }
} as const satisfies Record<string, unknown>;
