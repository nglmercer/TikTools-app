// THIS FILE IS GENERATED. Run bun run contracts:generate.

export const EVENT_REGISTRY_VERSION = 3 as const;
export const GENERATED_EVENT_REGISTRY = {
  "version": 3,
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
