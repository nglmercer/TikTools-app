/** Built-in event types published by the host automation boundary. */
export const BUILTIN_EVENT_TYPES = [
  'tiktok.chat',
  'tiktok.gift',
  'tiktok.like',
  'tiktok.follow',
  'tiktok.share',
  'tiktok.join',
  'tiktok.social',
  'tiktok.room_stats',
  'tiktok.connected',
  'tiktok.disconnected',
  'points.awarded',
  'plugin.emit',
] as const;

export type AutomationEventType = typeof BUILTIN_EVENT_TYPES[number];

/** Data contract selected for each built-in event envelope. */
export const BUILTIN_EVENT_CONTRACTS: Record<AutomationEventType, string> = {
  'tiktok.chat': 'ChatAutomationData',
  'tiktok.gift': 'GiftAutomationData',
  'tiktok.like': 'LikeAutomationData',
  'tiktok.follow': 'SocialAutomationData',
  'tiktok.share': 'SocialAutomationData',
  'tiktok.join': 'MemberAutomationData',
  'tiktok.social': 'SocialAutomationData',
  'tiktok.room_stats': 'RoomStatsAutomationData',
  'tiktok.connected': 'ConnectionAutomationData',
  'tiktok.disconnected': 'ConnectionAutomationData',
  'points.awarded': 'PointsAwardedAutomationData',
  'plugin.emit': 'PluginEmitAutomationData',
};
