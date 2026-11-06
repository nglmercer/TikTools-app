import { expect, test } from 'vitest';

import presetJson from '../../../../../tests/fixtures/profiles/minecraft-giftwall.tikprofile.json';
import { instantiateProfile, parseRuleProfile } from './rule-profiles-state.ts';

test('minecraft gift-wall preset parses on the desktop path', () => {
  const parsed = parseRuleProfile(presetJson);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(parsed.profile.id).toBe('minecraft-giftwall');
  expect(parsed.profile.templates).toHaveLength(16);
  const triggers = new Set(parsed.profile.templates.map((entry) => entry.template.event.trigger));
  expect(triggers.has('tiktok.gift')).toBe(true);
  expect(triggers.has('tiktok.follow')).toBe(true);
  expect(triggers.has('tiktok.share')).toBe(true);
});

test('every gift-wall rule runs plugin actions with a title + subtitle', () => {
  const parsed = parseRuleProfile(presetJson);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  for (const entry of parsed.profile.templates) {
    const template = entry.template;
    expect(template.actions.length).toBeGreaterThan(0);
    for (const action of template.actions) {
      expect(action.typeId.startsWith('minecraft.server.')).toBe(true);
    }
    const commands = template.actions
      .map((action) => action.config.command)
      .filter((command): command is string => typeof command === 'string');
    for (const needle of ['/title @a times ', '/title @a subtitle ', '/title @a title ']) {
      expect(commands.some((command) => command.includes(needle))).toBe(true);
    }
  }
});

test('gift-wall preset instantiates one event and its actions per entry', () => {
  const parsed = parseRuleProfile(presetJson);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const applied = instantiateProfile(parsed.profile);
  expect(applied).toHaveLength(16);
  for (const rule of applied) {
    expect(rule.actions.length).toBeGreaterThan(0);
    expect(rule.event.actionIds).toHaveLength(rule.actions.length);
  }
});
