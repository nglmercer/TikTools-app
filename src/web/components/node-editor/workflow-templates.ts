import type {
  AutomationEventType,
  JsonObject,
  NodeDefinition,
  WorkflowGraph,
} from '../../../automation/types.ts';
import type { I18nText } from '../../../automation/behavior/types.ts';
import type { IconName } from '../icons/index.ts';
import { appendNodeToGraph, createWorkflowGraph, createWorkflowNode } from './graph.ts';
import { buildHttpHeaders, getHeader } from '../http/http-request.ts';

/** Well-known node types referenced by built-in templates. */
export const NODE_TYPES = {
  triggerEvent: 'trigger.event',
  http: 'action.http',
  playSound: 'action.play-sound',
  adjustPoints: 'action.adjust-points',
  log: 'action.log',
} as const;

export type WorkflowTemplateBuildContext = {
  definitions: NodeDefinition[];
};

export type WorkflowTemplate = {
  id: string;
  title: I18nText;
  description: I18nText;
  icon: IconName;
  eventType: AutomationEventType;
  requiredNodeTypes: string[];
  category?: string;
  build(context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph;
};

export function requiredDefinition(definitions: NodeDefinition[], type: string): NodeDefinition {
  const definition = definitions.find((item) => item.type === type);
  if (!definition) throw new Error(`Required workflow node is unavailable: ${type}`);
  return definition;
}

/** A template is selectable only when every required node exists in the catalog. */
export function workflowTemplateAvailable(template: WorkflowTemplate, definitions: NodeDefinition[]): boolean {
  return missingTemplateNodes(template, definitions).length === 0;
}

export function missingTemplateNodes(template: WorkflowTemplate, definitions: NodeDefinition[]): string[] {
  const available = new Set(definitions.map((definition) => definition.type));
  return template.requiredNodeTypes.filter((type) => !available.has(type));
}

/**
 * Shared builder for `event trigger → single action` templates. Resolves
 * ports through the graph helpers so generated edges always survive
 * `normalizeWorkflowGraph`.
 */
export function buildTriggeredActionWorkflow(
  name: string,
  eventType: AutomationEventType,
  actionType: string,
  actionConfig: JsonObject,
  definitions: NodeDefinition[],
): WorkflowGraph {
  const triggerDefinition = requiredDefinition(definitions, NODE_TYPES.triggerEvent);
  const actionDefinition = requiredDefinition(definitions, actionType);
  let graph = createWorkflowGraph(name, eventType, triggerDefinition);
  const action = createWorkflowNode(actionDefinition, graph.nodes.length);
  action.config = { ...action.config, ...actionConfig };
  graph = appendNodeToGraph(graph, action, definitions);
  return graph;
}

function readName(options: JsonObject, fallback: string): string {
  const name = options.name;
  return typeof name === 'string' && name.trim().length > 0 ? name.trim() : fallback;
}

function readOptionalString(options: JsonObject, key: string): string {
  const value = options[key];
  return typeof value === 'string' ? value.trim() : '';
}

export function isHttpUrl(value: string): boolean {
  try {
    const parsed = new URL(value.trim());
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

function readNumberOption(options: JsonObject, key: string, fallback: number): number {
  const value = options[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

/* ------------------------------------------------------------------ */
/* Webhook templates                                                   */
/* ------------------------------------------------------------------ */

export type WebhookTemplateOptions = {
  url: string;
};

export function toWebhookOptions(options: JsonObject): WebhookTemplateOptions {
  return { url: readOptionalString(options, 'url') || 'https://' };
}

function buildWebhookConfig(url: string, body: string): JsonObject {
  return {
    method: 'POST',
    url,
    headers: buildHttpHeaders(undefined, { 'Content-Type': 'application/json' }),
    body,
    bodyMode: 'json',
    timeoutMs: 10000,
    responseType: 'json',
  };
}

export const CHAT_WEBHOOK_BODY = '{\n  "type": "{{ event.type }}",\n  "user": "{{ event.user.uniqueId }}",\n  "message": "{{ event.data.comment }}"\n}';

function buildChatWebhook(context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph {
  return buildTriggeredActionWorkflow(
    readName(options, 'Chat Webhook'),
    'tiktok.chat',
    NODE_TYPES.http,
    buildWebhookConfig(toWebhookOptions(options).url, CHAT_WEBHOOK_BODY),
    context.definitions,
  );
}

export const GIFT_WEBHOOK_BODY = '{\n  "type": "{{ event.type }}",\n  "user": "{{ event.user.uniqueId }}",\n  "gift": "{{ event.data.giftName }}",\n  "count": "{{ event.data.repeatCount }}"\n}';

function buildGiftWebhook(context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph {
  return buildTriggeredActionWorkflow(
    readName(options, 'Gift Webhook'),
    'tiktok.gift',
    NODE_TYPES.http,
    buildWebhookConfig(toWebhookOptions(options).url, GIFT_WEBHOOK_BODY),
    context.definitions,
  );
}

/* ------------------------------------------------------------------ */
/* Log / sound / points templates                                      */
/* ------------------------------------------------------------------ */

export const CHAT_LOG_MESSAGE = '{{ event.user.uniqueId }}: {{ event.data.comment }}';

function buildChatLog(context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph {
  return buildTriggeredActionWorkflow(
    readName(options, 'Chat Log'),
    'tiktok.chat',
    NODE_TYPES.log,
    { message: CHAT_LOG_MESSAGE },
    context.definitions,
  );
}

function buildSoundWorkflow(
  eventType: AutomationEventType,
  fallbackName: string,
  context: WorkflowTemplateBuildContext,
  options: JsonObject,
): WorkflowGraph {
  const filePath = readOptionalString(options, 'filePath');
  if (!filePath) throw new Error('Select an audio file for the sound template.');
  return buildTriggeredActionWorkflow(
    readName(options, fallbackName),
    eventType,
    NODE_TYPES.playSound,
    { filePath },
    context.definitions,
  );
}

function buildChatPoints(context: WorkflowTemplateBuildContext, options: JsonObject): WorkflowGraph {
  return buildTriggeredActionWorkflow(
    readName(options, 'Chat Points'),
    'tiktok.chat',
    NODE_TYPES.adjustPoints,
    {
      uniqueId: '{{ event.user.uniqueId }}',
      delta: readNumberOption(options, 'delta', 1),
    },
    context.definitions,
  );
}

/* ------------------------------------------------------------------ */
/* Registry                                                            */
/* ------------------------------------------------------------------ */

function text(defaultText: string, i18key: string): I18nText {
  return { default: defaultText, i18key };
}

export const WORKFLOW_TEMPLATES: WorkflowTemplate[] = [
  {
    id: 'chat-webhook',
    title: text('Chat to webhook', 'templateChatWebhook'),
    description: text('Send every chat message to an HTTP endpoint.', 'templateChatWebhookDesc'),
    icon: 'webhook',
    eventType: 'tiktok.chat',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.http],
    category: 'Webhooks',
    build: buildChatWebhook,
  },
  {
    id: 'gift-webhook',
    title: text('Gift to webhook', 'templateGiftWebhook'),
    description: text('Send gift events to an HTTP endpoint.', 'templateGiftWebhookDesc'),
    icon: 'webhook',
    eventType: 'tiktok.gift',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.http],
    category: 'Webhooks',
    build: buildGiftWebhook,
  },
  {
    id: 'chat-log',
    title: text('Chat to log', 'templateChatLog'),
    description: text('Write chat messages to the automation log.', 'templateChatLogDesc'),
    icon: 'code',
    eventType: 'tiktok.chat',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.log],
    category: 'Log',
    build: buildChatLog,
  },
  {
    id: 'gift-sound',
    title: text('Gift sound', 'templateGiftSound'),
    description: text('Play a sound when a gift arrives.', 'templateGiftSoundDesc'),
    icon: 'volume',
    eventType: 'tiktok.gift',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.playSound],
    category: 'Sounds',
    build: (context, options) => buildSoundWorkflow('tiktok.gift', 'Gift Sound', context, options),
  },
  {
    id: 'follow-sound',
    title: text('Follow sound', 'templateFollowSound'),
    description: text('Play a sound for new followers.', 'templateFollowSoundDesc'),
    icon: 'volume',
    eventType: 'tiktok.follow',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.playSound],
    category: 'Sounds',
    build: (context, options) => buildSoundWorkflow('tiktok.follow', 'Follow Sound', context, options),
  },
  {
    id: 'chat-points',
    title: text('Chat points', 'templateChatPoints'),
    description: text('Award points for chat messages.', 'templateChatPointsDesc'),
    icon: 'points',
    eventType: 'tiktok.chat',
    requiredNodeTypes: [NODE_TYPES.triggerEvent, NODE_TYPES.adjustPoints],
    category: 'Points',
    build: buildChatPoints,
  },
];

export function workflowTemplateById(id: string): WorkflowTemplate | undefined {
  return WORKFLOW_TEMPLATES.find((template) => template.id === id);
}

/**
 * Display-only fallback labels for node types. Used only when a template
 * requirement is missing from the catalog, so no host title is available.
 */
export function friendlyNodeType(type: string): string {
  return ({
    'trigger.event': 'Event Trigger',
    'action.http': 'HTTP Request',
    'action.play-sound': 'Play Sound',
    'action.adjust-points': 'Adjust Points',
    'action.log': 'Log',
  } as Record<string, string>)[type] ?? type;
}

/** Case-insensitive search over title, description, and category. */
export function filterWorkflowTemplates(templates: WorkflowTemplate[], query: string): WorkflowTemplate[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return templates;
  return templates.filter((template) => (
    template.title.default.toLowerCase().includes(needle)
    || template.description.default.toLowerCase().includes(needle)
    || (template.category ?? '').toLowerCase().includes(needle)
  ));
}

/**
 * Redacted authorization preview for template summaries (`Bearer ••••••••`
 * when a token is configured, otherwise undefined). Never leaks token text.
 */
export function redactAuthorization(headers: JsonObject): string | undefined {
  return getHeader(headers, 'authorization') === undefined ? undefined : 'Bearer ••••••••';
}
