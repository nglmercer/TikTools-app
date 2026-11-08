import type { JsonObject, JsonValue } from '../../../../automation/types.ts';
import { createActionId, createEventId } from '../../../../automation/behavior/schema.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { AppliedRuleTemplate, RuleTemplate } from './rule-templates.ts';

export const TEMPLATE_RECORD_BYTES = 256 * 1024;
export const TEMPLATE_AGGREGATE_BYTES = 8 * 1024 * 1024;
const MAX_DEPTH = 64;
const MAX_NODES = 65_536;
const substitute = Symbol('template-substitution');
const record = Symbol('template-record');
const solePattern = /^\{\{\s*params\.([a-zA-Z0-9_.]+)\s*\}\}$/;
const spans = () => /\{\{\s*params\.([a-zA-Z0-9_.]+)\s*\}\}/g;
interface Substitution { [substitute]: true; value: unknown; params: JsonObject }
interface RecordPlan { [record]: true; value: unknown }
interface Counter { bytes: number; nodes: number }
interface Context { aggregate: { bytes: number }; record?: Counter }
const tooLarge = (): never => { throw new ControlCallError('too_large', 'Expanded rules exceed the supported size or structure. Reduce repeated parameter values and try again.'); };
const invalid = (): never => { throw new ControlCallError('invalid_params', 'Template parameters must contain bounded JSON data.'); };
const marked = (value: unknown, key: symbol): value is Record<symbol, unknown> => !!value && typeof value === 'object' && Object.hasOwn(value, key);
const expanding = (value: unknown, params: JsonObject): Substitution => ({ [substitute]: true, value, params });
const recorded = (value: unknown): RecordPlan => ({ [record]: true, value });

function lookup(params: JsonObject, path: string): unknown {
  let value: unknown = params;
  for (const part of path.split('.')) {
    if (!value || typeof value !== 'object' || Array.isArray(value) || !Object.hasOwn(value, part)) return undefined;
    value = (value as Record<string, unknown>)[part];
  }
  return value;
}
function charge(context: Context, bytes: number): void {
  if (bytes > TEMPLATE_AGGREGATE_BYTES - context.aggregate.bytes || context.record && bytes > TEMPLATE_RECORD_BYTES - context.record.bytes) tooLarge();
  context.aggregate.bytes += bytes;
  if (context.record) context.record.bytes += bytes;
}
function node(context: Context, depth: number): void {
  if (depth > MAX_DEPTH || context.record && ++context.record.nodes > MAX_NODES) tooLarge();
}
function numberText(value: number): string {
  if (!Number.isFinite(value) || Number.isInteger(value) && !Number.isSafeInteger(value)) throw new ControlCallError('unsafe_source', 'Some parameter numbers cannot be represented exactly.');
  return String(value);
}
/** Count JSON string bytes directly, including surrogate/escape rules. Level1
 * measures JSON text embedded inside an outer JSON string, before serializing. */
function stringContent(context: Context, value: string, start = 0, end = value.length, level = 0): void {
  let bytes = 0;
  const remaining = Math.min(TEMPLATE_AGGREGATE_BYTES - context.aggregate.bytes, context.record ? TEMPLATE_RECORD_BYTES - context.record.bytes : Infinity);
  for (let i = start; i < end; ++i) {
    const unit = value.charCodeAt(i);
    if (unit === 34 || unit === 92) bytes += level ? 4 : 2;
    else if (unit < 32) bytes += [8, 9, 10, 12, 13].includes(unit) ? level ? 3 : 2 : level ? 7 : 6;
    else if (unit >= 0xd800 && unit <= 0xdbff && i + 1 < end && value.charCodeAt(i + 1) >= 0xdc00 && value.charCodeAt(i + 1) <= 0xdfff) { bytes += 4; ++i; }
    else if (unit >= 0xd800 && unit <= 0xdfff) bytes += level ? 7 : 6;
    else bytes += unit < 128 ? 1 : unit < 2048 ? 2 : 3;
    if (bytes > remaining) tooLarge();
  }
  charge(context, bytes);
}
function stringValue(context: Context, value: string, level = 0): void {
  charge(context, level ? 4 : 2); stringContent(context, value, 0, value.length, level);
}
/** Literal params inserted inline are JSON text. Their own traversal is bounded
 * too, even though the resulting record contains only one string node. */
