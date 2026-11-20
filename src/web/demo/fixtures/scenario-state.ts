import type { BehaviorSnapshot } from '../../../automation/behavior/types.ts';
import { baseHostState } from './base-host.ts';
import type { ScenarioId } from '../scenarios.ts';
import { demoViewers, demoGifts } from './audience.ts';

export const DEMO_AT = 1_760_000_000_000;
const text = (value: string) => ({ default: value, i18key: `demo.${value.replaceAll(/[^a-zA-Z]/g, '').slice(0, 40)}` });
export const logType = {
  id: 'core.log', version: 1,
  title: { default: 'Write to the log', i18key: 'automation.action.core.log.title' },
  description: { default: 'Leave a line in the automation log.', i18key: 'automation.action.core.log.description' },
  tag: 'log', source: { kind: 'builtin' }, requiredCapabilities: [],
  fields: [{ key: 'message', kind: 'textarea', label: { default: 'Message', i18key: 'automation.action.core.log.field.message.label' }, value: 'Hello {{ user.nickname }}', template: true }],
} satisfies BehaviorSnapshot['actionTypes'][number];
export function scenarioState(id: ScenarioId) {
  const snapshot: BehaviorSnapshot = {
    actions: [{ schemaVersion: 2, id: 'welcome-action', name: 'Welcome', typeId: 'core.log', enabled: true, config: { message: 'Hello {{ user.nickname }}' } }],
    events: [{ schemaVersion: 1, id: 'welcome-event', name: 'Welcome chat', enabled: true, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'global', actionIds: ['welcome-action'], runMode: 'all' }],
    actionTypes: [logType], plugins: [{ installed: true, enabled: true, available: true, running: true,
      descriptor: { id: 'demo.audio', name: text('Demo Audio'), version: '1.0.0', description: text('Simulated audio integration; no service or audio device is contacted.'), dependency: text('Demo runtime'), permissions: ['audio.play'], actionTypeIds: [], eventTypeIds: [], hasSettings: true, hasConnectionProbe: true } }],
    eventTypes: [], pluginTemplates: [], pluginPages: [], translations: {},
  };
  const state = baseHostState({
    automationSnapshot: snapshot as unknown as Record<string, unknown>,
    liveStatus: { connected: id !== 'connection', uniqueId: id !== 'connection' ? 'demo_creator' : null, roomId: 'demo-room', connectionId: 'demo-connection', native: false },
    creator: { uniqueId: 'demo_creator', roomId: 'demo-room', nickname: 'Demo Creator', avatarUrl: null, title: 'Fictional LIVE', lastConnected: DEMO_AT, connectCount: 1 },
    recentCreators: [{ uniqueId: 'demo_creator', nickname: 'Demo Creator', avatarUrl: null, roomId: 'demo-room', lastConnected: DEMO_AT, connectCount: 1 }],
    leaderboard: demoViewers(DEMO_AT),
    gifts: demoGifts.map((gift) => ({ ...gift })),
    typedGlobals: { greeting: { type: 'text', value: 'Hello' }, goal: { type: 'number', value: 100 } }, globals: { greeting: 'Hello', goal: '100' },
    sync: { signedIn: true, enabled: true, entries: [], resolved: [] },
    variableComparisons: [{ name: 'greeting', review: 'demo-review-1', local: { type: 'text', value: 'Hello' }, cloud: { type: 'text', value: 'Welcome' }, cloudDeleted: false, localSecret: false, blockedLocal: false, pendingStatus: null, canSend: true, canApply: true, canWithdraw: false }],
    pluginSettings: { 'demo.audio': { schema: { type: 'object', properties: { volume: { type: 'number', minimum: 0, maximum: 100, default: 50 }, enabled: { type: 'boolean', default: true } } }, values: { volume: 50, enabled: true } } },
    pluginHealth: { 'demo.audio': { ok: true, latencyMs: 2 } },
  });
  return { state, snapshot };
}
