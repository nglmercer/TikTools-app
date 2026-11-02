export const PLUGIN_PROTOCOL_VERSION = 1;
export const MAX_POLL_EVENTS = 16;

export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };
export type JsonObject = { [key: string]: JsonValue };

export interface ActionDescriptor {
  typeId?: string;
  type?: string;
  config?: JsonObject;
  [key: string]: JsonValue | undefined;
}

export interface DomainEventEnvelope {
  topic: string;
  data: JsonValue;
}

export interface EventEnrichmentRequest {
  processorId: string;
  event: JsonValue;
  settings?: JsonValue;
  inputs?: Record<string, JsonValue>;
}

export type PluginCall =
  | { type: 'action'; action: ActionDescriptor; event?: JsonValue }
  | { type: 'poll' }
  | { type: 'enrich'; request: EventEnrichmentRequest }
  | { type: 'event'; event: DomainEventEnvelope };

export interface PluginContext {
  pluginId?: string;
  name?: string;
  version?: string;
}

export interface MediaFileRef {
  referenceVersion?: number;
  path: string;
  directory?: string;
  name?: string;
}

export type AudioOverlap = 'allow' | 'restart' | 'drop';

export interface AudioPlayIntent {
  fileRef: MediaFileRef;
  volume?: number;
  overlap?: AudioOverlap;
}

export interface EmitIntent {
  eventType: string;
  data: JsonValue;
}

export type HostIntent = { type: 'emit'; data: EmitIntent } | { type: 'audio-play'; data: AudioPlayIntent };

export interface PluginEvent {
  type: string;
  data: JsonValue;
}

export interface PluginCallResult {
  summary?: string;
  logs?: string[];
  intents?: HostIntent[];
  events?: PluginEvent[];
}

export interface TextView {
  text: string;
  language?: string;
  confidence?: number;
  source: string;
  speak?: boolean;
  reason?: string;
}

export interface EventEnrichmentResult {
  annotations?: Record<string, JsonValue>;
  views?: Record<string, TextView>;
  logs?: string[];
}

export type CallResult = PluginCallResult | EventEnrichmentResult | Record<string, never>;

export interface TikToolsPlugin {
  onLoad?(context: PluginContext): void | Promise<void>;
  call(request: PluginCall, context: PluginContext): CallResult | Promise<CallResult>;
  onUnload?(context: PluginContext): unknown;
}

export function definePlugin(plugin: TikToolsPlugin): TikToolsPlugin {
  return plugin;
}

export function actionResult(summary: string, extras: Omit<PluginCallResult, 'summary'> = {}): PluginCallResult {
  return { summary, ...extras };
}

export function emit(eventType: string, data: JsonValue = {}): HostIntent {
  return { type: 'emit', data: { eventType, data } };
}

export function audioPlay(path: string, options: Omit<AudioPlayIntent, 'fileRef'> = {}): HostIntent {
  return { type: 'audio-play', data: { fileRef: { path }, ...options } };
}

export function pollResult(events: PluginEvent[], logs: string[] = []): PluginCallResult {
  if (events.length > MAX_POLL_EVENTS) {
    throw new RangeError(`a poll answer carries at most ${MAX_POLL_EVENTS} events`);
  }
  return logs.length ? { events, logs } : { events };
}

export function actionConfig(request: PluginCall): JsonObject {
  return request.type === 'action' ? (request.action.config ?? {}) : {};
}

export function actionType(request: PluginCall): string | undefined {
  if (request.type !== 'action') return undefined;
  return request.action.typeId ?? request.action.type;
}
