/**
 * Behavior rule templates: JSON-defined event+actions bundles.
 *
 * A template is data, not code: one trigger event, its actions, and a
 * creation-time params form. `{{ params.* }}` spans resolve when the template
 * is applied; `{{ event.* }}` spans survive for runtime. Built-ins ship below
 * as data; user templates arrive through file/paste import and persist in
 * host app.state. This module never touches the network or the DOM.
 */

import type { JsonObject, JsonValue } from '../../../automation/types.ts';
import { createActionId, createEventId } from '../../../automation/behavior/schema.ts';
import type {
  CooldownScope,
  EventFilter,
  EventRunMode,
  FilterOperator,
  LiveAction,
  LiveEvent,
} from '../../../automation/behavior/types.ts';

/** Template display text: plain default plus an optional catalog key. */
export interface TemplateText {
  default: string;
  i18key?: string;
}

export const RULE_TEMPLATE_VERSION = 1;
export const RULE_TEMPLATE_ID_PATTERN = /^[a-z0-9][a-z0-9._-]{1,63}$/;
export const MAX_RULE_TEMPLATE_BYTES = 65536;
const MAX_ACTIONS = 8;
const MAX_FILTERS = 12;

const OPERATORS: readonly FilterOperator[] = [
  'gte', 'gt', 'lte', 'lt', 'eq', 'neq', 'contains', 'starts-with', 'in', 'is-true', 'is-false',
];

/** `{{ params.* }}` spans substituted at creation; all other spans survive. */
const PARAMS_PATTERN = /\{\{\s*params\.([a-zA-Z0-9_.]+)\s*\}\}/g;
const SOLE_PARAMS_PATTERN = /^\{\{\s*params\.([a-zA-Z0-9_.]+)\s*\}\}$/;

export interface RuleTemplateAction {
  name: string;
  typeId: string;
  enabled: boolean;
  config: JsonObject;
}

export interface RuleTemplateEvent {
  name: string;
  enabled: boolean;
  trigger: string;
  filters: EventFilter[];
  cooldownMs: number;
  cooldownScope: CooldownScope;
  runMode: EventRunMode;
}

export interface RuleTemplate {
  templateVersion: 1;
  id: string;
  title: TemplateText;
  description: TemplateText;
  icon: string;
  /** JSON-schema subset rendered by the modal params form; absent means no params. */
  params?: JsonObject;
  actions: RuleTemplateAction[];
  event: RuleTemplateEvent;
  /** True for user-imported templates (deletable, exportable). Never persisted in the document. */
  custom?: boolean;
}

export type ParseRuleTemplateResult =
  | { ok: true; template: RuleTemplate }
  | { ok: false; errors: string[] };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function readText(value: unknown, max: number): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (trimmed === '') return null;
  return trimmed.slice(0, max);
}

function readLocalized(value: unknown, max: number): TemplateText | null {
  if (typeof value === 'string') {
    const text = readText(value, max);
    return text ? { default: text } : null;
  }
  if (!isRecord(value)) return null;
  const text = readText(value['default'], max);
  if (!text) return null;
  const localized: TemplateText = { default: text };
  if (typeof value['i18key'] === 'string' && value['i18key'].trim() !== '') {
    localized.i18key = value['i18key'].trim().slice(0, 128);
  }
  return localized;
}

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

/**
 * Deep-substitutes `{{ params.* }}` spans. A string holding exactly one span
 * takes the raw value (numbers and booleans survive for typed configs);
 * embedded spans interpolate as text. `{{ event.* }}` and unknown spans pass
 * through for the runtime.
 */
export function substituteRuleParams(value: JsonValue, params: JsonObject): JsonValue {
  if (typeof value === 'string') {
    const sole = SOLE_PARAMS_PATTERN.exec(value);
    if (sole?.[1]) {
      const raw = readPath(params, sole[1]);
      return raw === undefined ? '' : raw;
    }
    PARAMS_PATTERN.lastIndex = 0;
    return value.replace(PARAMS_PATTERN, (_span, path: string) => scalarText(readPath(params, path)));
  }
  if (Array.isArray(value)) return value.map((entry) => substituteRuleParams(entry, params));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as JsonObject).map(([key, entry]) => [key, substituteRuleParams(entry as JsonValue, params)]),
    );
  }
  return value;
}

