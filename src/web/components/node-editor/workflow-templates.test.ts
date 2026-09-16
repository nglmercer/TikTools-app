import { expect, test } from 'bun:test';

import type { JsonObject, NodeDefinition, WorkflowGraph } from '../../../automation/types.ts';
import { normalizeWorkflowGraph } from './graph.ts';
import {
  buildChatTtsConfig,
  buildChatTtsUrl,
  buildTriggeredActionWorkflow,
  CHAT_TTS_DEFAULTS,
  filterWorkflowTemplates,
  isChatTtsLocalPreset,
  isHttpUrl,
  missingTemplateNodes,
  NODE_TYPES,
  redactAuthorization,
  toChatTtsOptions,
  WORKFLOW_TEMPLATES,
  workflowTemplateAvailable,
  workflowTemplateById,
} from './workflow-templates.ts';

function flowPort(name: string, title: string) {
  return { name, title, kind: 'flow' as const };
}

function definition(type: string, kind: NodeDefinition['kind']): NodeDefinition {
  return {
    type,
    version: 1,
    pluginId: 'core',
    title: type,
    category: 'Test',
    kind,
    inputs: kind === 'trigger' ? [] : [flowPort('flow', 'Flow')],
    outputs: [flowPort('flow', 'Flow')],
    configSchema: { type: 'object' },
  };
}

const FULL_CATALOG = [
  definition(NODE_TYPES.triggerEvent, 'trigger'),
  definition(NODE_TYPES.http, 'action'),
  definition(NODE_TYPES.playSound, 'action'),
  definition(NODE_TYPES.adjustPoints, 'action'),
  definition(NODE_TYPES.log, 'action'),
];

function without(type: string): NodeDefinition[] {
  return FULL_CATALOG.filter((entry) => entry.type !== type);
}

function build(id: string, options: JsonObject, definitions: NodeDefinition[] = FULL_CATALOG): WorkflowGraph {
  const template = workflowTemplateById(id);
  if (!template) throw new Error(`missing template ${id}`);
  return template.build({ definitions }, options);
}

test('registry ids are unique and every template names its event', () => {
  const ids = WORKFLOW_TEMPLATES.map((template) => template.id);
  expect(new Set(ids).size).toBe(ids.length);
  expect(ids).toContain('chat-tts');
  for (const template of WORKFLOW_TEMPLATES) {
    expect(template.requiredNodeTypes).toContain(NODE_TYPES.triggerEvent);
    expect(template.title.default.length).toBeGreaterThan(0);
  }
});

test('no template requires a node outside the host catalog', () => {
  const catalogTypes = new Set(FULL_CATALOG.map((entry) => entry.type));
  for (const template of WORKFLOW_TEMPLATES) {
    for (const type of template.requiredNodeTypes) {
      expect(catalogTypes.has(type)).toBe(true);
    }
  }
});

test('templates become unavailable when a required node is missing', () => {
  const chatTts = workflowTemplateById('chat-tts');
  const giftSound = workflowTemplateById('gift-sound');
  if (!chatTts || !giftSound) throw new Error('fixtures missing');
  expect(workflowTemplateAvailable(chatTts, FULL_CATALOG)).toBe(true);
  expect(workflowTemplateAvailable(chatTts, without(NODE_TYPES.http))).toBe(false);
  expect(missingTemplateNodes(chatTts, without(NODE_TYPES.http))).toEqual([NODE_TYPES.http]);
  expect(workflowTemplateAvailable(giftSound, without(NODE_TYPES.playSound))).toBe(false);
  expect(workflowTemplateAvailable(giftSound, without(NODE_TYPES.http))).toBe(true);
});

test('triggered-action builder throws instead of generating invalid graphs', () => {
  expect(() => buildTriggeredActionWorkflow('x', 'tiktok.chat', NODE_TYPES.http, {}, without(NODE_TYPES.http)))
    .toThrow('action.http');
  expect(() => buildTriggeredActionWorkflow('x', 'tiktok.chat', NODE_TYPES.http, {}, without(NODE_TYPES.triggerEvent)))
    .toThrow('trigger.event');
});

