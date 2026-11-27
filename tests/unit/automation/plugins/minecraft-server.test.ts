import { expect, test } from 'vitest';

import manifestJson from '../../../fixtures/plugins/minecraft-server.plugin.json';
import type { ActionField, ActionFieldKind, ActionTypeDefinition, Localized } from '../../../../src/automation/behavior/types.ts';
import { defaultActionConfig } from '../../../../src/automation/behavior/action-config.ts';
import { propertyOf } from '../../support/json.ts';
import { defined } from '../../../shared/index.ts';

const FIELD_KINDS: readonly ActionFieldKind[] = ['text', 'textarea', 'number', 'range', 'select', 'boolean', 'keyvalue', 'media', 'code'];

const isFieldKind = (value: unknown): value is ActionFieldKind => FIELD_KINDS.some((kind) => kind === value);

const stringAt = (raw: unknown, key: string): string => {
  const value = propertyOf(raw, key);
  if (typeof value !== 'string') throw new Error(`Expected "${key}" to be a string`);
  return value;
};

const localizedAt = (raw: unknown, key: string): Localized => ({ default: stringAt(propertyOf(raw, key), 'default'), i18key: '' });

function toField(raw: unknown): ActionField {
  const kind = propertyOf(raw, 'kind');
  if (!isFieldKind(kind)) throw new Error(`Unknown field kind ${String(kind)}`);
  return { key: stringAt(raw, 'key'), label: localizedAt(raw, 'label'), kind, value: stringAt(raw, 'value') };
}

/** Reads a manifest `actionTypes` entry into the host's action-type contract, checking each field at runtime. */
function toActionType(raw: unknown): ActionTypeDefinition {
  const fields = propertyOf(raw, 'fields');
  const capabilities = propertyOf(raw, 'requiredCapabilities');
  if (!Array.isArray(fields) || !Array.isArray(capabilities)) throw new Error('Expected fields and requiredCapabilities arrays');
  return {
    id: stringAt(raw, 'id'),
    title: localizedAt(raw, 'title'),
    description: localizedAt(raw, 'description'),
    tag: stringAt(raw, 'tag'),
    icon: stringAt(raw, 'icon'),
    category: stringAt(raw, 'category'),
    source: { kind: 'plugin', pluginId: manifestJson.id },
    requiredCapabilities: capabilities.map((capability) => String(capability)),
    fields: fields.map(toField),
  };
}

const manifest = {
  ...manifestJson,
  actionTypes: manifestJson.actionTypes.map(toActionType),
};

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
  expect(defaultActionConfig(defined(byId['minecraft.server.give-item']))).toEqual({
    item: 'minecraft:apple',
    amount: 1,
    target: '@p',
  });
  expect(defaultActionConfig(defined(byId['minecraft.server.spawn-mob']))).toEqual({
    entity: 'minecraft:zombie',
    target: '@p',
  });
  expect(defaultActionConfig(defined(byId['minecraft.server.apply-effect']))).toEqual({
    effect: 'minecraft:speed',
    duration: 30,
    amplifier: 0,
    target: '@p',
  });
  expect(defaultActionConfig(defined(byId['minecraft.server.change-weather']))).toEqual({
    weather: 'clear',
    duration: 300,
  });
  expect(defaultActionConfig(defined(byId['minecraft.server.send-message']))).toEqual({
    message: 'Hello from TikTok!',
  });
  expect(defaultActionConfig(defined(byId['minecraft.server.run-command']))).toEqual({
    command: '/say Hello from TikTok!',
  });
});

/** The raw declarative `http` request of an action type, which the host contract does not model. */
interface DeclarativeHttp {
  method: string;
  path: string;
  body: string;
}

function httpOf(actionId: string): DeclarativeHttp {
  const raw = manifestJson.actionTypes.find((action) => action.id === actionId);
  if (!raw) throw new Error(`no manifest entry for ${actionId}`);
  return raw.http;
}

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
    const http = httpOf(action.id);
    expect(http.method).toBe('POST');
    expect(http.path).toBe('/api/chat');
    const rendered = renderBody(http.body, defaultActionConfig(action));
    const payload: { text: string } = JSON.parse(rendered);
    const expected = bodies[action.id];
    if (expected === undefined) throw new Error(`no command expectation for ${action.id}`);
    expect(payload.text).toBe(expected);
  }
});
