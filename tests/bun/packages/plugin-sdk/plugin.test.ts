import { expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  actionConfig,
  actionResult,
  actionType,
  audioPlay,
  definePlugin,
  emit,
  MAX_POLL_EVENTS,
  pollResult,
  type PluginCall,
} from '../../../../packages/plugin-sdk/src/plugin.ts';

test('action helpers build the host wire format', () => {
  expect(actionResult('done', { logs: ['ok'] })).toEqual({ summary: 'done', logs: ['ok'] });
  expect(emit('hotkey.pressed', { key: 'k' })).toEqual({
    type: 'emit',
    data: { eventType: 'hotkey.pressed', data: { key: 'k' } },
  });
  expect(audioPlay('/sounds/a.wav', { volume: 0.5, overlap: 'restart' })).toEqual({
    type: 'audio-play',
    data: { fileRef: { path: '/sounds/a.wav' }, volume: 0.5, overlap: 'restart' },
  });
});

test('poll results stay within the host limit', () => {
  expect(pollResult([{ type: 'timer.tick', data: {} }])).toEqual({ events: [{ type: 'timer.tick', data: {} }] });
  const tooMany = Array.from({ length: MAX_POLL_EVENTS + 1 }, () => ({ type: 'timer.tick', data: {} }));
  expect(() => pollResult(tooMany)).toThrow(RangeError);
});

test('definePlugin keeps the guest object and calls dispatch by type', async () => {
  const plugin = definePlugin({
    call(request) {
      if (request.type !== 'action') return {};
      return actionResult(String(actionConfig(request).message ?? ''));
    },
  });
  const call: PluginCall = { type: 'action', action: { typeId: 'acme.say', config: { message: 'hi' } } };
  expect(actionType(call)).toBe('acme.say');
  expect(await plugin.call(call, {})).toEqual({ summary: 'hi' });
  expect(await plugin.call({ type: 'poll' }, {})).toEqual({});
  expect(actionConfig({ type: 'poll' })).toEqual({});
});

const PARSER_KEYS = [
  'schemaVersion',
  'id',
  'name',
  'version',
  'description',
  'longDescription',
  'icon',
  'tags',
  'runtime',
  'entry',
  'trust',
  'capabilities',
  'capabilityProviders',
  'permissions',
  'protocolVersion',
  'abiVersion',
  'targets',
  'actionTypes',
  'eventTypes',
  'eventSubscriptions',
  'processorTypes',
  'autocomplete',
  'settings',
  'http',
  'templates',
  'pages',
  'ui',
  'nativeAddons',
  'nativeLibs',
  'publisher',
  'authors',
  'license',
  'homepage',
  'repository',
  'engines',
  'dependencies',
  'categories',
  'changelog',
];

type ManifestSchema = {
  properties: Record<string, unknown>;
  required: string[];
};

test('the manifest schema covers every key the host parser reads', () => {
  const schemaPath = join(import.meta.dir, '..', '..', '..', '..', 'packages', 'plugin-sdk', 'plugin.schema.json');
  const schema: ManifestSchema = JSON.parse(readFileSync(schemaPath, 'utf8'));
  for (const key of PARSER_KEYS) expect(Object.keys(schema.properties)).toContain(key);
  expect(schema.required).toEqual(['schemaVersion', 'id', 'name', 'version', 'runtime']);
});

test('repository manifests only use keys the schema knows', () => {
  const schemaPath = join(import.meta.dir, '..', '..', '..', '..', 'packages', 'plugin-sdk', 'plugin.schema.json');
  const schema: Pick<ManifestSchema, 'properties'> = JSON.parse(readFileSync(schemaPath, 'utf8'));
  const fixtures = join(import.meta.dir, '..', '..', '..', '..', 'tests', 'fixtures', 'plugins');
  const manifests = readdirSync(fixtures).filter((name) => name.endsWith('.plugin.json'));
  expect(manifests.length).toBeGreaterThan(0);
  for (const manifest of manifests) {
    const parsed: Record<string, unknown> = JSON.parse(readFileSync(join(fixtures, manifest), 'utf8'));
    for (const key of Object.keys(parsed)) {
      if (key === '$schema' || key === 'apiVersion') continue;
      expect(Object.keys(schema.properties)).toContain(key);
    }
  }
});
