import type { SpecialTtsUser, TtsCommentMode, TtsSettings } from './types.ts';

export const TTS_LIMITS = {
  minSpeed: 0.25,
  maxSpeed: 3,
  minPitch: 0.25,
  maxPitch: 2,
  minVolume: 0,
  maxVolume: 1,
  minPointsCost: 0,
  maxPointsCost: 100_000,
  minTeamLevel: 0,
  maxTeamLevel: 100,
  minTopGifterCount: 1,
  maxTopGifterCount: 100,
  maxCommandLength: 32,
  maxLanguageLength: 16,
  maxVoiceLength: 128,
  maxHandleLength: 64,
  maxAllowedUsers: 500,
  maxSpecialUsers: 500,
  maxCommentLength: 4_096,
} as const;

/** `true` while SonicBoom synthesis ignores speed/pitch controls. */
export const TTS_SPEED_PITCH_UNSUPPORTED = true;

export function defaultTtsSettings(): TtsSettings {
  return {
    enabled: false,
    language: 'en',
    defaultVoice: '',
    randomVoice: false,
    defaultSpeed: 1,
    defaultPitch: 1,
    volume: 1,

    allowAllUsers: true,
    allowFollowers: false,
    allowSubscribers: false,
    allowModerators: false,
    allowTeamMembers: false,
    minTeamLevel: 0,
    allowTopGifters: false,
    topGifterCount: 10,
    allowListedUsers: false,
    allowedUsers: [],

    commentMode: 'any',
    command: '!tts',
    stripCommand: true,

    chargePoints: false,
    pointsCost: 0,

    specialUsers: [],
  };
}

/** Normalizes a TikTok handle for comparison: trims, strips `@`, casefolds. */
export function normalizeHandle(handle: string): string {
  return handle.trim().replace(/^@+/, '').toLowerCase();
}

export function clampNumber(value: number, min: number, max: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  if (value < min) return min;
  if (value > max) return max;
  return value;
}

function toBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === 'boolean' ? value : fallback;
}

function toTrimmedString(value: unknown, maxLength: number, fallback: string): string {
  if (typeof value !== 'string') return fallback;
  const trimmed = value.trim();
  if (!trimmed) return fallback;
  return trimmed.slice(0, maxLength);
}

function toVoiceString(value: unknown): string {
  if (typeof value !== 'string') return '';
  return value.trim().slice(0, TTS_LIMITS.maxVoiceLength);
}

function toHandleList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const clean: string[] = [];
  for (const entry of value) {
    if (typeof entry !== 'string') continue;
    const handle = normalizeHandle(entry);
    if (!handle || handle.length > TTS_LIMITS.maxHandleLength) continue;
    if (seen.has(handle)) continue;
    seen.add(handle);
    clean.push(handle);
    if (clean.length >= TTS_LIMITS.maxAllowedUsers) break;
  }
  return clean;
}

function toSpecialUsers(value: unknown): SpecialTtsUser[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const clean: SpecialTtsUser[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) continue;
    const record = entry as Record<string, unknown>;
    const handle = typeof record.handle === 'string' ? normalizeHandle(record.handle) : '';
    if (!handle || handle.length > TTS_LIMITS.maxHandleLength) continue;
    if (seen.has(handle)) continue;
    seen.add(handle);
    clean.push({
      handle,
      allowed: toBoolean(record.allowed, true),
      voice: toVoiceString(record.voice),
      speed: clampNumber(
        typeof record.speed === 'number' ? record.speed : 1,
        TTS_LIMITS.minSpeed,
        TTS_LIMITS.maxSpeed,
        1,
      ),
      pitch: clampNumber(
        typeof record.pitch === 'number' ? record.pitch : 1,
        TTS_LIMITS.minPitch,
        TTS_LIMITS.maxPitch,
        1,
      ),
    });
    if (clean.length >= TTS_LIMITS.maxSpecialUsers) break;
  }
  return clean;
}

function toCommentMode(value: unknown): TtsCommentMode {
  return value === 'dot' || value === 'slash' || value === 'command' ? value : 'any';
}

