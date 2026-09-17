/** Host-owned TTS policy for plugin `tts` page sections.
 *
 * All filtering and eligibility lives here so Vue components stay thin and
 * the same decision function serves the voice tester, automatic chat TTS,
 * and unit tests. The runtime order is:
 *
 * chat event → enabled? → comment filter → special-user rules →
 * allowed-user rules → points check → choose voice.
 *
 * Role checks never fake metadata: follower, moderator, team-member, and
 * top-gifter rules grant access only when the caller supplies authoritative
 * role data. Missing role data means "not granted by this rule", never
 * "granted". Subscriber, allow-list, and special-user rules work today;
 * the remaining roles are structured so they light up once the host
 * provides authoritative flags.
 */

export type TtsCommentMode = 'any' | 'dot' | 'slash' | 'command';

export type SpecialTtsUser = {
  handle: string;
  allowed: boolean;
  voice: string;
  speed: number;
  pitch: number;
};

export type TtsSettings = {
  enabled: boolean;
  language: string;
  defaultVoice: string;
  randomVoice: boolean;
  defaultSpeed: number;
  defaultPitch: number;
  volume: number;

  allowAllUsers: boolean;
  allowFollowers: boolean;
  allowSubscribers: boolean;
  allowModerators: boolean;
  allowTeamMembers: boolean;
  minTeamLevel: number;
  allowTopGifters: boolean;
  topGifterCount: number;
  allowListedUsers: boolean;
  allowedUsers: string[];

  commentMode: TtsCommentMode;
  command: string;
  stripCommand: boolean;

  chargePoints: boolean;
  pointsCost: number;

  specialUsers: SpecialTtsUser[];
};

/** Authoritative role flags for one chat author. Absent means unknown. */
export type TtsAuthorRoles = {
  isSubscriber?: boolean;
  isFollower?: boolean;
  isModerator?: boolean;
  isTeamMember?: boolean;
  teamLevel?: number;
  isTopGifter?: boolean;
  topGifterRank?: number;
};

export type TtsAuthor = {
  handle: string;
  points?: number;
  roles?: TtsAuthorRoles;
};

export type TtsCommentDecision = {
  allowed: boolean;
  spokenText: string;
  reason: string;
};

export type TtsEligibility = {
  allowed: boolean;
  reason: string;
  via: 'special-user' | 'all-users' | 'allow-list' | 'subscriber' | 'follower' | 'moderator' | 'team-member' | 'top-gifter' | 'none';
  specialUser?: SpecialTtsUser;
};

export type TtsDecision = {
  speak: boolean;
  reason: string;
  spokenText: string;
  voice: string;
  language: string;
  pointsCost: number;
  via: TtsEligibility['via'];
  specialUser?: SpecialTtsUser;
};

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

/** Applies the comment-mode filter and strips the command prefix when asked. */
export function evaluateCommentFilter(comment: string, settings: TtsSettings): TtsCommentDecision {
  const text = comment.slice(0, TTS_LIMITS.maxCommentLength);
  const trimmed = text.trim();
  if (!trimmed) {
    return { allowed: false, spokenText: '', reason: 'Empty comments are never spoken.' };
  }
  switch (settings.commentMode) {
    case 'any':
      return { allowed: true, spokenText: trimmed, reason: 'Any comment may be spoken.' };
    case 'dot': {
      if (!trimmed.startsWith('.') || trimmed.length < 2) {
        return { allowed: false, spokenText: '', reason: 'Only comments starting with `.` are spoken.' };
      }
      const spoken = settings.stripCommand ? trimmed.slice(1).trim() : trimmed;
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping `.`.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Dot-prefixed comment.' };
    }
    case 'slash': {
      if (!trimmed.startsWith('/') || trimmed.length < 2) {
        return { allowed: false, spokenText: '', reason: 'Only comments starting with `/` are spoken.' };
      }
      const spoken = settings.stripCommand ? trimmed.slice(1).trim() : trimmed;
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping `/`.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Slash-prefixed comment.' };
    }
    case 'command': {
      const command = settings.command.trim();
      if (!command) {
        return { allowed: false, spokenText: '', reason: 'No TTS command is configured.' };
      }
      const lowered = trimmed.toLowerCase();
      const prefix = command.toLowerCase();
      if (lowered !== prefix && !lowered.startsWith(`${prefix} `)) {
        return { allowed: false, spokenText: '', reason: `Only comments starting with \`${command}\` are spoken.` };
      }
      if (!settings.stripCommand) {
        return { allowed: true, spokenText: trimmed, reason: 'Command-prefixed comment.' };
      }
      const spoken = trimmed.slice(command.length).trim();
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping the command.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Command-prefixed comment.' };
    }
  }
}

