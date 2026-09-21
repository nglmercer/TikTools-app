import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { TtsDeduper } from './deduper.ts';
import { decideTts } from './eligibility.ts';
import { defaultTtsSettings } from './settings.ts';
import * as barrel from './tts-policy.ts';
import type {
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
} from './tts-policy.ts';
import { chooseVoice } from './voice.ts';

/**
 * The compatibility barrel must keep exporting the exact pre-split
 * `tts-policy.ts` API (18 values + 11 types). Value parity is checked at
 * runtime below; every type import above is referenced by
 * `BarrelTypeParity`, so removing a type export fails `bun run typecheck`.
 */
export type BarrelTypeParity = {
  author: TtsAuthor;
  comment: TtsCommentDecision;
  decision: TtsDecision;
  deduperOptions: TtsDeduperOptions;
  eligibility: TtsEligibility;
  log: TtsLogEntry;
  mode: TtsCommentMode;
  request: TtsRequest;
  roles: TtsAuthorRoles;
  settings: TtsSettings;
  special: SpecialTtsUser;
};

const EXPECTED_VALUES = [
  'TTS_LIMITS',
  'TTS_SPEED_PITCH_UNSUPPORTED',
  'TtsDeduper',
  'canAffordTts',
  'chooseVoice',
  'clampNumber',
  'decideTts',
  'defaultTtsSettings',
  'evaluateCommentFilter',
  'evaluateUserEligibility',
  'findSpecialUser',
  'firstAvailableVoice',
  'normalizeHandle',
  'parseTtsSettings',
  'sanitizeTtsSettings',
  'serializeTtsSettings',
  'ttsFingerprint',
  'ttsSettingsKey',
];

const EXPECTED_TYPES = [
  'SpecialTtsUser',
  'TtsAuthor',
  'TtsAuthorRoles',
  'TtsCommentDecision',
  'TtsCommentMode',
  'TtsDecision',
  'TtsDeduperOptions',
  'TtsEligibility',
  'TtsLogEntry',
  'TtsRequest',
  'TtsSettings',
];

/** Type names re-exported via `export type {…}` or inline `type X` items. */
function typeExportsOf(source: string): string[] {
  const names: string[] = [];
  for (const block of source.matchAll(/^export\s+(type\s+)?\{([^}]*)\}/gm)) {
    const wholeBlockIsType = (block[1] ?? '').trim() === 'type';
    for (const part of (block[2] ?? '').split(',')) {
      const trimmed = part.trim();
      if (!trimmed) continue;
      const isType = wholeBlockIsType || trimmed.startsWith('type ');
      if (!isType) continue;
      const name = trimmed.split(/\s+as\s+/).pop()?.replace(/^type\s+/, '').trim() ?? '';
      if (name) names.push(name);
    }
  }
  return names.sort();
}

test('barrel exports the exact pre-split value API', () => {
  expect(Object.keys(barrel).sort()).toEqual([...EXPECTED_VALUES].sort());
});

test('barrel re-exports the exact pre-split type API', () => {
  const source = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'tts-policy.ts'), 'utf8');
  expect(typeExportsOf(source)).toEqual([...EXPECTED_TYPES].sort());
});

test('split-module exports are the same bindings (no barrel drift)', () => {
  expect(barrel.decideTts).toBe(decideTts);
  expect(barrel.defaultTtsSettings).toBe(defaultTtsSettings);
  expect(barrel.chooseVoice).toBe(chooseVoice);
  expect(barrel.TtsDeduper).toBe(TtsDeduper);
});
