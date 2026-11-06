import { expect, test } from 'bun:test';

import manifestJson from '../../../tests/fixtures/plugins/minecraft-server.plugin.json';
import type { ActionTypeDefinition } from '../behavior/types.ts';
import { defaultActionConfig } from '../behavior/action-config.ts';

type MinecraftManifest = {
  id: string;
  runtime: string;
  schemaVersion: number;
  http: { baseUrl: string };
  actionTypes: ActionTypeDefinition[];
  settings: { schema: { properties: { serverUrl: { default: string } } } };
};

const manifest = manifestJson as unknown as MinecraftManifest;

/** Renders one declarative body template the way the host does (single pass, unknown paths empty). */
function renderBody(template: string, config: Record<string, unknown>): string {
  return template.replace(/\{\{\s*config\.([A-Za-z0-9_]+)\s*\}\}/g, (_, key: string) => {
    const value = config[key];
    return value === undefined || value === null ? '' : String(value);
  });
}

test('minecraft example exposes six structured actions', () => {
  expect(manifest.schemaVersion).toBe(3);
  expect(manifest.id).toBe('minecraft.server');
  expect(manifest.runtime).toBe('declarative');
  expect(manifest.http.baseUrl).toBe('{{ settings.serverUrl }}');
  expect(manifest.settings.schema.properties.serverUrl.default).toBe('http://127.0.0.1:8080');
  expect(manifest.actionTypes.map((action) => action.id)).toEqual([
    'minecraft.server.give-item',
    'minecraft.server.spawn-mob',
    'minecraft.server.apply-effect',
    'minecraft.server.change-weather',
    'minecraft.server.send-message',
    'minecraft.server.run-command',
  ]);
  for (const action of manifest.actionTypes) {
    expect(action.icon).toBeTruthy();
    expect(typeof action.category).toBe('string');
    expect(action.requiredCapabilities).toEqual(['http.request']);
  }
});

test('minecraft action defaults match the form contracts', () => {
  const byId = Object.fromEntries(manifest.actionTypes.map((action) => [action.id, action]));
  expect(defaultActionConfig(byId['minecraft.server.give-item']!)).toEqual({
    item: 'minecraft:apple',
    amount: 1,
    target: '@p',
  });
  expect(defaultActionConfig(byId['minecraft.server.spawn-mob']!)).toEqual({
    entity: 'minecraft:zombie',
    target: '@p',
  });
  expect(defaultActionConfig(byId['minecraft.server.apply-effect']!)).toEqual({
    effect: 'minecraft:speed',
    duration: 30,
    amplifier: 0,
    target: '@p',
  });
  expect(defaultActionConfig(byId['minecraft.server.change-weather']!)).toEqual({
    weather: 'clear',
    duration: 300,
  });
  expect(defaultActionConfig(byId['minecraft.server.send-message']!)).toEqual({
    message: 'Hello from TikTok!',
  });
  expect(defaultActionConfig(byId['minecraft.server.run-command']!)).toEqual({
    command: '/say Hello from TikTok!',
  });
});

test('minecraft commands render from structured fields', () => {
  const bodies: Record<string, string> = {
    'minecraft.server.give-item': '/give @p minecraft:apple 1',
    'minecraft.server.spawn-mob': '/execute at @p run summon minecraft:zombie ~ ~ ~',
    'minecraft.server.apply-effect': '/effect give @p minecraft:speed 30 0',
    'minecraft.server.change-weather': '/weather clear 300',
    'minecraft.server.send-message': '/say Hello from TikTok!',
    'minecraft.server.run-command': '/say Hello from TikTok!',
  };
  for (const action of manifest.actionTypes) {
    const http = (action as unknown as { http: { method: string; path: string; body: string } }).http;
    expect(http.method).toBe('POST');
    expect(http.path).toBe('/api/chat');
    const rendered = renderBody(http.body, defaultActionConfig(action) as Record<string, unknown>);
    const payload = JSON.parse(rendered) as { text: string };
    const expected = bodies[action.id];
    if (expected === undefined) throw new Error(`no command expectation for ${action.id}`);
    expect(payload.text).toBe(expected);
  }
});
