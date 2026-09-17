import { expect, test } from 'bun:test';

import type { JsonObject, NodeDefinition } from '../../../automation/types.ts';
import type { PluginTemplateDescriptor } from '../../../automation/behavior/types.ts';
import { normalizeWorkflowGraph } from './graph.ts';
import {
  pluginTemplateToWorkflowTemplate,
  resolveTemplateParams,
  substituteTemplateParams,
  templateParamDefaults,
} from './plugin-templates.ts';
import { workflowTemplateAvailable } from './workflow-templates.ts';

function flowPort(name: string, title: string) {
  return { name, title, kind: 'flow' as const };
}

function definition(type: string): NodeDefinition {
  return {
    type,
    version: 1,
    pluginId: 'core',
    title: type,
    category: 'Test',
    kind: type === 'trigger.event' ? 'trigger' : 'action',
    inputs: type === 'trigger.event' ? [] : [flowPort('in', 'In')],
    outputs: [flowPort('out', 'Out')],
    configSchema: { type: 'object', properties: {} },
  };
}

function descriptor(overrides: Partial<PluginTemplateDescriptor> = {}): PluginTemplateDescriptor {
  return {
    id: 'sonicboom.server/chat-tts',
    pluginId: 'sonicboom.server',
    title: { default: 'Chat to TTS', i18key: 'x' },
    description: { default: 'Speak chat', i18key: 'x' },
    icon: 'voice',
    eventType: 'tiktok.chat',
    requiredNodeTypes: ['trigger.event', 'action.http'],
    category: 'Voice',
    params: {
      type: 'object',
      properties: {
        serverUrl: { type: 'string', default: 'http://localhost:3000' },
        voice: { type: 'string', default: 'M1' },
      },
    },
    workflow: {
      nodes: [
        { type: 'trigger.event', config: { eventType: 'tiktok.chat' } },
        {
          type: 'action.http',
          config: {
            method: 'POST',
            url: '{{ params.serverUrl }}/api/tts/play?voice={{ params.voice }}',
            body: '{{ event.data.comment }}',
          },
        },
      ],
    },
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
    ...overrides,
  };
}

test('substitutes params but keeps event spans for runtime', () => {
  const params = { serverUrl: 'http://x', voice: 'F2' };
  expect(
    substituteTemplateParams('{{ params.serverUrl }}/p?voice={{params.voice}}&t={{ event.data.comment }}', params),
  ).toBe('http://x/p?voice=F2&t={{ event.data.comment }}');
  expect(substituteTemplateParams({ a: ['{{ params.missing }}', 1, true] }, {})).toEqual({ a: ['', 1, true] });
});

test('resolves user options over schema defaults', () => {
  const entry = descriptor();
  expect(templateParamDefaults(entry.params)).toEqual({ serverUrl: 'http://localhost:3000', voice: 'M1' });
  expect(resolveTemplateParams(entry, { name: 'Chat', voice: 'F2' })).toEqual({
    serverUrl: 'http://localhost:3000',
    voice: 'F2',
  });
  expect(templateParamDefaults(undefined)).toEqual({});
});

test('converts plugin templates to creatable workflows', () => {
  const template = pluginTemplateToWorkflowTemplate(descriptor());
  expect(template?.id).toBe('sonicboom.server/chat-tts');
  expect(template?.icon).toBe('voice');
  const definitions = [definition('trigger.event'), definition('action.http')];
  expect(workflowTemplateAvailable(template!, definitions)).toBe(true);
  const graph = normalizeWorkflowGraph(
    template!.build({ definitions }, { name: 'Chat', serverUrl: 'http://s:3000', voice: 'F2' }),
    definitions,
  );
  expect(graph.nodes).toHaveLength(2);
  expect(graph.edges).toHaveLength(1);
  const action = graph.nodes[1]!.config as JsonObject;
  expect(action.url).toBe('http://s:3000/api/tts/play?voice=F2');
  expect(action.body).toBe('{{ event.data.comment }}');
});

test('rejects descriptors the generic builder cannot instantiate', () => {
  expect(pluginTemplateToWorkflowTemplate(descriptor({ eventType: 'custom.event' }))).toBeUndefined();
  expect(
    pluginTemplateToWorkflowTemplate(
      descriptor({ workflow: { nodes: [{ type: 'action.http' }] } }),
    ),
  ).toBeUndefined();
  const fallback = pluginTemplateToWorkflowTemplate(descriptor({ icon: 'not-an-icon' }));
  expect(fallback?.icon).toBe('template');
});
