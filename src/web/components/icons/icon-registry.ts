import type { IconComponent } from './icon-base.tsx';
import { IconArrowDown, IconBarChart, IconBolt, IconChat, IconCheck, IconChevronLeft, IconChevronRight, IconClose, IconCode, IconConnected, IconCopy, IconCrown, IconDice, IconDisconnected, IconDot, IconEdit, IconFlame, IconFollow, IconFormat, IconGift, IconGlobe, IconHeart, IconHttp, IconInfo, IconJoin, IconJson, IconLink, IconLock, IconMoon, IconPause, IconPlay, IconPlugins, IconPlus, IconPoints, IconPower, IconRadio, IconRefresh, IconSearch, IconSettings, IconShare, IconSparkles, IconSpeaker, IconStar, IconStop, IconSun, IconTemplate, IconTikTok, IconTrash, IconTrophy, IconUnlock, IconUsers, IconVoice, IconVolume, IconWarning, IconWebhook } from './glyphs.tsx';

/**
 * Icon identifiers for data-driven UI (event presentation, templates). The
 * registry below is the single place that maps a name to its component, so
 * new pickers reuse names instead of embedding SVG or emoji.
 */
export type IconName =
  | 'tiktok'
  | 'chat'
  | 'gift'
  | 'heart'
  | 'users'
  | 'follow'
  | 'share'
  | 'join'
  | 'stats'
  | 'connected'
  | 'disconnected'
  | 'plugin'
  | 'points'
  | 'trophy'
  | 'speaker'
  | 'volume'
  | 'voice'
  | 'http'
  | 'webhook'
  | 'globe'
  | 'code'
  | 'json'
  | 'template'
  | 'sparkles'
  | 'search'
  | 'refresh'
  | 'settings'
  | 'trash'
  | 'edit'
  | 'plus'
  | 'chevron-left'
  | 'chevron-right'
  | 'arrow-down'
  | 'check'
  | 'info'
  | 'warning'
  | 'close'
  | 'play'
  | 'pause'
  | 'stop'
  | 'format'
  | 'copy'
  | 'link'
  | 'lock'
  | 'unlock'
  | 'radio'
  | 'dice'
  | 'power'
  | 'sun'
  | 'moon'
  | 'crown'
  | 'flame'
  | 'star'
  | 'bolt'
  | 'dot';

export const ICONS: Record<IconName, IconComponent> = {
  tiktok: IconTikTok,
  chat: IconChat,
  gift: IconGift,
  heart: IconHeart,
  users: IconUsers,
  follow: IconFollow,
  share: IconShare,
  join: IconJoin,
  stats: IconBarChart,
  connected: IconConnected,
  disconnected: IconDisconnected,
  plugin: IconPlugins,
  points: IconPoints,
  trophy: IconTrophy,
  speaker: IconSpeaker,
  volume: IconVolume,
  voice: IconVoice,
  http: IconHttp,
  webhook: IconWebhook,
  globe: IconGlobe,
  code: IconCode,
  json: IconJson,
  template: IconTemplate,
  sparkles: IconSparkles,
  search: IconSearch,
  refresh: IconRefresh,
  settings: IconSettings,
  trash: IconTrash,
  edit: IconEdit,
  plus: IconPlus,
  'chevron-left': IconChevronLeft,
  'chevron-right': IconChevronRight,
  'arrow-down': IconArrowDown,
  check: IconCheck,
  info: IconInfo,
  warning: IconWarning,
  close: IconClose,
  play: IconPlay,
  pause: IconPause,
  stop: IconStop,
  format: IconFormat,
  copy: IconCopy,
  link: IconLink,
  lock: IconLock,
  unlock: IconUnlock,
  radio: IconRadio,
  dice: IconDice,
  power: IconPower,
  sun: IconSun,
  moon: IconMoon,
  crown: IconCrown,
  flame: IconFlame,
  star: IconStar,
  bolt: IconBolt,
  dot: IconDot,
};

/** Resolve an icon name with a neutral fallback for unknown values. */
export function resolveIcon(name: IconName | string): IconComponent {
  return (ICONS as Record<string, IconComponent>)[name] ?? IconDot;
}

/**
 * Strictly read an icon name from untrusted JSON (plugin uiHints, stored
 * configs). Anything outside the registry — including prototype keys — yields
 * no icon instead of the neutral fallback, so invalid data never draws a
 * misleading glyph. Plugins can only name icons, never supply SVG.
 */
export function readIconName(value: unknown): IconName | undefined {
  return typeof value === 'string' && Object.hasOwn(ICONS, value) ? (value as IconName) : undefined;
}