export function findSpecialUser(handle: string, settings: TtsSettings): SpecialTtsUser | undefined {
  const clean = normalizeHandle(handle);
  if (!clean) return undefined;
  return settings.specialUsers.find((entry) => entry.handle === clean);
}

/** Special-user allow/block plus the global allow rules, in priority order. */
export function evaluateUserEligibility(author: TtsAuthor, settings: TtsSettings): TtsEligibility {
  const special = findSpecialUser(author.handle, settings);
  if (special) {
    if (!special.allowed) {
      return { allowed: false, reason: 'This user is blocked in Special Users.', via: 'special-user', specialUser: special };
    }
    return { allowed: true, reason: 'Allowed by Special Users.', via: 'special-user', specialUser: special };
  }
  if (settings.allowAllUsers) {
    return { allowed: true, reason: 'All users may use TTS.', via: 'all-users' };
  }
  const clean = normalizeHandle(author.handle);
  if (settings.allowListedUsers && clean && settings.allowedUsers.includes(clean)) {
    return { allowed: true, reason: 'Listed in Allowed Users.', via: 'allow-list' };
  }
  const roles = author.roles ?? {};
  if (settings.allowSubscribers && roles.isSubscriber === true) {
    return { allowed: true, reason: 'Subscribers may use TTS.', via: 'subscriber' };
  }
  // The remaining roles grant access only with authoritative data. A missing
  // flag means "unknown", which never satisfies the rule.
  if (settings.allowFollowers && roles.isFollower === true) {
    return { allowed: true, reason: 'Followers may use TTS.', via: 'follower' };
  }
  if (settings.allowModerators && roles.isModerator === true) {
    return { allowed: true, reason: 'Moderators may use TTS.', via: 'moderator' };
  }
  if (settings.allowTeamMembers && roles.isTeamMember === true) {
    const level = typeof roles.teamLevel === 'number' ? roles.teamLevel : 0;
    if (level >= settings.minTeamLevel) {
      return { allowed: true, reason: 'Team members may use TTS.', via: 'team-member' };
    }
    return { allowed: false, reason: `Team level ${settings.minTeamLevel} or higher is required.`, via: 'none' };
  }
  if (settings.allowTopGifters && roles.isTopGifter === true) {
    const rank = typeof roles.topGifterRank === 'number' ? roles.topGifterRank : Number.MAX_SAFE_INTEGER;
    if (rank <= settings.topGifterCount) {
      return { allowed: true, reason: 'Top gifters may use TTS.', via: 'top-gifter' };
    }
    return { allowed: false, reason: `Only the top ${settings.topGifterCount} gifters may use TTS.`, via: 'none' };
  }
  return { allowed: false, reason: 'This user is not allowed to use TTS.', via: 'none' };
}

/** First usable voice from a loaded list, or '' when none is known. */
export function firstAvailableVoice(availableVoices: readonly string[]): string {
  return availableVoices.map((voice) => voice.trim()).find((voice) => voice.length > 0) ?? '';
}

/**
 * Voice priority: special-user voice → random voice → default voice →
 * first available voice. The trailing fallback matters: servers such as
 * SonicBoom 400 on a present-but-empty `voice=` param (their built-in
 * default only applies when the param is absent, which templates cannot
 * express), so an empty resolution must never be sent while voices are
 * known. Only when no voice list was ever loaded can '' still come out,
 * and then the server error names the problem.
 */
export function chooseVoice(
  settings: TtsSettings,
  specialVoice: string | undefined,
  availableVoices: readonly string[],
  randomFn: () => number = Math.random,
): string {
  const special = (specialVoice ?? '').trim();
  if (special) return special;
  const pool = availableVoices.map((voice) => voice.trim()).filter(Boolean);
  if (settings.randomVoice && pool.length > 0) {
    const index = Math.floor(randomFn() * pool.length);
    const picked = pool[Math.min(Math.max(index, 0), pool.length - 1)];
    if (picked) return picked;
  }
  return settings.defaultVoice.trim() || firstAvailableVoice(availableVoices);
}

