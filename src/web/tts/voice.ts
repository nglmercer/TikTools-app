import type { TtsSettings } from './types.ts';

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
