import { expect, test } from 'bun:test';

import type { JsonObject, NodeDefinition, WorkflowGraph } from '../../../automation/types.ts';
import { normalizeWorkflowGraph } from './graph.ts';
import {
  buildTriggeredActionWorkflow,
  filterWorkflowTemplates,
  isHttpUrl,
  missingTemplateNodes,
  NODE_TYPES,
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
  expect(ids).toContain('chat-webhook');
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
  const chatWebhook = workflowTemplateById('chat-webhook');
  const giftSound = workflowTemplateById('gift-sound');
  if (!chatWebhook || !giftSound) throw new Error('fixtures missing');
  expect(workflowTemplateAvailable(chatWebhook, FULL_CATALOG)).toBe(true);
  expect(workflowTemplateAvailable(chatWebhook, without(NODE_TYPES.http))).toBe(false);
  expect(missingTemplateNodes(chatWebhook, without(NODE_TYPES.http))).toEqual([NODE_TYPES.http]);
  expect(workflowTemplateAvailable(giftSound, without(NODE_TYPES.playSound))).toBe(false);
  expect(workflowTemplateAvailable(giftSound, without(NODE_TYPES.http))).toBe(true);
});

test('http url check accepts http(s) only', () => {
  expect(isHttpUrl('http://localhost:8080')).toBe(true);
  expect(isHttpUrl('https://example.com/x')).toBe(true);
  expect(isHttpUrl('ftp://example.com')).toBe(false);
  expect(isHttpUrl('')).toBe(false);
});

test('triggered-action builder throws instead of generating invalid graphs', () => {
  expect(() => buildTriggeredActionWorkflow('x', 'tiktok.chat', NODE_TYPES.http, {}, without(NODE_TYPES.http)))
    .toThrow('action.http');
  expect(() => buildTriggeredActionWorkflow('x', 'tiktok.chat', NODE_TYPES.http, {}, without(NODE_TYPES.triggerEvent)))
    .toThrow('trigger.event');
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
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, 'webhook').map((t) => t.id))
    .toEqual(['chat-webhook', 'gift-webhook']);
  expect(filterWorkflowTemplates(WORKFLOW_TEMPLATES, 'sound').map((t) => t.id))
    .toEqual(['gift-sound', 'follow-sound']);
});
