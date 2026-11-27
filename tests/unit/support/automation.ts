/** Complete builders for automation fixtures: every required field has a default, so tests never cast a partial value. */
import type { ActionTypeDefinition, BehaviorRun, LiveAction, LiveEvent, PluginEventType, PluginStatus } from '../../../src/automation/behavior/types.ts';
import type { AutomationEdit } from '../../../src/web/features/edit-automation-record/model/automation-editor.ts';
import { EVENT_KIND } from '../../shared/index.ts';
import { isRecord } from './json.ts';

export const text = (value: string) => ({ default: value, i18key: '' });

/** An action type definition with every required field filled; `overrides` win. */
export function actionType(overrides: Partial<ActionTypeDefinition> & { id: string }): ActionTypeDefinition {
  return {
    title: text(overrides.id),
    description: text(''),
    tag: '',
    source: { kind: 'builtin' },
    requiredCapabilities: [],
    ...overrides,
  };
}

/** An installed, enabled plugin status for `id`; `overrides` win over the status flags. */
export function pluginStatus(id: string, overrides: Partial<Omit<PluginStatus, 'descriptor'>> = {}): PluginStatus {
  return {
    descriptor: {
      id,
      name: text(id),
      version: '1.0.0',
      description: text(''),
      dependency: text(''),
      permissions: [],
      actionTypeIds: [],
      eventTypeIds: [],
    },
    installed: true,
    enabled: true,
    available: true,
    ...overrides,
  };
}

/** A behavior run with every required field filled; `overrides` win. */
export function behaviorRun(overrides: Partial<BehaviorRun> & { id: string }): BehaviorRun {
  return { at: 0, status: 'ok', actionName: '', summary: '', durationMs: 0, test: false, logs: [], ...overrides };
}

/** A disabled event with no filters or actions; `overrides` win. */
export function liveEvent(overrides: Partial<LiveEvent> = {}): LiveEvent {
  return {
    schemaVersion: 1,
    id: 'event',
    name: 'Event',
    enabled: false,
    trigger: EVENT_KIND.tiktokGift,
    filters: [],
    cooldownMs: 0,
    cooldownScope: 'global',
    actionIds: [],
    runMode: 'all',
    ...overrides,
  };
}

/** A plugin-declared event type; `overrides` win. */
export function pluginEventType(overrides: Partial<PluginEventType> & { type: string }): PluginEventType {
  return { title: text(overrides.type), source: { kind: 'builtin' }, ...overrides };
}

/** A live action with every required field filled; `overrides` win. */
export function liveAction(overrides: Partial<LiveAction> & { id: string }): LiveAction {
  return { schemaVersion: 2, name: overrides.id, typeId: 'say', enabled: true, config: {}, ...overrides };
}

export const isLiveAction = (value: unknown): value is LiveAction =>
  isRecord(value) &&
  (value.schemaVersion === 1 || value.schemaVersion === 2) &&
  typeof value.id === 'string' &&
  typeof value.name === 'string' &&
  typeof value.typeId === 'string' &&
  typeof value.enabled === 'boolean' &&
  isRecord(value.config);

/** Narrow a value that must be an action record (a request body, an acknowledgement); throws a readable error otherwise. */
export function toLiveAction(value: unknown): LiveAction {
  if (!isLiveAction(value)) throw new Error(`Expected a live action, got ${JSON.stringify(value)?.slice(0, 80)}`);
  return value;
}

export const isLiveEvent = (value: unknown): value is LiveEvent =>
  isRecord(value) &&
  value.schemaVersion === 1 &&
  typeof value.id === 'string' &&
  typeof value.name === 'string' &&
  typeof value.trigger === 'string' &&
  Array.isArray(value.filters) &&
  Array.isArray(value.actionIds);

export interface CreateRequest {
  kind: 'action' | 'event';
  record: LiveAction | LiveEvent;
}

/** The `{ kind, record }` body of an automation create call, checked at runtime; throws a readable error otherwise. */
export function createRequest(params: Record<string, unknown> | undefined): CreateRequest {
  const kind = params?.kind;
  const record = params?.record;
  if (kind === 'action' && isLiveAction(record)) return { kind, record };
  if (kind === 'event' && isLiveEvent(record)) return { kind, record };
  throw new Error(`Expected an automation create body, got ${JSON.stringify(params)?.slice(0, 80)}`);
}

/** An automation editor draft with every field filled (an idle, clean event draft by default); `overrides` win. */
export function automationEdit(overrides: Partial<AutomationEdit> = {}): AutomationEdit {
  const candidate = overrides.candidate ?? liveEvent();
  return {
    token: 0,
    generation: 0,
    kind: 'event',
    candidate,
    base: candidate,
    revision: '1',
    create: false,
    busy: false,
    reviewRequired: false,
    autoResolved: false,
    failure: null,
    message: '',
    ...overrides,
  };
}
