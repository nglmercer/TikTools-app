import type { AutomationEventType, JsonObject } from '../types.ts';
// Leaf import only: contracts.ts never imports automation, so this cannot
// cycle (never import the plugin-ui barrel from automation).
import type { PluginUiDescriptor } from '../../plugin-ui/contracts.ts';

/** Canonical localized-text contract lives in shared; re-exported for compatibility. */
import type { Localized, TranslationCatalog } from '../../shared/localized.ts';
export type { I18nText, Localized, TranslationCatalog } from '../../shared/localized.ts';

/** A built-in trigger or a plugin-declared event type (hotkey.pressed). */
export type TriggerType = AutomationEventType | (string & {});

/**
 * Behavior is two records, not one.
 *
 * An ACTION is a configured action type with a name of its own — "Aplausos" —
 * and can be reused by several events. An EVENT is a trigger plus optional
 * filters plus the actions it runs. There is no graph and no nesting: every
 * filter must pass, and an "or" is expressed inside a single filter with the
 * `in` operator.
 */
export type ActionFieldKind =
  | 'text'
  | 'textarea'
  | 'number'
  | 'range'
  | 'select'
  | 'boolean'
  | 'keyvalue'
  | 'media'
  | 'code';

/**
 * A field that only makes sense once another one holds a certain value —
 * a body has nothing to say on a GET. Hidden fields keep their stored value,
 * so switching back does not lose what was typed.
 */
export interface FieldCondition {
  key: string;
  equals?: string[];
  notEquals?: string[];
}

export interface ActionField {
  key: string;
  label: Localized;
  kind: ActionFieldKind;
  /** Default used when the action is created from this type. */
  value: string;
  /** Bounds for `range` fields (slider); ignored by other kinds. */
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
  options?: Array<{ value: string; label: Localized }>;
  /**
   * Host-provided option source for `select` fields
   * (`plugin-action-options:<actionType>:<field>`). Requested on demand via
   * `get-action-options`; static `options` stay as the fallback.
   */
  optionsFrom?: string;
  /** True when `{{ event.* }}` placeholders are rendered before use. */
  template?: boolean;
  /** Kept behind the "advanced options" disclosure so the form stays short. */
  advanced?: boolean;
  hint?: Localized;
  /** Rendered only while this holds. */
  showIf?: FieldCondition;
}

export type ActionSource =
  | { kind: 'builtin' }
  | { kind: 'plugin'; pluginId: string };

export interface ActionTypeDefinition {
  id: string;
  /** Versioned configuration contract shared by the host and plugins. */
  version?: number;
  title: Localized;
  description: Localized;
  /** Short machine-ish label shown on the card: fetch, emit, audio… */
  tag: string;
  source: ActionSource;
  /** Optional field descriptors for small runtimes that do not need JSON Schema. */
  fields?: ActionField[];
  /** JSON Schema subset used by the host-owned configuration renderer. */
  configSchema?: JsonObject;
  /** Host-owned presentation hints; never executable plugin code. */
  uiHints?: JsonObject;
  requiredCapabilities: string[];
}

export interface PluginEventField {
  /** Dotted path, exactly what filters store. */
  path: string;
  kind: 'text' | 'number' | 'boolean';
  label?: Localized;
  hint?: Localized;
  /** Fixed choices render as a dropdown instead of free text. */
  options?: Array<{ value: string; label?: Localized }>;
}

export interface PluginEventType {
  /** Dotted lowercase name, never in a host namespace (hotkey.pressed). */
  type: string;
  title: Localized;
  description?: Localized;
  fields?: PluginEventField[];
  /** Example payload merged into test-event samples. */
  sample?: JsonObject;
  source: ActionSource;
}

export interface LiveAction {
  /** Version 2 is the descriptor/JSON-schema action format; v1 is migrated on read. */
  schemaVersion: 1 | 2;
  id: string;
  name: string;
  typeId: string;
  enabled: boolean;
  /** Field key → value. Key/value fields hold a nested object of strings. */
  config: JsonObject;
}

export type FilterOperator =
  | 'gte'
  | 'gt'
  | 'lte'
  | 'lt'
  | 'eq'
  | 'neq'
  | 'contains'
  | 'starts-with'
  | 'in'
  | 'is-true'
  | 'is-false';

export interface EventFilter {
  /** Dotted path resolved against `{ event, data, user }`. */
  path: string;
  operator: FilterOperator;
  /** Single-value operators. */
  value: string;
  /** `in` only: the "or" lives here instead of in a nested group. */
  values?: string[];
}

