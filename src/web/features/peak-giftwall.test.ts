import { expect, test } from 'vitest';
import profile from '../../../tests/fixtures/profiles/peak-giftwall.tikprofile.json';
import manifest from '../../../tests/fixtures/plugins/peak-game.plugin.json';
import { instantiateProfile, parseRuleProfile } from '@/entities/rule-template/model/rule-profiles-state.ts';

test('PEAK giftwall imports safely and exposes editable live catalog actions', () => {
  const parsed = parseRuleProfile(profile);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const rules = instantiateProfile(parsed.profile);
  expect(rules).toHaveLength(12);
  for (const entry of parsed.profile.templates) {
    expect(entry.template.event.enabled).toBe(true);
    expect(entry.template.event.cooldownMs).toBeGreaterThanOrEqual(5000);
    for (const action of entry.template.actions) {
      const descriptor = manifest.actionTypes.find((a) => a.id === action.typeId);
      expect(descriptor).toBeDefined();
      expect(Number(action.config.quantity)).toBeLessThanOrEqual(5);
      expect(Number(action.config.quantity)).toBeGreaterThanOrEqual(1);
    }
  }
});

test('every PEAK giftwall rule names catalog content, not a blank id', () => {
  const parsed = parseRuleProfile(profile);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  for (const entry of parsed.profile.templates) {
    for (const action of entry.template.actions) {
      const config = action.config;
      const id =
        typeof config.itemId === 'string'
          ? config.itemId
          : typeof config.entityId === 'string'
            ? config.entityId
            : typeof config.hazardId === 'string'
              ? config.hazardId
              : '';
      // A blank id would be a rule that can only fail: the catalog ids come
      // from the running game and are filled in on purpose.
      expect(id.length).toBeGreaterThan(0);
      expect(action.config.positionMode).toBe('random_player');
    }
  }
});

test('entities and hazards of the giftwall carry a bounded lifetime', () => {
  const parsed = parseRuleProfile(profile);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  let entities = 0;
  let hazards = 0;
  for (const entry of parsed.profile.templates) {
    for (const action of entry.template.actions) {
      if (action.typeId === 'peak.game.spawn-entity') entities += 1;
      if (action.typeId === 'peak.game.spawn-hazard') hazards += 1;
      if (action.typeId !== 'peak.game.spawn-entity' && action.typeId !== 'peak.game.spawn-hazard') {
        continue;
      }
      const lifetime = Number(action.config.lifetimeSeconds);
      expect(lifetime).toBeGreaterThanOrEqual(1);
      expect(lifetime).toBeLessThanOrEqual(300);
    }
  }
  expect(entities).toBeGreaterThan(0);
  expect(hazards).toBeGreaterThan(0);
});

test('all PEAK actions opt into authenticated idempotent JSON requests', () => {
  expect(manifest.actionTypes).toHaveLength(5);
  expect(manifest.http.auth.tokenSetting).toBe('apiToken');
  expect(manifest.settings.schema.properties.apiToken.secret).toBe(true);
  for (const action of manifest.actionTypes) {
    expect(action.http.method).toBe('POST');
    expect(action.http.idempotency.enabled).toBe(true);
    expect(action.http.idempotency.header).toBe('Idempotency-Key');
    expect(action.http.headers['Content-Type']).toBe('application/json');
    expect(action.requiredCapabilities).toEqual(['http.request']);
  }
});

test('every PEAK action sends a validated position', () => {
  for (const action of manifest.actionTypes) {
    if (!action.http.path.startsWith('/api/spawn/')) continue;
    const variants = action.http.bodyVariants;
    expect(variants).toBeDefined();
    if (!variants) continue;
    expect(variants.field).toBe('positionMode');
    const values = variants.values;
    expect(Object.keys(values)).toContain('near_player');
    expect(Object.keys(values)).toContain('random_player');
    expect(Object.keys(values)).toContain('world_position');
    for (const variant of Object.values(values)) {
      expect(variant.position.mode).toBeDefined();
    }
  }
});

test('PEAK option sources read the endpoints PEAKapi actually serves', () => {
  const paths = new Set<string>();
  for (const action of manifest.actionTypes) {
    for (const source of Object.values(action.optionSources ?? {})) {
      paths.add(source.path);
    }
  }
  expect(paths.has('/api/catalog/items')).toBe(true);
  expect(paths.has('/api/catalog/entities')).toBe(true);
  expect(paths.has('/api/catalog/hazards')).toBe(true);
  expect(paths.has('/api/players')).toBe(true);
  expect(paths.has('/api/entities')).toBe(true);
});

test('spawned entities and hazards carry a bounded lifetime that survives a missing field', () => {
  const lifetime = manifest.settings.schema.properties.lifetimeSeconds;
  expect(lifetime.default).toBe(60);
  expect(lifetime.minimum).toBe(1);
  expect(lifetime.maximum).toBe(300);
  for (const id of ['peak.game.spawn-entity', 'peak.game.spawn-hazard']) {
    const action = manifest.actionTypes.find((a) => a.id === id);
    const binding = action?.http.bodyJson?.lifetimeSeconds;
    expect(binding).toBeDefined();
    // A profile-imported action carries no field value, so the binding has
    // to fall back to the setting instead of failing the request.
    expect(binding?.$number).toBe('config.lifetimeSeconds');
    expect(binding?.fallback).toBe('settings.lifetimeSeconds');
    expect(binding?.min).toBe(1);
    expect(binding?.max).toBe(300);
    expect(binding?.integer).toBe(true);
  }
});

test('PEAK discovers the BepInEx configuration of the game', () => {
  expect(manifest.http.tokenImport.format).toBe('ini');
  expect(manifest.http.tokenImport.fileName).toBe('org.peak.rest-api.cfg');
  expect(manifest.http.tokenImport.section).toBe('HTTP');
  expect(manifest.http.tokenImport.key).toBe('AuthenticationToken');
  expect(manifest.http.tokenImport.discovery.steamFolder).toBe('PEAK');
  expect(manifest.http.tokenImport.discovery.modManagerGame).toBe('PEAK');
  expect(manifest.http.tokenImport.discovery.relativePath).toBe(
    'BepInEx/config/org.peak.rest-api.cfg',
  );
  expect(manifest.http.tokenImport.connection.setting).toBe('serverUrl');
  expect(manifest.http.tokenImport.connection.hostKey).toBe('Host');
  expect(manifest.http.tokenImport.connection.portKey).toBe('Port');
  expect(manifest.http.health.path).toBe('/health');
  expect(manifest.http.health.readinessField).toBe('gameplayReady');
});