test('chat TTS creates a chat trigger wired to an HTTP action', () => {
  const graph = build('chat-tts', { name: 'Chat to TTS', ...CHAT_TTS_DEFAULTS });
  expect(graph.schemaVersion).toBe(1);
  expect(graph.nodes).toHaveLength(2);
  expect(graph.nodes[0]?.type).toBe(NODE_TYPES.triggerEvent);
  expect(graph.nodes[0]?.config.eventType).toBe('tiktok.chat');
  expect(graph.nodes[1]?.type).toBe(NODE_TYPES.http);
  expect(graph.edges).toHaveLength(1);
  const edge = graph.edges[0];
  expect(edge?.source).toBe(graph.nodes[0]?.id);
  expect(edge?.target).toBe(graph.nodes[1]?.id);
  const normalized = normalizeWorkflowGraph(graph, FULL_CATALOG);
  expect(normalized.edges).toHaveLength(1);
});

test('chat TTS posts plain text to the SonicBoom endpoint', () => {
  const config = buildChatTtsConfig({ ...CHAT_TTS_DEFAULTS, voice: 'M1', language: 'en', playNow: false });
  expect(config.method).toBe('POST');
  expect(config.url).toBe('http://localhost:3000/api/tts/play?voice=M1&lang=en');
  expect(config.body).toBe('{{ event.data.comment }}');
  expect(config.bodyMode).toBe('text');
  expect((config.headers as JsonObject)['Content-Type']).toBe('text/plain');
  expect(config.allowPrivateNetwork).toBe(true);
  expect(config.timeoutMs).toBe(10000);
});

test('chat TTS textintel mode uses the intel TTS path', () => {
  const config = buildChatTtsConfig({ ...CHAT_TTS_DEFAULTS, textSource: 'textintel', playNow: true });
  expect(config.body).toBe('{{ event.intel.comment.tts.text }}');
  expect(config.url).toBe('http://localhost:3000/api/tts/play?voice=M1&lang=en&play_now=true');
});

test('chat TTS omits authorization without a token and includes it with one', () => {
  const anonymous = buildChatTtsConfig({ ...CHAT_TTS_DEFAULTS, apiToken: '' });
  expect('Authorization' in ((anonymous.headers as JsonObject) ?? {})).toBe(false);
  const authed = buildChatTtsConfig({ ...CHAT_TTS_DEFAULTS, apiToken: 'secret' });
  expect((authed.headers as JsonObject).Authorization).toBe('Bearer secret');
  expect(redactAuthorization(authed.headers as JsonObject)).toBe('Bearer ••••••••');
  expect(redactAuthorization(anonymous.headers as JsonObject)).toBeUndefined();
});

test('chat TTS enables private network only for the explicit local preset', () => {
  expect(isChatTtsLocalPreset('http://localhost:3000')).toBe(true);
  expect(isChatTtsLocalPreset('http://localhost:3000/')).toBe(true);
  expect(isChatTtsLocalPreset('http://127.0.0.1:3000')).toBe(false);
  expect(isChatTtsLocalPreset('http://192.168.1.10:3000')).toBe(false);
  expect(isChatTtsLocalPreset('https://tts.example.com')).toBe(false);
  const remote = buildChatTtsConfig({ ...CHAT_TTS_DEFAULTS, serverUrl: 'https://tts.example.com' });
  expect(remote.url).toBe('https://tts.example.com/api/tts/play?voice=M1&lang=en');
  expect(remote.allowPrivateNetwork).toBe(false);
});

test('chat TTS url builder rejects non-http servers', () => {
  expect(() => buildChatTtsUrl('notaurl', 'M1', 'en', false)).toThrow();
  expect(isHttpUrl('http://localhost:3000')).toBe(true);
  expect(isHttpUrl('https://example.com/x')).toBe(true);
  expect(isHttpUrl('ftp://example.com')).toBe(false);
  expect(isHttpUrl('')).toBe(false);
});