/** Coerces unknown persisted state into valid settings with clamped numbers. */
export function sanitizeTtsSettings(input: unknown): TtsSettings {
  const defaults = defaultTtsSettings();
  if (!input || typeof input !== 'object' || Array.isArray(input)) return defaults;
  const record = input as Record<string, unknown>;
  const pointsCost = typeof record.pointsCost === 'number' ? record.pointsCost : defaults.pointsCost;
  const minTeamLevel = typeof record.minTeamLevel === 'number' ? record.minTeamLevel : defaults.minTeamLevel;
  const topGifterCount =
    typeof record.topGifterCount === 'number' ? record.topGifterCount : defaults.topGifterCount;
  const defaultSpeed = typeof record.defaultSpeed === 'number' ? record.defaultSpeed : defaults.defaultSpeed;
  const defaultPitch = typeof record.defaultPitch === 'number' ? record.defaultPitch : defaults.defaultPitch;
  const volume = typeof record.volume === 'number' ? record.volume : defaults.volume;
  return {
    enabled: toBoolean(record.enabled, defaults.enabled),
    language: toTrimmedString(record.language, TTS_LIMITS.maxLanguageLength, defaults.language),
    defaultVoice: toVoiceString(record.defaultVoice),
    randomVoice: toBoolean(record.randomVoice, defaults.randomVoice),
    defaultSpeed: clampNumber(defaultSpeed, TTS_LIMITS.minSpeed, TTS_LIMITS.maxSpeed, 1),
    defaultPitch: clampNumber(defaultPitch, TTS_LIMITS.minPitch, TTS_LIMITS.maxPitch, 1),
    volume: clampNumber(volume, TTS_LIMITS.minVolume, TTS_LIMITS.maxVolume, 1),

    allowAllUsers: toBoolean(record.allowAllUsers, defaults.allowAllUsers),
    allowFollowers: toBoolean(record.allowFollowers, defaults.allowFollowers),
    allowSubscribers: toBoolean(record.allowSubscribers, defaults.allowSubscribers),
    allowModerators: toBoolean(record.allowModerators, defaults.allowModerators),
    allowTeamMembers: toBoolean(record.allowTeamMembers, defaults.allowTeamMembers),
    minTeamLevel: Math.round(
      clampNumber(minTeamLevel, TTS_LIMITS.minTeamLevel, TTS_LIMITS.maxTeamLevel, 0),
    ),
    allowTopGifters: toBoolean(record.allowTopGifters, defaults.allowTopGifters),
    topGifterCount: Math.round(
      clampNumber(topGifterCount, TTS_LIMITS.minTopGifterCount, TTS_LIMITS.maxTopGifterCount, 10),
    ),
    allowListedUsers: toBoolean(record.allowListedUsers, defaults.allowListedUsers),
    allowedUsers: toHandleList(record.allowedUsers),

    commentMode: toCommentMode(record.commentMode),
    command: toTrimmedString(record.command, TTS_LIMITS.maxCommandLength, defaults.command),
    stripCommand: toBoolean(record.stripCommand, defaults.stripCommand),

    chargePoints: toBoolean(record.chargePoints, defaults.chargePoints),
    pointsCost: Math.round(
      clampNumber(pointsCost, TTS_LIMITS.minPointsCost, TTS_LIMITS.maxPointsCost, 0),
    ),

    specialUsers: toSpecialUsers(record.specialUsers),
  };
}

/** App-state storage key for one plugin's TTS settings. */
export function ttsSettingsKey(pluginId: string): string {
  return `tts.settings:${pluginId}`;
}

export function serializeTtsSettings(settings: TtsSettings): string {
  return JSON.stringify(sanitizeTtsSettings(settings));
}

export function parseTtsSettings(raw: string | undefined): TtsSettings {
  if (!raw) return defaultTtsSettings();
  try {
    return sanitizeTtsSettings(JSON.parse(raw) as unknown);
  } catch {
    return defaultTtsSettings();
  }
}
