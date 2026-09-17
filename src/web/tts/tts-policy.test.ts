import { describe, expect, test } from 'bun:test';

import {
  canAffordTts,
  chooseVoice,
  decideTts,
  defaultTtsSettings,
  evaluateCommentFilter,
  evaluateUserEligibility,
  normalizeHandle,
  parseTtsSettings,
  sanitizeTtsSettings,
  ttsFingerprint,
  TtsDeduper,
  type TtsSettings,
} from './tts-policy.ts';

function settings(overrides: Partial<TtsSettings> = {}): TtsSettings {
  return sanitizeTtsSettings({ ...defaultTtsSettings(), enabled: true, ...overrides });
}

describe('normalizeHandle', () => {
  test('strips @ and casefolds', () => {
    expect(normalizeHandle('@Viewer_1')).toBe('viewer_1');
    expect(normalizeHandle('  @@MiXeD ')).toBe('mixed');
    expect(normalizeHandle('')).toBe('');
  });
});

describe('sanitizeTtsSettings', () => {
  test('fills defaults for unknown input', () => {
    const clean = sanitizeTtsSettings(undefined);
    expect(clean.enabled).toBe(false);
    expect(clean.language).toBe('en');
    expect(clean.command).toBe('!tts');
  });

  test('clamps numbers and dedupes handles', () => {
    const clean = sanitizeTtsSettings({
      volume: 9,
      defaultSpeed: -1,
      pointsCost: 1_000_000,
      minTeamLevel: -5,
      topGifterCount: 500,
      allowedUsers: ['@Alice', 'alice', '  ', 'Bob'],
      specialUsers: [
        { handle: '@Cara', allowed: true, voice: ' F1 ', speed: 99, pitch: -2 },
        { handle: 'cara', allowed: false, voice: 'X', speed: 1, pitch: 1 },
      ],
    });
    expect(clean.volume).toBe(1);
    expect(clean.defaultSpeed).toBe(0.25);
    expect(clean.pointsCost).toBe(100_000);
    expect(clean.minTeamLevel).toBe(0);
    expect(clean.topGifterCount).toBe(100);
    expect(clean.allowedUsers).toEqual(['alice', 'bob']);
    expect(clean.specialUsers).toEqual([
      { handle: 'cara', allowed: true, voice: 'F1', speed: 3, pitch: 0.25 },
    ]);
  });

  test('parse falls back on corrupt JSON', () => {
    expect(parseTtsSettings(undefined).enabled).toBe(false);
    expect(parseTtsSettings('not-json').enabled).toBe(false);
    expect(parseTtsSettings('{"enabled":true}').enabled).toBe(true);
  });
});

describe('evaluateCommentFilter', () => {
  test('any mode speaks trimmed text', () => {
    const decision = evaluateCommentFilter('  hello  ', settings({ commentMode: 'any' }));
    expect(decision).toEqual({ allowed: true, spokenText: 'hello', reason: expect.any(String) });
  });

  test('empty comments never speak', () => {
    const decision = evaluateCommentFilter('   ', settings({ commentMode: 'any' }));
    expect(decision.allowed).toBe(false);
    expect(decision.spokenText).toBe('');
  });

  test('dot mode requires a prefix and strips it', () => {
    const on = settings({ commentMode: 'dot', stripCommand: true });
    expect(evaluateCommentFilter('hello', on).allowed).toBe(false);
    expect(evaluateCommentFilter('.hello world', on)).toMatchObject({ allowed: true, spokenText: 'hello world' });
    expect(evaluateCommentFilter('.', on).allowed).toBe(false);
    expect(evaluateCommentFilter('.hello', settings({ commentMode: 'dot', stripCommand: false })).spokenText).toBe('.hello');
  });

  test('slash mode mirrors dot mode', () => {
    const on = settings({ commentMode: 'slash', stripCommand: true });
    expect(evaluateCommentFilter('/speak this', on)).toMatchObject({ allowed: true, spokenText: 'speak this' });
    expect(evaluateCommentFilter('speak this', on).allowed).toBe(false);
  });

  test('command mode matches whole commands and strips the prefix', () => {
    const on = settings({ commentMode: 'command', command: '!tts', stripCommand: true });
    expect(evaluateCommentFilter('!tts hello there', on)).toMatchObject({ allowed: true, spokenText: 'hello there' });
    expect(evaluateCommentFilter('!TTS hello', on).spokenText).toBe('hello');
    expect(evaluateCommentFilter('!ttsx hello', on).allowed).toBe(false);
    expect(evaluateCommentFilter('!tts', on).allowed).toBe(false);
    const keep = settings({ commentMode: 'command', command: '!tts', stripCommand: false });
    expect(evaluateCommentFilter('!tts hello', keep)).toMatchObject({ allowed: true, spokenText: '!tts hello' });
  });
});

