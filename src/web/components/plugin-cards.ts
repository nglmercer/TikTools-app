import type {
  ActionTypeDefinition,
  PluginDescriptor,
  PluginStatus,
} from '../../automation/behavior/types.ts';
import { i18nText, type Locale } from '../i18n.ts';
import { readIconName, type IconName } from './icons/index.ts';

/** Icon tile accents, mapped onto the app palette (no gradients, no glow). */
export type PluginIconTone = 'red' | 'teal' | 'purple' | 'cyan' | 'amber' | 'slate';

export const MAX_PLUGIN_TAGS = 4;

const TONE_BY_ICON: Partial<Record<IconName, PluginIconTone>> = {
  audio: 'red',
  speaker: 'red',
  volume: 'red',
  live: 'red',
  keyboard: 'teal',
  voice: 'purple',
  webhook: 'purple',
  chat: 'cyan',
  globe: 'cyan',
  language: 'cyan',
  http: 'cyan',
  connect: 'cyan',
  connected: 'teal',
  format: 'slate',
  code: 'slate',
  json: 'slate',
  settings: 'slate',
  plugin: 'slate',
  plugins: 'slate',
  points: 'amber',
  trophy: 'amber',
  crown: 'amber',
  star: 'amber',
};

const TONES: PluginIconTone[] = ['red', 'teal', 'purple', 'cyan', 'amber', 'slate'];

const HEURISTIC_ICONS: Array<{ match: RegExp; icon: IconName }> = [
  { match: /hotkey|keyboard|shortcut/, icon: 'keyboard' },
  { match: /tts|voice|speak|sonic/, icon: 'voice' },
  { match: /audio|sound|music|wave/, icon: 'audio' },
  { match: /chat|comment/, icon: 'chat' },
  { match: /webhook|discord/, icon: 'webhook' },
  { match: /obs|stream|scene/, icon: 'live' },
  { match: /http|api|server|bridge/, icon: 'globe' },
  { match: /text|intel|language|translat|spell/, icon: 'format' },
  { match: /code|script|json|template/, icon: 'code' },
  { match: /point|reward|redeem/, icon: 'points' },
  { match: /timer|schedul|cron/, icon: 'refresh' },
];

/**
 * Card icon: an explicitly declared registry name wins, otherwise a keyword
 * heuristic over id, name, tags, and the plugin's own action tags. Unknown
 * names never draw a misleading glyph — they fall through to the heuristic,
 * then `plugin`.
 */
export function resolvePluginIcon(
  descriptor: PluginDescriptor,
  actionTypes: ActionTypeDefinition[],
): IconName {
  const named = readIconName(descriptor.icon);
  if (named) return named;
  const name = typeof descriptor.name?.default === 'string' ? descriptor.name.default : '';
  // Only the plugin's own actions count: the caller passes the global
  // catalog, and scanning it whole would resolve every card to the same
  // icon (the first heuristic rule any action tag matches).
  const ownTags = actionTypes
    .filter((action) => descriptor.actionTypeIds.includes(action.id))
    .map((action) => action.tag ?? '');
  const haystack = [descriptor.id, name, ...(descriptor.tags ?? []), ...ownTags]
    .join(' ')
    .toLowerCase();
  for (const { match, icon } of HEURISTIC_ICONS) {
    if (match.test(haystack)) return icon;
  }
  return 'plugin';
}

/** Deterministic tile accent: fixed per icon, hashed per plugin id fallback. */
export function pluginIconTone(descriptor: Pick<PluginDescriptor, 'id'>, icon: IconName): PluginIconTone {
  const direct = TONE_BY_ICON[icon];
  if (direct) return direct;
  let hash = 0;
  for (const char of descriptor.id) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return TONES[hash % TONES.length] ?? 'slate';
}

/**
 * Card chips: declared manifest tags win, otherwise the plugin's action tags
 * plus an `events` hint when it publishes its own triggers. Capped so the
 * card stays scannable.
 */
export function pluginTags(
  descriptor: PluginDescriptor,
  actionTypes: ActionTypeDefinition[],
): string[] {
  const declared = (descriptor.tags ?? []).filter(
    (tag): tag is string => typeof tag === 'string' && tag.length > 0,
  );
  if (declared.length > 0) return declared.slice(0, MAX_PLUGIN_TAGS);
  const derived: string[] = [];
  for (const id of descriptor.actionTypeIds) {
    const tag = actionTypes
      .find((action) => action.id === id)
      ?.tag?.trim()
      .toLowerCase();
    if (tag && !derived.includes(tag)) derived.push(tag);
    if (derived.length >= MAX_PLUGIN_TAGS) return derived;
  }
  if (descriptor.eventTypeIds.length > 0 && derived.length < MAX_PLUGIN_TAGS) {
    derived.push('events');
  }
  return derived;
}

/** Every query word must appear somewhere in the plugin's searchable text. */
export function matchesPluginQuery(
  locale: Locale,
  plugin: PluginStatus,
  actionTypes: ActionTypeDefinition[],
  query: string,
): boolean {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const haystack = [
    plugin.descriptor.id,
    i18nText(locale, plugin.descriptor.name),
    i18nText(locale, plugin.descriptor.description),
    ...pluginTags(plugin.descriptor, actionTypes),
  ]
    .join(' ')
    .toLowerCase();
  return words.every((word) => haystack.includes(word));
}
