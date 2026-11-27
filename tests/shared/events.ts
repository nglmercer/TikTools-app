/** Event kinds and trigger types used by the live feed and automations. */
export const EVENT_KIND = {
  tiktokChat: 'tiktok.chat',
  tiktokGift: 'tiktok.gift',
  tiktokLike: 'tiktok.like',
  tiktokShare: 'tiktok.share',
  hotkeyPressed: 'hotkey.pressed',
} as const;
export type EventKind = (typeof EVENT_KIND)[keyof typeof EVENT_KIND];
