import type { IconComponent } from './icon-base.tsx';
import { IconArrowDown, IconAudio, IconBarChart, IconBolt, IconChat, IconCheck, IconChevronLeft, IconChevronRight, IconClose, IconCloud, IconCode, IconConnected, IconCopy, IconCrown, IconDatabase, IconDice, IconDisconnected, IconDoc, IconDot, IconEdit, IconFlame, IconFollow, IconFormat, IconGift, IconGlobe, IconHeart, IconHttp, IconImage, IconInfo, IconJoin, IconJson, IconKeyboard, IconLink, IconLock, IconMoon, IconMore, IconPause, IconPlay, IconPlugins, IconPlus, IconPoints, IconPower, IconRadio, IconRedo, IconRefresh, IconSearch, IconServer, IconSettings, IconShare, IconShield, IconSparkles, IconSpeaker, IconSquare, IconStar, IconStop, IconSun, IconTemplate, IconTerminal, IconTikTok, IconTrash, IconTrophy, IconUndo, IconUnlock, IconUsers, IconVoice, IconVolume, IconWarning, IconWebhook } from './glyphs.tsx';

/**
 * Icon identifiers for data-driven UI (event presentation, templates). The
 * registry below is the single place that maps a name to its component, so
 * new pickers reuse names instead of embedding SVG or emoji.
 */
export type IconName =
  | 'tiktok'
  | 'chat'
  | 'gift'
  | 'gifts'
  | 'heart'
  | 'like'
  | 'likes'
  | 'users'
  | 'contributors'
  | 'follow'
  | 'share'
  | 'join'
  | 'stats'
  | 'analytics'
  | 'connected'
  | 'connect'
  | 'disconnected'
  | 'disconnect'
  | 'live'
  | 'plugin'
  | 'plugins'
  | 'points'
  | 'trophy'
  | 'speaker'
  | 'volume'
  | 'voice'
  | 'http'
  | 'webhook'
  | 'globe'
  | 'language'
  | 'code'
  | 'json'
  | 'template'
  | 'sparkles'
  | 'automation'
  | 'search'
  | 'refresh'
  | 'settings'
  | 'theme'
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
  | 'dot'
  | 'keyboard'
  | 'audio'
  | 'server'
  | 'cloud'
  | 'database'
  | 'shield'
  | 'terminal'
  | 'doc'
  | 'undo'
  | 'redo'
  | 'more'
  | 'image'
  | 'square';

export const ICONS: Record<IconName, IconComponent> = {
  tiktok: IconTikTok,
  chat: IconChat,
  gift: IconGift,
  gifts: IconGift,
  heart: IconHeart,
  like: IconHeart,
  likes: IconHeart,
  users: IconUsers,
  contributors: IconUsers,
  follow: IconFollow,
  share: IconShare,
  join: IconJoin,
  stats: IconBarChart,
  analytics: IconBarChart,
  connected: IconConnected,
  connect: IconConnected,
  disconnected: IconDisconnected,
  disconnect: IconDisconnected,
  live: IconRadio,
  plugin: IconPlugins,
  plugins: IconPlugins,
  points: IconPoints,
  trophy: IconTrophy,
  speaker: IconSpeaker,
  volume: IconVolume,
  voice: IconVoice,
  http: IconHttp,
  webhook: IconWebhook,
  globe: IconGlobe,
  language: IconGlobe,
  code: IconCode,
  json: IconJson,
  template: IconTemplate,
  sparkles: IconSparkles,
  automation: IconSparkles,
  search: IconSearch,
  refresh: IconRefresh,
  settings: IconSettings,
  theme: IconSun,
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
  keyboard: IconKeyboard,
  audio: IconAudio,
  server: IconServer,
  cloud: IconCloud,
  database: IconDatabase,
  shield: IconShield,
  terminal: IconTerminal,
  doc: IconDoc,
  undo: IconUndo,
  redo: IconRedo,
  more: IconMore,
  image: IconImage,
  square: IconSquare,
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