function serializedText(context: Context, value: unknown, depth = 0, counter = { nodes: 0 }): void {
  if (depth > MAX_DEPTH || ++counter.nodes > MAX_NODES) tooLarge();
  if (value === null || value === undefined) { charge(context, 4); return; }
  if (typeof value === 'string') { stringValue(context, value, 1); return; }
  if (typeof value === 'number') { charge(context, numberText(value).length); return; }
  if (typeof value === 'boolean') { charge(context, value ? 4 : 5); return; }
  if (Array.isArray(value)) {
    charge(context, 2); for (let i = 0; i < value.length; ++i) { if (i) charge(context, 1); serializedText(context, value[i], depth + 1, counter); } return;
  }
  if (typeof value !== 'object') invalid();
  charge(context, 2); let count = 0;
  for (const key of Object.keys(value)) {
    const child = (value as Record<string, unknown>)[key]; if (child === undefined) continue;
    if (count++) charge(context, 1); stringValue(context, key, 1); charge(context, 1); serializedText(context, child, depth + 1, counter);
  }
}
function inlineText(context: Context, value: unknown): void {
  if (value === null || value === undefined) return;
  if (typeof value === 'string') stringContent(context, value);
  else if (typeof value === 'number') charge(context, numberText(value).length);
  else if (typeof value === 'boolean') charge(context, value ? 4 : 5);
  else serializedText(context, value);
}
function measure(value: unknown, context: Context, depth = 0, params?: JsonObject): void {
  if (marked(value, record)) { measure((value as unknown as RecordPlan).value, { aggregate: context.aggregate, record: { bytes: 0, nodes: 0 } }); return; }
  if (marked(value, substitute)) { const plan = value as unknown as Substitution; measure(plan.value, context, depth, plan.params); return; }
  if (typeof value === 'string' && params) {
    const sole = solePattern.exec(value);
    if (sole?.[1]) { const raw = lookup(params, sole[1]); measure(raw === undefined ? '' : raw, context, depth); return; }
    node(context, depth); charge(context, 2);
    const pattern = spans(); let cursor = 0; let match: RegExpExecArray | null;
    while ((match = pattern.exec(value))) {
      stringContent(context, value, cursor, match.index); inlineText(context, lookup(params, match[1]!)); cursor = pattern.lastIndex;
    }
    stringContent(context, value, cursor); return;
  }
  node(context, depth);
  if (value === null || value === undefined) { charge(context, 4); return; }
  if (typeof value === 'string') { stringValue(context, value); return; }
  if (typeof value === 'number') { charge(context, numberText(value).length); return; }
  if (typeof value === 'boolean') { charge(context, value ? 4 : 5); return; }
  if (Array.isArray(value)) {
    charge(context, 2); for (let i = 0; i < value.length; ++i) { if (i) charge(context, 1); measure(value[i], context, depth + 1, params); } return;
  }
  if (typeof value !== 'object') invalid();
  charge(context, 2); let count = 0;
  for (const key of Object.keys(value)) {
    const child = (value as Record<string, unknown>)[key]; if (child === undefined) continue;
    if (count++) charge(context, 1); stringValue(context, key); charge(context, 1); measure(child, context, depth + 1, params);
  }
}

// Only called after the complete batch has passed measurement. Serialize plain
// JSON ourselves so object toJSON hooks cannot bypass the checked source tree.
function plainText(value: unknown): string {
  const pieces: string[] = [];
  function write(node: unknown): void {
    if (node === null || node === undefined) { pieces.push('null'); return; }
    if (typeof node === 'string') { pieces.push(JSON.stringify(node)); return; }
    if (typeof node === 'number') { pieces.push(numberText(node)); return; }
    if (typeof node === 'boolean') { pieces.push(node ? 'true' : 'false'); return; }
    if (Array.isArray(node)) { pieces.push('['); for (let i = 0; i < node.length; ++i) { if (i) pieces.push(','); write(node[i]); } pieces.push(']'); return; }
    if (typeof node !== 'object') invalid();
    pieces.push('{'); let count = 0;
    for (const key of Object.keys(node)) {
      const child = (node as Record<string, unknown>)[key]; if (child === undefined) continue;
      if (count++) pieces.push(','); pieces.push(JSON.stringify(key), ':'); write(child);
    }
    pieces.push('}');
  }
  write(value); return pieces.join('');
}
function scalarText(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number') return numberText(value);
  if (typeof value === 'boolean') return String(value);
  return plainText(value);
}
function materialize(value: unknown, params?: JsonObject): unknown {
  if (marked(value, record)) return materialize((value as unknown as RecordPlan).value);
  if (marked(value, substitute)) { const plan = value as unknown as Substitution; return materialize(plan.value, plan.params); }
  if (typeof value === 'string' && params) {
    const sole = solePattern.exec(value);
    if (sole?.[1]) { const raw = lookup(params, sole[1]); return raw === undefined ? '' : materialize(raw); }
    const pieces: string[] = []; const pattern = spans(); let cursor = 0; let match: RegExpExecArray | null;
    while ((match = pattern.exec(value))) { pieces.push(value.slice(cursor, match.index), scalarText(lookup(params, match[1]!))); cursor = pattern.lastIndex; }
    pieces.push(value.slice(cursor)); return pieces.join('');
  }
  if (Array.isArray(value)) return Array.from(value, (entry) => materialize(entry, params));
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).map((key) => [key, materialize((value as Record<string, unknown>)[key], params)]));
  return value;
}
export function boundedSubstitute(value: JsonValue, params: JsonObject): JsonValue {
  const plan = recorded(expanding(value, params)); measure(plan, { aggregate: { bytes: 0 } }); return materialize(plan) as JsonValue;
}
export interface TemplateExpansionInput { template: RuleTemplate; params: JsonObject; overrides?: { eventName?: string; actionNames?: string[] } }
export function instantiateBoundedTemplates(inputs: TemplateExpansionInput[]): AppliedRuleTemplate[] {
  if (!inputs.length || inputs.length > 32 || inputs.some(({ template }) => !template.actions.length || template.actions.length > 8)) tooLarge();
  // IDs and links are minted once, before measuring; no expanded strings or
  // trees exist until every record and the complete batch pass the same budget.
  const plans = inputs.map(({ template, params, overrides = {} }) => {
    const ids = template.actions.map(() => createActionId());
    const actions = template.actions.map((action, index) => recorded({ schemaVersion: 2, id: ids[index]!,
      name: overrides.actionNames?.[index]?.trim() || expanding(action.name, params), typeId: action.typeId, enabled: action.enabled, config: expanding(action.config, params) }));
    const event = recorded({ schemaVersion: 1, id: createEventId(), name: overrides.eventName?.trim() || expanding(template.event.name, params),
      enabled: template.event.enabled, trigger: template.event.trigger, filters: expanding(template.event.filters, params), cooldownMs: template.event.cooldownMs,
      cooldownScope: template.event.cooldownScope, actionIds: ids, runMode: template.event.runMode });
    return { actions, event };
  });
  measure(plans, { aggregate: { bytes: 0 } });
  return materialize(plans) as AppliedRuleTemplate[];
}
