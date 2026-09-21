use ttl_live_events::method as live_method;

/// How a normalized JSON value derives from its protobuf source field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTransform {
    /// Value passes through unchanged (string stays string, integer stays integer).
    Native,
    /// Normalized `u64` rendered as a decimal string. The `u64` is the
    /// `ttl-live-events` normalized intermediate (`count()` clamps the raw
    /// protobuf integer at zero); the raw descriptor type is usually `int64`
    /// and is recorded separately as `protobufType`.
    U64ToString,
    /// Normalized integer rendered as a decimal string.
    IntegerToString,
    /// Non-zero integer becomes `true`.
    NonzeroToBoolean,
    /// Signed protobuf integer clamped at zero into an unsigned count.
    NormalizedUnsigned,
}

impl SourceTransform {
    /// Kebab-case name used in `x-native-source` and the TS registry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::U64ToString => "u64-to-string",
            Self::IntegerToString => "integer-to-string",
            Self::NonzeroToBoolean => "nonzero-to-boolean",
            Self::NormalizedUnsigned => "normalized-unsigned",
        }
    }
}

/// One TikTools field's projection from its protobuf source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Projection {
    /// `$defs` entry the field belongs to, e.g. `GiftAutomationData`.
    pub(super) contract: &'static str,
    /// camelCase JSON key, e.g. `giftId`.
    pub(super) field: &'static str,
    /// Vendor method, one of `ttl_live_events::method::*`.
    pub(super) method: &'static str,
    /// Dotted protobuf path, e.g. `gift.name` for the nested gift detail.
    pub(super) path: &'static str,
    pub(super) transform: SourceTransform,
    /// Expected schemars JSON type: `string`, `boolean`, or `integer`.
    pub(super) json_type: &'static str,
}

/// The full TikTools-field <- protobuf-source projection table.
///
/// TikTools-only fields (`method`, `msgId`, `isHistory`, `streakable`,
/// `giftIconUrl`) are deliberately absent: they must never claim provenance.
pub(super) const PROJECTIONS: &[Projection] = &[
    // ChatAutomationData <- WebcastChatMessage
    Projection {
        contract: "ChatAutomationData",
        field: "comment",
        method: live_method::CHAT,
        path: "content",
        transform: SourceTransform::Native,
        json_type: "string",
    },
    // GiftAutomationData <- WebcastGiftMessage
    Projection {
        contract: "GiftAutomationData",
        field: "giftId",
        method: live_method::GIFT,
        path: "gift_id",
        transform: SourceTransform::U64ToString,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "giftName",
        method: live_method::GIFT,
        path: "gift.name",
        transform: SourceTransform::Native,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "diamondCount",
        method: live_method::GIFT,
        path: "gift.diamond_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "repeatCount",
        method: live_method::GIFT,
        path: "repeat_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "comboCount",
        method: live_method::GIFT,
        path: "combo_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "groupId",
        method: live_method::GIFT,
        path: "group_id",
        transform: SourceTransform::IntegerToString,
        json_type: "string",
    },
    Projection {
        contract: "GiftAutomationData",
        field: "repeatEnd",
        method: live_method::GIFT,
        path: "repeat_end",
        transform: SourceTransform::NonzeroToBoolean,
        json_type: "boolean",
    },
    // LikeAutomationData <- WebcastLikeMessage
    Projection {
        contract: "LikeAutomationData",
        field: "count",
        method: live_method::LIKE,
        path: "count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "LikeAutomationData",
        field: "total",
        method: live_method::LIKE,
        path: "total",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    // SocialAutomationData <- WebcastSocialMessage
    Projection {
        contract: "SocialAutomationData",
        field: "action",
        method: live_method::SOCIAL,
        path: "action",
        transform: SourceTransform::Native,
        json_type: "integer",
    },
    Projection {
        contract: "SocialAutomationData",
        field: "followCount",
        method: live_method::SOCIAL,
        path: "follow_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "SocialAutomationData",
        field: "shareCount",
        method: live_method::SOCIAL,
        path: "share_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    // MemberAutomationData <- WebcastMemberMessage
    Projection {
        contract: "MemberAutomationData",
        field: "memberCount",
        method: live_method::MEMBER,
        path: "member_count",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "MemberAutomationData",
        field: "action",
        method: live_method::MEMBER,
        path: "action",
        transform: SourceTransform::Native,
        json_type: "integer",
    },
    // RoomStatsAutomationData <- WebcastRoomUserSeqMessage. `viewers` is the
    // normalized `total`; the raw name never leaks into the contract.
    Projection {
        contract: "RoomStatsAutomationData",
        field: "viewers",
        method: live_method::ROOM_USER,
        path: "total",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "totalUsers",
        method: live_method::ROOM_USER,
        path: "total_user",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "popularity",
        method: live_method::ROOM_USER,
        path: "popularity",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
    Projection {
        contract: "RoomStatsAutomationData",
        field: "anonymous",
        method: live_method::ROOM_USER,
        path: "anonymous",
        transform: SourceTransform::NormalizedUnsigned,
        json_type: "integer",
    },
];
