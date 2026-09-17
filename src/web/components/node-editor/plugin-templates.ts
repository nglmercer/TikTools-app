import type { AutomationEventType, JsonObject, JsonValue, WorkflowGraph } from '../../../automation/types.ts';
import { BUILTIN_EVENT_TYPES } from '../../../automation/contracts/events.ts';
import type { PluginTemplateDescriptor } from '../../../automation/behavior/types.ts';
import { readIconName } from '../icons/icon-registry.ts';
import { appendNodeToGraph, createWorkflowGraph, createWorkflowNode } from './graph.ts';
import {
  NODE_TYPES,
  requiredDefinition,
  type WorkflowTemplate,
  type WorkflowTemplateBuildContext,
} from './workflow-templates.ts';

const TRIGGER_NODE = NODE_TYPES.triggerEvent;

/** `{{ params.* }}` spans substituted at creation time; all other spans survive for runtime. */
const PARAMS_PATTERN = /\{\{\s*params\.([a-zA-Z0-9_.]+)\s*\}\}/g;

function readPath(params: JsonObject, path: string): JsonValue | undefined {
  let current: JsonValue | undefined = params;
  for (const part of path.split('.')) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) return undefined;
    current = (current as JsonObject)[part];
  }
  return current;
}

function scalarText(value: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value) ?? '';
  } catch {
    return '';
  }
}

/** Deep-substitutes `{{ params.* }}` spans; `{{ event.* }}` and unknown spans pass through. */
export function substituteTemplateParams(value: JsonValue, params: JsonObject): JsonValue {
  if (typeof value === 'string') {
    return value.replace(PARAMS_PATTERN, (_, path: string) => scalarText(readPath(params, path)));
  }
  if (Array.isArray(value)) return value.map((entry) => substituteTemplateParams(entry, params));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as JsonObject).map(([key, entry]) => [key, substituteTemplateParams(entry as JsonValue, params)]),
    );
  }
  return value;
}

/** Schema defaults for a template's `params` block (`{properties: {key: {default}}}`). */
export function templateParamDefaults(params: JsonObject | undefined): JsonObject {
  if (!params || typeof params !== 'object' || Array.isArray(params)) return {};
  const properties = params.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return {};
  const defaults: JsonObject = {};
  for (const [key, field] of Object.entries(properties as JsonObject)) {
    if (field && typeof field === 'object' && !Array.isArray(field) && 'default' in field) {
      defaults[key] = (field as JsonObject).default as JsonValue;
    }
  }
  return defaults;
}

/** User options over schema defaults; the workflow `name` is not a param. */
export function resolveTemplateParams(descriptor: PluginTemplateDescriptor, options: JsonObject): JsonObject {
  const params = { ...templateParamDefaults(descriptor.params), ...options };
  delete params.name;
  return params;
}

function isEventType(value: string): value is AutomationEventType {
  return (BUILTIN_EVENT_TYPES as readonly string[]).includes(value);
}

/**
 * Converts a host-stamped plugin template to a selectable workflow template.
 * Rejects descriptors the generic builder cannot instantiate (unknown event
 * type, or a node chain that does not start with the event trigger) so the
 * modal only lists creatable templates.
 */
export function pluginTemplateToWorkflowTemplate(
  descriptor: PluginTemplateDescriptor,
): WorkflowTemplate | undefined {
  if (!isEventType(descriptor.eventType)) return undefined;
  const nodes = descriptor.workflow.nodes;
  const first = nodes[0];
  if (!first || first.type !== TRIGGER_NODE) return undefined;
  const eventType = descriptor.eventType;
  return {
    id: descriptor.id,
    title: descriptor.title,
    description: descriptor.description ?? { default: '', i18key: '' },
    icon: readIconName(descriptor.icon) ?? 'template',
    eventType,
    requiredNodeTypes: [...descriptor.requiredNodeTypes],
    category: descriptor.category,
    build: (context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph => {
      const params = resolveTemplateParams(descriptor, options);
      const name = typeof options.name === 'string' && options.name.trim() ? options.name.trim() : descriptor.title.default;
      const triggerDefinition = requiredDefinition(context.definitions, TRIGGER_NODE);
      let graph = createWorkflowGraph(name, eventType, triggerDefinition);
      const triggerNode = graph.nodes[0];
      if (!triggerNode) throw new Error('Template trigger node is missing.');
      const triggerConfig = substituteTemplateParams(first.config ?? {}, params);
      if (triggerConfig && typeof triggerConfig === 'object' && !Array.isArray(triggerConfig)) {
        triggerNode.config = { ...triggerNode.config, ...(triggerConfig as JsonObject) };
      }
      nodes.slice(1).forEach((entry, position) => {
        const definition = requiredDefinition(context.definitions, entry.type);
        const node = createWorkflowNode(definition, position + 1);
        const rendered = substituteTemplateParams(entry.config ?? {}, params);
        if (rendered && typeof rendered === 'object' && !Array.isArray(rendered)) {
          node.config = { ...node.config, ...(rendered as JsonObject) };
        }
        graph = appendNodeToGraph(graph, node, context.definitions);
      });
      return graph;
    },
  };
}