describe('evaluateUserEligibility', () => {
  test('blocked special users win over allow-all', () => {
    const on = settings({
      allowAllUsers: true,
      specialUsers: [{ handle: 'blocked', allowed: false, voice: '', speed: 1, pitch: 1 }],
    });
    const decision = evaluateUserEligibility({ handle: '@Blocked' }, on);
    expect(decision.allowed).toBe(false);
    expect(decision.via).toBe('special-user');
  });

  test('allowed special users override a closed room', () => {
    const on = settings({
      allowAllUsers: false,
      specialUsers: [{ handle: 'vip', allowed: true, voice: 'F1', speed: 1, pitch: 1 }],
    });
    const decision = evaluateUserEligibility({ handle: 'vip' }, on);
    expect(decision).toMatchObject({ allowed: true, via: 'special-user' });
    expect(decision.specialUser?.voice).toBe('F1');
  });

  test('allow list is handle-normalized', () => {
    const on = settings({ allowAllUsers: false, allowListedUsers: true, allowedUsers: ['alice'] });
    expect(evaluateUserEligibility({ handle: '@ALICE' }, on)).toMatchObject({ allowed: true, via: 'allow-list' });
    expect(evaluateUserEligibility({ handle: 'mallory' }, on).allowed).toBe(false);
  });

  test('subscribers need an authoritative flag', () => {
    const on = settings({ allowAllUsers: false, allowSubscribers: true });
    expect(evaluateUserEligibility({ handle: 'a', roles: { isSubscriber: true } }, on).via).toBe('subscriber');
    expect(evaluateUserEligibility({ handle: 'a' }, on).allowed).toBe(false);
    expect(evaluateUserEligibility({ handle: 'a', roles: { isSubscriber: false } }, on).allowed).toBe(false);
  });

  test('follower/moderator/team/top-gifter never grant without data', () => {
    const on = settings({
      allowAllUsers: false,
      allowFollowers: true,
      allowModerators: true,
      allowTeamMembers: true,
      minTeamLevel: 5,
      allowTopGifters: true,
      topGifterCount: 3,
    });
    expect(evaluateUserEligibility({ handle: 'ghost' }, on).allowed).toBe(false);
    expect(evaluateUserEligibility({ handle: 'f', roles: { isFollower: true } }, on).via).toBe('follower');
    expect(evaluateUserEligibility({ handle: 'm', roles: { isModerator: true } }, on).via).toBe('moderator');
    expect(
      evaluateUserEligibility({ handle: 't', roles: { isTeamMember: true, teamLevel: 9 } }, on).via,
    ).toBe('team-member');
    expect(
      evaluateUserEligibility({ handle: 't', roles: { isTeamMember: true, teamLevel: 1 } }, on).allowed,
    ).toBe(false);
    expect(
      evaluateUserEligibility({ handle: 'g', roles: { isTopGifter: true, topGifterRank: 2 } }, on).via,
    ).toBe('top-gifter');
    expect(
      evaluateUserEligibility({ handle: 'g', roles: { isTopGifter: true, topGifterRank: 9 } }, on).allowed,
    ).toBe(false);
  });
});

describe('chooseVoice', () => {
  test('special voice wins, then random, then default', () => {
    const on = settings({ defaultVoice: 'D1', randomVoice: true });
    expect(chooseVoice(on, 'S1', ['A', 'B'], () => 0.9)).toBe('S1');
    expect(chooseVoice(on, '', ['A', 'B'], () => 0)).toBe('A');
    expect(chooseVoice(on, '  ', ['A', 'B'], () => 0.99)).toBe('B');
    expect(chooseVoice(settings({ defaultVoice: 'D1' }), '', ['A'])).toBe('D1');
    expect(chooseVoice(settings({ defaultVoice: 'D1', randomVoice: true }), '', [])).toBe('D1');
  });
});

describe('points', () => {
  test('free TTS always affordable; paid TTS rejects short balances', () => {
    expect(canAffordTts(undefined, settings({ chargePoints: false }))).toBe(true);
    expect(canAffordTts(5, settings({ chargePoints: true, pointsCost: 10 }))).toBe(false);
    expect(canAffordTts(10, settings({ chargePoints: true, pointsCost: 10 }))).toBe(true);
    expect(canAffordTts(undefined, settings({ chargePoints: true, pointsCost: 10 }))).toBe(false);
  });
});

describe('decideTts', () => {
  test('disabled TTS never speaks', () => {
    const decision = decideTts({
      comment: 'hello',
      author: { handle: 'a' },
      settings: settings({ enabled: false }),
      availableVoices: [],
    });
    expect(decision.speak).toBe(false);
    expect(decision.reason).toMatch(/disabled/i);
  });

  test('comment filter gates before eligibility (old shouldRead bug)', () => {
    const decision = decideTts({
      comment: 'hello without prefix',
      author: { handle: 'a' },
      settings: settings({ commentMode: 'dot' }),
      availableVoices: [],
    });
    expect(decision.speak).toBe(false);
    expect(decision.spokenText).toBe('');
  });

  test('insufficient points reject without a voice', () => {
    const decision = decideTts({
      comment: 'hello',
      author: { handle: 'a', points: 1 },
      settings: settings({ chargePoints: true, pointsCost: 50 }),
      availableVoices: ['V1'],
    });
    expect(decision.speak).toBe(false);
    expect(decision.pointsCost).toBe(50);
    expect(decision.voice).toBe('');
  });

  test('happy path returns stripped text and chosen voice', () => {
    const decision = decideTts({
      comment: '!tts speak me',
      author: { handle: 'a', points: 100 },
      settings: settings({
        commentMode: 'command',
        command: '!tts',
        chargePoints: true,
        pointsCost: 10,
        defaultVoice: 'D1',
        language: 'es',
      }),
      availableVoices: ['V1'],
    });
    expect(decision).toMatchObject({
      speak: true,
      spokenText: 'speak me',
      voice: 'D1',
      language: 'es',
      pointsCost: 10,
      via: 'all-users',
    });
  });
});

describe('TtsDeduper', () => {
  test('drops replays inside the window only', () => {
    const deduper = new TtsDeduper({ windowMs: 1000 });
    const fingerprint = ttsFingerprint('@Alice', 'Hello');
    expect(deduper.claim(fingerprint, 1000)).toBe(true);
    expect(deduper.claim(fingerprint, 1500)).toBe(false);
    expect(deduper.claim(fingerprint, 2500)).toBe(true);
    expect(ttsFingerprint('alice', 'hello')).toBe(fingerprint);
  });
});