test('chat TTS options fall back to safe defaults', () => {
  expect(toChatTtsOptions({})).toEqual(CHAT_TTS_DEFAULTS);
  expect(toChatTtsOptions({ textSource: 'other' }).textSource).toBe('raw');
});

test('chat webhook posts templated JSON without private network', () => {
  const graph = build('chat-webhook', { name: 'Chat Webhook', url: 'https://hooks.example.com/live' });
  const config = graph.nodes[1]?.config as JsonObject;
  expect(graph.nodes[0]?.config.eventType).toBe('tiktok.chat');
  expect(config.method).toBe('POST');
  expect(config.bodyMode).toBe('json');
  expect(config.allowPrivateNetwork).toBeUndefined();
  expect(JSON.parse(String(config.body))).toEqual({
    type: '{{ event.type }}',
    user: '{{ event.user.uniqueId }}',
    message: '{{ event.data.comment }}',
  });
  expect(normalizeWorkflowGraph(graph, FULL_CATALOG).edges).toHaveLength(1);
});

test('gift webhook uses gift registry fields', () => {
  const graph = build('gift-webhook', { name: 'Gift Webhook', url: 'https://hooks.example.com/live' });
  const config = graph.nodes[1]?.config as JsonObject;
  expect(graph.nodes[0]?.config.eventType).toBe('tiktok.gift');
  expect(JSON.parse(String(config.body))).toEqual({
    type: '{{ event.type }}',
    user: '{{ event.user.uniqueId }}',
    gift: '{{ event.data.giftName }}',
    count: '{{ event.data.repeatCount }}',
  });
  expect(normalizeWorkflowGraph(graph, FULL_CATALOG).edges).toHaveLength(1);
});

test('chat log writes viewer and comment', () => {
  const graph = build('chat-log', { name: 'Chat Log' });
  expect(graph.nodes[1]?.type).toBe(NODE_TYPES.log);
  expect(graph.nodes[1]?.config.message).toBe('{{ event.user.uniqueId }}: {{ event.data.comment }}');
  expect(normalizeWorkflowGraph(graph, FULL_CATALOG).edges).toHaveLength(1);
});

test('sound templates require a file and target their trigger', () => {
  const gift = build('gift-sound', { name: 'Gift Sound', filePath: '/sounds/ding.mp3' });
  expect(gift.nodes[0]?.config.eventType).toBe('tiktok.gift');
  expect(gift.nodes[1]?.config.filePath).toBe('/sounds/ding.mp3');
  const follow = build('follow-sound', { name: 'Follow Sound', filePath: '/sounds/pop.mp3' });
  expect(follow.nodes[0]?.config.eventType).toBe('tiktok.follow');
  expect(() => build('gift-sound', { name: 'Gift Sound', filePath: '' })).toThrow();
  expect(normalizeWorkflowGraph(gift, FULL_CATALOG).edges).toHaveLength(1);
});

test('chat points adjusts the event viewer by the configured delta', () => {
  const graph = build('chat-points', { name: 'Chat Points', delta: 5 });
  expect(graph.nodes[1]?.type).toBe(NODE_TYPES.adjustPoints);
  expect(graph.nodes[1]?.config.uniqueId).toBe('{{ event.user.uniqueId }}');
  expect(graph.nodes[1]?.config.delta).toBe(5);
  const fallback = build('chat-points', { name: 'Chat Points' });
  expect(fallback.nodes[1]?.config.delta).toBe(1);
  expect(normalizeWorkflowGraph(graph, FULL_CATALOG).edges).toHaveLength(1);
});

test('template search matches title, description, and category', () => {
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, '')).toHaveLength(WORKFLOW_TEMPLATES.length);
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, 'tts').map((t) => t.id)).toEqual(['chat-tts']);
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, 'webhook').map((t) => t.id))
    .toEqual(['chat-webhook', 'gift-webhook']);
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, 'sound').map((t) => t.id))
    .toEqual(['gift-sound', 'follow-sound']);
});
