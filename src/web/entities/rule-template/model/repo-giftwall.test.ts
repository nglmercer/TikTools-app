import { expect, test } from 'vitest';
import profile from '../../../../../tests/fixtures/profiles/repo-giftwall.tikprofile.json';
import manifest from '../../../../../tests/fixtures/plugins/repo-game.plugin.json';
import { instantiateProfile, parseRuleProfile } from './rule-profiles-state.ts';

test('REPO giftwall imports safely and exposes editable live catalog actions', () => {
  const parsed = parseRuleProfile(profile);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const rules = instantiateProfile(parsed.profile);
  expect(rules).toHaveLength(8);
  for (const entry of parsed.profile.templates) {
    expect(entry.template.event.enabled).toBe(false);
    expect(entry.template.event.cooldownMs).toBeGreaterThanOrEqual(10000);
    for (const action of entry.template.actions) {
      const descriptor = manifest.actionTypes.find((a) => a.id === action.typeId);
      expect(descriptor).toBeDefined();
      expect(Number(action.config.quantity)).toBeLessThanOrEqual(10);
    }
  }
});

test('all REPO actions opt into authenticated idempotent JSON requests', () => {
  expect(manifest.actionTypes).toHaveLength(8);
  expect(manifest.http.auth.tokenSetting).toBe('apiToken');
  expect(manifest.settings.schema.properties.apiToken.secret).toBe(true);
  for (const action of manifest.actionTypes) {
    expect(action.http.method).toBe('POST');
    expect(action.http.idempotency.enabled).toBe(true);
    expect(action.http.headers['Content-Type']).toBe('application/json');
    expect(action.requiredCapabilities).toEqual(['http.request']);
  }
});
