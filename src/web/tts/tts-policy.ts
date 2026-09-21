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
 *
 * Compatibility barrel: the implementation moved into sibling modules
 * (`types.ts`, `settings.ts`, `eligibility.ts`, `voice.ts`, `deduper.ts`);
 * every export below preserves the original `tts-policy.ts` public API.
 */
export type {
  SpecialTtsUser,
  TtsAuthor,
  TtsAuthorRoles,
  TtsCommentDecision,
  TtsCommentMode,
  TtsDecision,
  TtsDeduperOptions,
  TtsEligibility,
  TtsLogEntry,
  TtsRequest,
  TtsSettings,
} from './types.ts';
export {
  clampNumber,
  defaultTtsSettings,
  normalizeHandle,
  parseTtsSettings,
  sanitizeTtsSettings,
  serializeTtsSettings,
  TTS_LIMITS,
  TTS_SPEED_PITCH_UNSUPPORTED,
  ttsSettingsKey,
} from './settings.ts';
export {
  canAffordTts,
  decideTts,
  evaluateCommentFilter,
  evaluateUserEligibility,
  findSpecialUser,
} from './eligibility.ts';
export { chooseVoice, firstAvailableVoice } from './voice.ts';
export { TtsDeduper, ttsFingerprint } from './deduper.ts';