/** `true` when the viewer can cover the per-message points cost. */
export function canAffordTts(points: number | undefined, settings: TtsSettings): boolean {
  if (!settings.chargePoints || settings.pointsCost <= 0) return true;
  return typeof points === 'number' && Number.isFinite(points) && points >= settings.pointsCost;
}

export type TtsRequest = {
  comment: string;
  author: TtsAuthor;
  settings: TtsSettings;
  availableVoices: readonly string[];
  randomFn?: () => number;
};

/** Single full-pipeline decision. Callers must check `speak` before acting:
 * computing eligibility never speaks by itself (old `shouldRead` bug fix). */
export function decideTts(request: TtsRequest): TtsDecision {
  const { comment, author, settings } = request;
  const idle = (reason: string): TtsDecision => ({
    speak: false,
    reason,
    spokenText: '',
    voice: '',
    language: settings.language,
    pointsCost: 0,
    via: 'none',
  });
  if (!settings.enabled) {
    return idle('TTS is disabled.');
  }
  const commentDecision = evaluateCommentFilter(comment, settings);
  if (!commentDecision.allowed) {
    return idle(commentDecision.reason);
  }
  const eligibility = evaluateUserEligibility(author, settings);
  if (!eligibility.allowed) {
    return {
      speak: false,
      reason: eligibility.reason,
      spokenText: '',
      voice: '',
      language: settings.language,
      pointsCost: 0,
      via: eligibility.via,
      specialUser: eligibility.specialUser,
    };
  }
  const pointsCost = settings.chargePoints ? Math.max(0, Math.round(settings.pointsCost)) : 0;
  if (pointsCost > 0 && !canAffordTts(author.points, settings)) {
    return {
      speak: false,
      reason: `Needs ${pointsCost} points to use TTS.`,
      spokenText: '',
      voice: '',
      language: settings.language,
      pointsCost,
      via: eligibility.via,
      specialUser: eligibility.specialUser,
    };
  }
  const voice = chooseVoice(
    settings,
    eligibility.specialUser?.voice,
    request.availableVoices,
    request.randomFn,
  );
  return {
    speak: true,
    reason: eligibility.reason,
    spokenText: commentDecision.spokenText,
    voice,
    language: settings.language,
    pointsCost,
    via: eligibility.via,
    specialUser: eligibility.specialUser,
  };
}

/** Fingerprint for one speakable chat line, used for de-duplication. */
export function ttsFingerprint(handle: string, spokenText: string): string {
  return `${normalizeHandle(handle)}\n${spokenText.trim().toLowerCase()}`;
}

export type TtsDeduperOptions = {
  windowMs?: number;
  maxEntries?: number;
};

/** Drops repeated deliveries of the same handle+text within a short window.
 * Guards against transport replays and double event emission speaking twice.
 * Points deduction must happen only after `claim` returns true, exactly once
 * per claimed fingerprint, so a replay can never double-charge. */
export class TtsDeduper {
  private readonly windowMs: number;
  private readonly maxEntries: number;
  private readonly seen = new Map<string, number>();

  constructor(options: TtsDeduperOptions = {}) {
    this.windowMs = Math.max(0, options.windowMs ?? 1500);
    this.maxEntries = Math.max(1, options.maxEntries ?? 500);
  }

  claim(fingerprint: string, now: number = Date.now()): boolean {
    const last = this.seen.get(fingerprint);
    if (last !== undefined && now - last < this.windowMs) {
      return false;
    }
    this.seen.set(fingerprint, now);
    if (this.seen.size > this.maxEntries) {
      const oldest = [...this.seen.entries()].sort((a, b) => a[1] - b[1])[0];
      if (oldest) this.seen.delete(oldest[0]);
    }
    return true;
  }

  clear(): void {
    this.seen.clear();
  }
}

export type TtsLogEntry = {
  id: number;
  at: number;
  ok: boolean;
  source: 'tester' | 'auto';
  text: string;
  voice: string;
  summary: string;
};

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