export type EventRunMode = 'all' | 'random';
export type CooldownScope = 'global' | 'user';

export interface LiveEvent {
  schemaVersion: 1;
  id: string;
  name: string;
  enabled: boolean;
  trigger: TriggerType;
  /** All of them must pass. Empty means the event always fires. */
  filters: EventFilter[];
  cooldownMs: number;
  cooldownScope: CooldownScope;
  actionIds: string[];
  runMode: EventRunMode;
}

export interface PluginDescriptor {
  id: string;
  source?: 'builtin' | 'user' | 'development';
  name: Localized;
  version: string;
  description: Localized;
  /** Long-form Details copy (markdown-lite); falls back to `description`. */
  longDescription?: Localized;
  /** Host-registry icon name; unknown names fall back to a heuristic icon. */
  icon?: string;
  /** Short discovery tags rendered as chips; derived from actions when absent. */
  tags?: string[];
  /** What it needs from outside the app, in plain words. */
  dependency: Localized;
  permissions: string[];
  actionTypeIds: string[];
  eventTypeIds: string[];
  /** True when the plugin declares a JSON settings schema for the Plugins UI. */
  hasSettings?: boolean;
  /** True when the plugin declares a health endpoint the host can probe. */
  hasConnectionProbe?: boolean;
  /** True when the host can mint this plugin's API token from an admin login. */
  supportsTokenProvisioning?: boolean;
}

export interface PluginStatus {
  descriptor: PluginDescriptor;
  installed: boolean;
  enabled: boolean;
  running?: boolean;
  /** False when the dependency cannot be loaded on this machine. */
  available: boolean;
  unavailableReason?: string;
}

export type RunStatus = 'ok' | 'error';

export interface BehaviorRun {
  id: string;
  at: number;
  status: RunStatus;
  eventId?: string;
  eventName?: string;
  actionId?: string;
  actionName: string;
  summary: string;
  durationMs: number;
  test: boolean;
  logs: string[];
  error?: string;
}

export interface BehaviorSnapshot {
  actions: LiveAction[];
  events: LiveEvent[];
  plugins: PluginStatus[];
  actionTypes: ActionTypeDefinition[];
  /** Plugin-declared event types merged by the host; absent on old hosts. */
  eventTypes?: PluginEventType[];
  /** Plugin-contributed automation templates, stamped by the host; absent on old hosts. */
  pluginTemplates?: PluginTemplateDescriptor[];
  /** Plugin-contributed configuration pages, stamped by the host; absent on old hosts. */
  pluginPages?: PluginPageDescriptor[];
  /** Typed `ui` descriptors, stamped by the host; absent on old hosts. */
  pluginUis?: PluginUiDescriptor[];
  /** Host and loaded plugin translations, keyed by locale and i18key. */
  translations: TranslationCatalog;
}

/** One automation template contributed by a plugin manifest (schema v3). */
export interface PluginTemplateDescriptor {
  /** Namespaced by the host as `<pluginId>/<templateId>`. */
  id: string;
  pluginId: string;
  title: Localized;
  description?: Localized;
  icon?: string;
  eventType: string;
  requiredNodeTypes: string[];
  category?: string;
  /** JSON Schema subset for the creation-time parameters form. */
  params?: JsonObject;
  uiHints?: JsonObject;
  /** Node chain instantiated by the template modal. */
  workflow: {
    nodes: Array<{ type: string; config?: JsonObject }>;
  };
  source: ActionSource;
}

export type PluginPageSectionKind = 'text' | 'form' | 'connection' | 'list' | 'tts';

/** One host-rendered section of a plugin configuration page. */
export interface PluginPageSection {
  kind: PluginPageSectionKind;
  title?: Localized;
  /** `text` only. Rendered as plain text, never markup. */
  text?: Localized;
  /** `form` only. Defaults to the plugin's full settings schema. */
  schema?: JsonObject;
  /** `form` only. Defaults to the plugin's settings UI hints. */
  uiHints?: JsonObject;
  /** `list` only. Option source id feeding the list rows. */
  optionsFrom?: string;
  /** `tts` only. Plugin action type executed for speech (real execution). */
  actionType?: string;
  /** `tts` only. Option source id feeding the voice selectors. */
  voicesFrom?: string;
  /** `tts` only. Optional option source id feeding the audio output selector. */
  outputsFrom?: string;
}

/** One configuration page contributed by a plugin manifest (schema v3). */
export interface PluginPageDescriptor {
  id: string;
  pluginId: string;
  title: Localized;
  icon?: string;
  sections: PluginPageSection[];
  source: ActionSource;
}