/** Schema defaults for a template `params` block (`{properties: {key: {default}}}`). */
export function ruleParamDefaults(params: JsonObject | undefined): JsonObject {
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

function parseFilter(value: unknown, index: number, errors: string[]): EventFilter | null {
  if (!isRecord(value)) {
    errors.push(`event.filters[${index}] must be an object`);
    return null;
  }
  const path = readText(value['path'], 200);
  const operator = value['operator'];
  if (!path) errors.push(`event.filters[${index}].path is required`);
  if (typeof operator !== 'string' || !(OPERATORS as readonly string[]).includes(operator)) {
    errors.push(`event.filters[${index}].operator must be one of ${OPERATORS.join(', ')}`);
    return null;
  }
  let values: string[] | undefined;
  if (value['values'] !== undefined) {
    if (!Array.isArray(value['values']) || value['values'].some((entry) => typeof entry !== 'string')) {
      errors.push(`event.filters[${index}].values must be an array of strings`);
      return null;
    }
    values = (value['values'] as string[]).map((entry) => entry.slice(0, 500));
  }
  if (!path) return null;
  return {
    path,
    operator: operator as FilterOperator,
    value: typeof value['value'] === 'string' ? value['value'].slice(0, 500) : '',
    ...(values ? { values } : {}),
  };
}

/**
 * Validates unknown JSON into a normalized template. Never throws:
 * malformed input yields a display-safe error list instead.
 */
export function parseRuleTemplate(value: unknown): ParseRuleTemplateResult {
  const errors: string[] = [];
  if (!isRecord(value)) return { ok: false, errors: ['template must be a JSON object'] };
  if (value['templateVersion'] !== RULE_TEMPLATE_VERSION) {
    return { ok: false, errors: [`unsupported templateVersion (want ${RULE_TEMPLATE_VERSION})`] };
  }
  const id = readText(value['id'], 64);
  if (!id || !RULE_TEMPLATE_ID_PATTERN.test(id)) {
    errors.push('id must match [a-z0-9][a-z0-9._-]{1,63}');
  }
  const title = readLocalized(value['title'], 120);
  if (!title) errors.push('title is required');
  const description = readLocalized(value['description'], 500) ?? { default: '' };
  const icon = readText(value['icon'], 32) ?? 'plugin';

  let params: JsonObject | undefined;
  if (value['params'] !== undefined) {
    if (!isRecord(value['params'])) errors.push('params must be an object');
    else params = value['params'] as JsonObject;
  }

  const actions: RuleTemplateAction[] = [];
  if (!Array.isArray(value['actions'])) {
    errors.push('actions must be an array');
  } else if (value['actions'].length === 0 || value['actions'].length > MAX_ACTIONS) {
    errors.push(`actions must hold 1-${MAX_ACTIONS} entries`);
  } else {
    value['actions'].forEach((entry: unknown, index: number) => {
      if (!isRecord(entry)) {
        errors.push(`actions[${index}] must be an object`);
        return;
      }
      const name = readText(entry['name'], 80);
      const typeId = readText(entry['typeId'], 128);
      if (!name) errors.push(`actions[${index}].name is required`);
      if (!typeId) errors.push(`actions[${index}].typeId is required`);
      if (!isRecord(entry['config'])) errors.push(`actions[${index}].config must be an object`);
      if (name && typeId && isRecord(entry['config'])) {
        actions.push({
          name,
          typeId,
          enabled: entry['enabled'] === undefined ? true : entry['enabled'] === true,
          config: entry['config'] as JsonObject,
        });
      }
    });
  }

  let event: RuleTemplateEvent | null = null;
  if (!isRecord(value['event'])) {
    errors.push('event must be an object');
  } else {
    const raw = value['event'] as Record<string, unknown>;
    const name = readText(raw['name'], 80);
    const trigger = readText(raw['trigger'], 128);
    if (!name) errors.push('event.name is required');
    if (!trigger) errors.push('event.trigger is required');
    const filters: EventFilter[] = [];
    if (raw['filters'] !== undefined) {
      if (!Array.isArray(raw['filters'])) errors.push('event.filters must be an array');
      else if (raw['filters'].length > MAX_FILTERS) errors.push(`event.filters allows at most ${MAX_FILTERS} entries`);
      else {
        raw['filters'].forEach((entry: unknown, index: number) => {
          const filter = parseFilter(entry, index, errors);
          if (filter) filters.push(filter);
        });
      }
    }
    const cooldownMs = typeof raw['cooldownMs'] === 'number' && Number.isFinite(raw['cooldownMs'])
      ? Math.max(0, Math.floor(raw['cooldownMs']))
      : 0;
    const cooldownScope: CooldownScope = raw['cooldownScope'] === 'user' ? 'user' : 'global';
    const runMode: EventRunMode = raw['runMode'] === 'random' ? 'random' : 'all';
    if (name && trigger) {
      event = {
        name,
        enabled: raw['enabled'] === undefined ? true : raw['enabled'] === true,
        trigger,
        filters,
        cooldownMs,
        cooldownScope,
        runMode,
      };
    }
  }

  if (errors.length > 0 || !id || !title || !event || actions.length === 0) {
    return { ok: false, errors };
  }
  return { ok: true, template: { templateVersion: 1, id, title, description, icon, params, actions, event } };
}

/** Accepts one template document, an array, or `{templates: [...]}`. */
export function parseRuleTemplateList(value: unknown): { templates: RuleTemplate[]; errors: string[] } {
  if (isRecord(value) && value['profileVersion'] !== undefined && value['templateVersion'] === undefined) {
    return { templates: [], errors: ['this is a profile document; import it with the template CLI profile-import'] };
  }
  const documents: unknown[] = Array.isArray(value)
    ? value
    : isRecord(value) && Array.isArray(value['templates'])
      ? (value['templates'] as unknown[])
      : [value];
  if (documents.length === 0) return { templates: [], errors: ['no templates found'] };
  if (documents.length > 32) return { templates: [], errors: ['at most 32 templates per import'] };
  const templates: RuleTemplate[] = [];
  const errors: string[] = [];
  documents.forEach((entry, index) => {
    const parsed = parseRuleTemplate(entry);
    if (parsed.ok) templates.push(parsed.template);
    else {
      const label = isRecord(entry) && typeof entry['id'] === 'string' ? ` (${entry['id']})` : `#${index + 1}`;
      for (const error of parsed.errors) errors.push(`template${label}: ${error}`);
    }
  });
  return { templates, errors };
}

export interface RuleTemplateRequirements {
  missingTypeIds: string[];
  unknownTrigger: boolean;
}

/** Availability gates resolved against the live snapshot at apply time. */
export function missingRuleTemplateRequirements(
  template: RuleTemplate,
  availableTypeIds: Set<string>,
  knownTriggers: string[],
): RuleTemplateRequirements {
  return {
    missingTypeIds: [...new Set(template.actions.map((action) => action.typeId))].filter((id) => !availableTypeIds.has(id)),
    unknownTrigger: !knownTriggers.includes(template.event.trigger),
  };
}

export interface AppliedRuleTemplate {
  actions: LiveAction[];
  event: LiveEvent;
}

/** Instantiates records with fresh ids and creation-time params resolved. */
export function applyRuleTemplate(
  template: RuleTemplate,
  params: JsonObject,
  overrides: { eventName?: string; actionNames?: string[] } = {},
): AppliedRuleTemplate {
  const actions: LiveAction[] = template.actions.map((action, index) => ({
    schemaVersion: 2,
    id: createActionId(),
    name: overrides.actionNames?.[index]?.trim() || (substituteRuleParams(action.name, params) as string),
    typeId: action.typeId,
    enabled: action.enabled,
    config: substituteRuleParams(action.config, params) as JsonObject,
  }));
  const eventName = overrides.eventName?.trim() || (substituteRuleParams(template.event.name, params) as string);
  return {
    actions,
    event: {
      schemaVersion: 1,
      id: createEventId(),
      name: eventName,
      enabled: template.event.enabled,
      trigger: template.event.trigger,
      filters: substituteRuleParams(template.event.filters as unknown as JsonObject, params) as unknown as EventFilter[],
      cooldownMs: template.event.cooldownMs,
      cooldownScope: template.event.cooldownScope,
      actionIds: actions.map((action) => action.id),
      runMode: template.event.runMode,
    },
  };
}

/** Portable export document (drops the runtime-only `custom` flag). */
export function exportRuleTemplate(template: RuleTemplate): string {
  const { custom: _dropped, ...document } = template;
  void _dropped;
  return JSON.stringify(document, null, 2);
}

/** Built-in rule templates, shipped as data through the same parser. */
const BUILTIN_DOCUMENTS: unknown[] = [
  {
    templateVersion: 1,
    id: 'gift-webhook',
    title: { default: 'Gift to webhook', i18key: 'behavior.copy.tplGiftWebhookTitle' },
    description: { default: 'POST every gift to a server or webhook.', i18key: 'behavior.copy.tplGiftWebhookDesc' },
    icon: 'webhook',
    params: {
      type: 'object',
      properties: {
        url: { type: 'string', title: 'Webhook URL', default: 'https://' },
      },
      required: ['url'],
    },
    actions: [{
      name: 'Send gift webhook',
      typeId: 'core.fetch',
      config: {
        method: 'POST',
        url: '{{ params.url }}',
        headers: {},
        body: '{\n  "viewer": "{{ event.user.uniqueId }}",\n  "gift": "{{ event.data.giftName }}",\n  "count": "{{ event.data.repeatCount }}"\n}',
        timeoutMs: 5000,
        emitResponseAs: '',
        allowPrivateNetwork: false,
      },
    }],
    event: { name: 'Gift webhook', trigger: 'tiktok.gift' },
  },
  {
    templateVersion: 1,
    id: 'chat-webhook',
    title: { default: 'Chat to webhook', i18key: 'behavior.copy.tplChatWebhookTitle' },
    description: { default: 'POST every chat message to a server or webhook.', i18key: 'behavior.copy.tplChatWebhookDesc' },
    icon: 'webhook',
    params: {
      type: 'object',
      properties: {
        url: { type: 'string', title: 'Webhook URL', default: 'https://' },
      },
      required: ['url'],
    },
    actions: [{
      name: 'Send chat webhook',
      typeId: 'core.fetch',
      config: {
        method: 'POST',
        url: '{{ params.url }}',
        headers: {},
        body: '{\n  "viewer": "{{ event.user.uniqueId }}",\n  "comment": "{{ event.data.comment }}"\n}',
        timeoutMs: 5000,
        emitResponseAs: '',
        allowPrivateNetwork: false,
      },
    }],
    event: { name: 'Chat webhook', trigger: 'tiktok.chat' },
  },
  {
    templateVersion: 1,
    id: 'gift-sound',
    title: { default: 'Gift sound alert', i18key: 'behavior.copy.tplGiftSoundTitle' },
    description: { default: 'Play a local sound when a gift arrives.', i18key: 'behavior.copy.tplGiftSoundDesc' },
    icon: 'volume',
    params: {
      type: 'object',
      properties: {
        giftName: { type: 'string', title: 'Gift name', default: 'Rose' },
        file: { type: 'string', title: 'Audio file path', default: '' },
      },
      required: ['giftName', 'file'],
    },
    actions: [{
      name: 'Play gift sound',
      typeId: 'audio.play',
      config: { file: '{{ params.file }}', volume: 1, overlap: 'restart' },
    }],
    event: {
      name: 'Gift sound',
      trigger: 'tiktok.gift',
      filters: [{ path: 'event.data.giftName', operator: 'eq', value: '{{ params.giftName }}' }],
    },
  },
  {
    templateVersion: 1,
    id: 'chat-points',
    title: { default: 'Chat points', i18key: 'behavior.copy.tplChatPointsTitle' },
    description: { default: 'Award points for every chat message.', i18key: 'behavior.copy.tplChatPointsDesc' },
    icon: 'points',
    params: {
      type: 'object',
      properties: {
        delta: { type: 'number', title: 'Points', default: 10 },
      },
      required: ['delta'],
    },
    actions: [{
      name: 'Award chat points',
      typeId: 'core.points',
      config: { uniqueId: '{{ event.user.uniqueId }}', delta: '{{ params.delta }}' },
    }],
    event: { name: 'Chat points', trigger: 'tiktok.chat' },
  },
  {
    templateVersion: 1,
    id: 'moderation-subtract',
    title: { default: 'Moderation: subtract points', i18key: 'behavior.copy.tplModerationSubtractTitle' },
    description: { default: 'Deduct points when a chat message trips the repetition filter.', i18key: 'behavior.copy.tplModerationSubtractDesc' },
    icon: 'shield',
    params: {
      type: 'object',
      properties: {
        threshold: { type: 'number', title: 'Repetition threshold', default: 0.7 },
        amount: { type: 'number', title: 'Points to subtract', default: 10 },
      },
      required: ['threshold', 'amount'],
    },
    actions: [{
      name: 'Subtract moderation points',
      typeId: 'core.points.subtract',
      config: { uniqueId: '{{ event.user.uniqueId }}', amount: '{{ params.amount }}' },
    }],
    event: {
      name: 'Moderation penalty',
      trigger: 'tiktok.chat',
      filters: [{ path: 'event.intel.comment.composition.repetitionScore', operator: 'gte', value: '{{ params.threshold }}' }],
    },
  },
];

function loadBuiltins(): RuleTemplate[] {
  const templates: RuleTemplate[] = [];
  for (const document of BUILTIN_DOCUMENTS) {
    const parsed = parseRuleTemplate(document);
    if (!parsed.ok) throw new Error(`builtin rule template invalid: ${parsed.errors.join('; ')}`);
    templates.push(parsed.template);
  }
  return templates;
}

export const BUILTIN_RULE_TEMPLATES: RuleTemplate[] = loadBuiltins();
