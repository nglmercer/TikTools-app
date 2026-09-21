import type { ActionOptionItem } from '../types.ts';

export type VoiceOption = { value: string; label: string };

/** Voice dropdown options, keeping the current selection even when it is not listed. */
export function voiceOptions(voices: ActionOptionItem[], current: string): VoiceOption[] {
  const options = voices.map((voice) => ({ value: voice.value, label: voice.label || voice.value }));
  if (current && !options.some((option) => option.value === current)) {
    options.unshift({ value: current, label: current });
  }
  return options;
}
