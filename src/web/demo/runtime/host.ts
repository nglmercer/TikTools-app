import type { BehaviorRun, LiveAction, LiveEvent } from '../../../automation/behavior/types.ts';
import type { AutomationEvent } from '../../../automation/types.ts';
import { matchesAllFilters } from '../../../automation/behavior/filter-simulation.ts';
import { renderTemplate } from '../../../automation/behavior/templates.ts';
import type { UiEvent, AnalyticsTotals } from '../../../shared/messages.ts';
import type { RequestMetadata, ResourceVersion } from '@/shared/api/control-contracts.ts';
import type { RuntimeVariable } from '@/entities/runtime-global/lib/runtime-variable.ts';
import { isRuntimeVariable, variableText } from '@/entities/runtime-global/lib/runtime-variable.ts';
import { emptyOverlay, validateOverlay, type OverlayDocument } from '../../../shared/cloud-overlay-schema.ts';
import type { ScenarioId } from '../scenarios.ts';
import { scenarioState, DEMO_AT } from '../fixtures/scenario-state.ts';
import { createDemoTransport, demoMethod, DemoRpcError, objectParams, stringParam, type DemoMethod } from './transport.ts';

const clone = <T>(value: T): T => structuredClone(value);
const bounded = (value: unknown, name: string, max = 512) => stringParam(value, name, max);
const fail = (message: string): never => { throw new DemoRpcError('demo_unsupported', message); };
const safeNumber = (value: unknown, fallback = 0) => typeof value === 'number' && Number.isFinite(value) ? Math.max(-10000, Math.min(10000, value)) : fallback;

/** Deliberately explicit host: supported methods mutate bounded memory, never execute external effects. */
export function createDemoHost(scenario: ScenarioId) {
  const { state, snapshot } = scenarioState(scenario);
  const methods: Record<string, DemoMethod> = {};
  const versions = new Map<string, string>();
  const committed = new Set<string>();
  const cooldowns = new Map<string, number>();
  let ticks = 0;
  let paused = true;
  let disposed = false;
  let signedIn = true;
  let logError = false;
  let runSequence = 0;
  let overlaySequence = 1;
  const totals: AnalyticsTotals = { chats: 40, giftEvents: 2, gifts: 3, diamonds: 10, likeEvents: 1, likes: 5, joins: 1, follows: 1, shares: 1, peakViewers: 24 };
  const overlayId = '00000000-0000-4000-8000-000000000001';
  const overlays = new Map<string, { id: string; revision: number; status: string; document: OverlayDocument }>([[overlayId, { id: overlayId, revision: 1, status: 'draft', document: { ...emptyOverlay('chat', 'Demo chat'), elements: [{ id: 'text-1', type: 'text', x: 20, y: 20, width: 600, height: 80, source: { kind: 'field', field: 'chat.message' } }] } }]]);
  const resource = (kind: string, id: string): ResourceVersion => ({ kind: 'automation', scope: kind, id, revision: versions.get(`${kind}:${id}`) ?? (rows(kind).some((r) => r.id === id) ? '1' : '0') });
  function rows(kind: string): Array<LiveAction | LiveEvent> {
    if (kind !== 'action' && kind !== 'event') throw new DemoRpcError('invalid_params', 'Expected action or event.');
    return kind === 'action' ? snapshot.actions : snapshot.events;
  }
  function add<R>(name: string, run: (p: Record<string, unknown>, meta?: RequestMetadata) => R, destructive = false) {
    methods[name] = demoMethod(name, objectParams, (p, meta) => {
      if (disposed) throw new DemoRpcError('transport', 'Demo disposed.');
      // Keep a bounded diagnostic trace without retaining visitor source,
      // credentials or megabyte-sized request objects after dispatch.
      state.calls.push({ method: name, params: { bytes: new TextEncoder().encode(JSON.stringify(p)).length } });
      if (state.calls.length > 160) state.calls.shift();
      const result = clone(run(p, meta));
      committed.add(name);
      return { result };
    }, destructive);
  }
  const status = () => ({ origin: null, state: signedIn ? 'signed-in' : 'signed-out', accountId: signedIn ? 'demo-account' : null, displayName: 'Demo', persistence: 'memory-only', error: null });
  const syncStatus = () => ({ enabled: state.sync!.enabled, signedIn, syncing: false, pending: state.variableComparisons!.length, conflicts: [], lastSyncedAt: state.sync!.lastSyncedAt ?? null, lastError: null });
  add('system.info', () => ({ name: 'tiktools-demo', version: '4.2.2', features: { persistence: false }, demo: true }));
  add('system.health', () => ({ liveIngress: { received: ticks, decoded: ticks, accepted: ticks, processed: ticks, effects: { queued: 0, dropped: 0 } }, liveDedup: { duplicates: 0 }, liveTotals: { unknownMethods: {} } }));
  add('app.state.get', (p) => ({ state: Object.fromEntries(Object.entries(state.appState).filter(([key]) => !Array.isArray(p.keys) || p.keys.includes(key))) }));
  add('app.state.set', (p) => { const key = bounded(p.key, 'key'); const value = bounded(p.value, 'value', 262144); if ((!Object.hasOwn(state.appState, key) && Object.keys(state.appState).length >= 128) || new TextEncoder().encode(JSON.stringify({ ...state.appState, [key]: value })).length > 512 * 1024) fail('Demo storage is full.'); Object.defineProperty(state.appState, key, { value, enumerable: true, configurable: true, writable: true }); return { ok: true }; }, true);
  const updateStatus = () => ({ currentVersion: '4.2.2', availableVersion: null, channel: 'stable', state: 'idle', installationType: 'development', installSupported: false, checkSupported: false, downloadedBytes: 0, totalBytes: null, progress: null, releaseNotes: null, lastChecked: null, error: null });
  add('app.update.status', updateStatus);
  add('app.update.preferences.get', () => ({ channel: 'stable', automaticChecks: false, notifications: false, lastChecked: 0, lastAttempt: 0 }));
  add('app.update.check', () => fail('The public demo does not contact update feeds.'));
  add('app.update.install', () => fail('Installation requires the desktop application.'));
  add('automation.library.snapshot', () => { throw new DemoRpcError('capability_unavailable', 'The demo uses the memory host.'); });
  methods['automation.snapshot'] = demoMethod('Read the simulated automation snapshot', objectParams, () => ({ result: clone(snapshot), meta: { resourceVersions: [...snapshot.actions.map((r) => resource('action', r.id)), ...snapshot.events.map((r) => resource('event', r.id))] } }));
  methods['automation.get'] = demoMethod('Read an automation record', objectParams, (p) => {
    const kind = bounded(p.kind ?? 'event', 'kind'); const id = bounded(p.id, 'id'); const row = rows(kind).find((r) => r.id === id);
    if (!row) throw new DemoRpcError('automation_not_found', 'Record does not exist.');
    return { result: clone(row), meta: { resourceVersions: [resource(kind, id)] } };
  });
  for (const operation of ['create', 'update', 'delete', 'enable', 'disable'] as const) methods[`automation.${operation}`] = demoMethod(`Simulate automation ${operation}`, objectParams, (p, meta) => {
    const kind = bounded(p.kind ?? 'event', 'kind'); const record = operation === 'create' || operation === 'update' ? objectParams(p.record) : undefined;
    const id = bounded(p.id ?? record?.id, 'id'); const list = rows(kind); const index = list.findIndex((r) => r.id === id); const before = resource(kind, id);
    if (record && record.id !== id) throw new DemoRpcError('invalid_params', 'The record identity must match the requested identity.');
    if (record && new TextEncoder().encode(JSON.stringify(record)).length > 8192) throw new DemoRpcError('too_large', 'Demo record exceeds its byte limit.');
    if (!versions.has(`${kind}:${id}`) && versions.size >= 512) fail('Demo revision limit reached; reset the scenario.');
    if (meta?.expectedRevision !== undefined && meta.expectedRevision !== before.revision) throw new DemoRpcError('revision_conflict', 'Record changed; reload the comparison.', { resourceVersions: [before] });
    if (operation === 'create' ? index >= 0 : index < 0) throw new DemoRpcError('revision_conflict', 'Record existence changed.');
    if (list.length >= 100 && operation === 'create') fail('Demo record limit reached.');
    if (record && (typeof record.name !== 'string' || record.name.length > 200 || (kind === 'action' ? typeof record.typeId !== 'string' || !record.config : typeof record.trigger !== 'string' || !Array.isArray(record.actionIds) || !Array.isArray(record.filters)))) throw new DemoRpcError('invalid_params', 'Invalid automation record.');
    if (operation === 'delete') { list.splice(index, 1); if (kind === 'action') for (const event of snapshot.events) event.actionIds = event.actionIds.filter((v) => v !== id); }
    else if (operation === 'create') list.push(clone(record as unknown as LiveAction | LiveEvent));
    else if (operation === 'update') list[index] = clone(record as unknown as LiveAction | LiveEvent);
    else list[index]!.enabled = operation === 'enable';
    versions.set(`${kind}:${id}`, String(BigInt(before.revision) + 1n));
    committed.add(`automation.${operation}`);
    return { result: operation === 'delete' ? { id, deleted: true } : clone(list.find((r) => r.id === id)), meta: { resourceVersions: [resource(kind, id)], ...(meta?.commandId ? { command: { commandId: meta.commandId, state: 'succeeded' as const, committedAt: DEMO_AT + ticks } } : {}) } };
  }, true);
  add('automation.runs', () => ({ runs: state.automationRuns }));
  function sample(type: AutomationEvent['type'] = 'tiktok.chat'): AutomationEvent {
    return { id: `demo-message-${ticks}`, type, timestamp: DEMO_AT + ticks * 1000, user: { uniqueId: 'luna', nickname: 'Luna', secUid: 'demo', userId: '1', avatarUrl: null }, data: { comment: ticks % 2 ? '!hello' : 'Hello TikTools!', giftName: 'Rose', repeatCount: 1, diamondCount: 1 }, creator: { uniqueId: 'demo_creator', roomId: 'demo-room' } };
  }
  function runAction(action: LiveAction, eventId?: string, test = true, event = sample()): BehaviorRun {
    if (runSequence >= 10000) fail('Demo execution limit reached; reset the scenario.');
    const ok = action.typeId === 'core.log' && !logError;
    const summary = ok ? bounded(action.config.message ?? 'Demo log', 'message', 2000).replace(/\{\{\s*([^}]+?)\s*\}\}/g, (placeholder, rawPath: string) => {
      const path = rawPath.trim();
      if (!path.startsWith('globals.')) return renderTemplate(placeholder, event);
      const name = path.slice(8); const value = Object.hasOwn(state.typedGlobals ?? {}, name) ? state.typedGlobals![name] : undefined;
      return value ? value.type === 'secret' ? '••••••••' : variableText(value) : '';
    }) : logError && action.typeId === 'core.log' ? 'Demo: simulated action failure. Disable the failure simulation to retry.' : 'Demo: external effects and scripts are unavailable.';
    const run: BehaviorRun = { id: `demo-run-${++runSequence}`, at: DEMO_AT + ticks * 1000, status: ok ? 'ok' : 'error', actionId: action.id, actionName: action.name, eventId, summary, durationMs: 1, test, logs: [summary], ...(ok ? {} : { error: summary }), eventSource: test ? 'sample' : 'live' };
    state.automationRuns.push(run as unknown as Record<string, unknown>); if (state.automationRuns.length > 60) state.automationRuns.shift(); return run;
  }
  add('automation.test', (p) => {
    const record = objectParams(p.record);
    if (p.kind === 'action') return runAction(record as unknown as LiveAction);
    const event = record as unknown as LiveEvent; const action = snapshot.actions.find((r) => event.actionIds?.includes(r.id));
    if (!action) throw new DemoRpcError('invalid_params', 'Choose a configured log action first.');
    return runAction(action, event.id);
  });
  add('automation.fire', (p) => { const event = objectParams(p.record) as unknown as LiveEvent; const action = snapshot.actions.find((r) => event.actionIds?.includes(r.id)); if (!action) fail('Choose an action.'); const run = runAction(action!, event.id); return { trigger: event.trigger, eventSource: 'sample', pinned: true, matched: 1, draftMatched: 1, status: run.status, summary: run.summary, durationMs: 1 }; });
  add('automation.script.analyze', () => fail('Visitor code is never analyzed or executed in this demo.'));
  add('live.platforms', () => state.livePlatforms ?? []);
  add('live.connections', () => state.liveConnections ?? []);
  add('live.status', () => state.liveStatus);
  add('live.connect', (p) => { const uniqueId = bounded(p.uniqueId ?? 'demo_creator', 'uniqueId', 64); state.liveStatus = { connected: true, uniqueId, roomId: 'demo-room', connectionId: 'demo-connection', native: false }; wire.emitTopic('live.connected', state.liveStatus); return state.liveStatus; });
  add('live.disconnect', () => { state.liveStatus.connected = false; wire.emitTopic('live.disconnected', {}); return state.liveStatus; });
  add('live.channels', () => ({ rooms: [], nextOffset: null, fetchedAt: DEMO_AT, source: 'demo' }));
  const monitor = { enabled: false, phase: 'stopped', uniqueId: 'demo_creator', pollIntervalSeconds: 30, checks: 0, failures: 0, lastCheckedAt: null as number | null, nextCheckAt: null as number | null, lastError: null };
  add('live.monitor.status', () => monitor);
  add('live.monitor.start', (p) => { Object.assign(monitor, { enabled: true, phase: 'waiting', uniqueId: bounded(p.uniqueId, 'uniqueId', 64), checks: monitor.checks + 1, lastCheckedAt: DEMO_AT }); return monitor; });
  add('live.monitor.stop', () => { Object.assign(monitor, { enabled: false, phase: 'stopped' }); return monitor; });
  add('creators.get', () => ({ creator: state.creator })); add('creators.recent', () => ({ creators: state.recentCreators })); add('gifts.list', () => ({ gifts: [] }));
  add('points.config.get', () => state.pointsConfig); add('points.config.set', (p) => { for (const [key, value] of Object.entries(p)) if (Object.hasOwn(state.pointsConfig, key) && ['number', 'boolean', 'string'].includes(typeof value)) state.pointsConfig[key] = typeof value === 'number' ? safeNumber(value) : value; return state.pointsConfig; });
  add('points.leaderboard', () => ({ viewers: state.leaderboard }));
  add('points.adjust', (p) => { const viewer = state.leaderboard.find((v) => v.uniqueId === p.uniqueId); if (!viewer) throw new DemoRpcError('not_found', 'Unknown demo viewer.'); viewer.points = Math.max(0, safeNumber(viewer.points) + safeNumber(p.delta)); viewer.level = Math.floor(Number(viewer.points) / Math.max(10, safeNumber(state.pointsConfig.pointsPerLevel, 100))) + 1; return { ok: true }; });
  add('points.reset', () => { for (const v of state.leaderboard) { v.points = 0; v.level = 1; } return { ok: true }; });
  add('analytics.summary', (p) => ({ summary: { creatorUniqueId: p.creatorUniqueId, startDay: p.startDay, endDay: p.endDay, totals, days: [{ day: p.startDay, ...totals }], hours: [], sessions: 1, topViewers: [{ uniqueId: 'luna', chats: totals.chats, gifts: totals.gifts, diamonds: totals.diamonds, likes: totals.likes, shares: totals.shares, interactions: totals.chats + totals.gifts + totals.likes, lastSeen: DEMO_AT + ticks * 1000 }] } }));
  add('globals.list', () => ({ globals: state.globals }));
  const variables = () => ({ revision: state.typedGlobalsRevision ?? 'a'.repeat(64), variables: state.typedGlobals });
  add('globals.typed.list', variables);
  add('globals.typed.replace', (p) => { if (p.expectedRevision !== variables().revision) throw new DemoRpcError('conflict', 'Variables changed.'); const next = objectParams(p.variables); if (Object.keys(next).length > 60 || !Object.values(next).every(isRuntimeVariable)) throw new DemoRpcError('invalid_params', 'Invalid demo variables.'); state.typedGlobals = clone(next as Record<string, RuntimeVariable>); state.globals = Object.fromEntries(Object.entries(state.typedGlobals).map(([key, value]) => [key, variableText(value)])); state.typedGlobalsRevision = (BigInt(`0x${variables().revision}`) + 1n).toString(16).padStart(64, '0'); return variables(); });
  add('processors.status', () => ({ processors: [] }));
  add('cloud.status', status); add('cloud.restore', status); add('cloud.logout', () => { signedIn = false; return status(); });
  add('cloud.login.reserve', () => ({ operationId: 'demo-login' })); add('cloud.login.begin', () => ({ browserUrl: '#demo-account' })); add('cloud.login.open', () => null); add('cloud.login.complete', () => { signedIn = true; return status(); }); add('cloud.login.cancel', status);
  add('sync.status', syncStatus); add('sync.configure', (p) => { state.sync!.enabled = p.enabled === true; return syncStatus(); });
  add('sync.v2.selected', () => true);
  add('sync.v2.section', (p) => { if (p.section !== 'plugins') fail('This Cloud section is unavailable in the demo.'); return { pushed: 0, pulled: 0, workspacePending: 0, availableRecords: 0, conflicts: [], moreQueued: false }; });
  add('sync.v2.changes.list', () => []); add('sync.v2.variables.review', () => ({ changes: state.variableComparisons })); add('sync.v2.variables.fetch', () => ({ conflicts: [] }));
  for (const operation of ['send', 'apply', 'withdraw']) add(`sync.v2.variables.${operation}`, (p) => {
    const row = state.variableComparisons!.find((r) => r.review === p.review); if (!row) throw new DemoRpcError('conflict', 'The displayed comparison is no longer current.');
    if (operation === 'apply') { if (row.cloud) state.typedGlobals![row.name] = clone(row.cloud); else delete state.typedGlobals![row.name]; state.typedGlobalsRevision = (BigInt(`0x${variables().revision}`) + 1n).toString(16).padStart(64, '0'); }
    state.globals = Object.fromEntries(Object.entries(state.typedGlobals ?? {}).map(([key, value]) => [key, variableText(value)])); state.variableComparisons = state.variableComparisons!.filter((r) => r !== row); return operation === 'apply' ? null : { conflicts: [] };
  });
  add('catalog.items', () => ({ items: [], nextCursor: null, dataState: 'ready' })); add('catalog.mine', () => ({ items: [], nextCursor: null, dataState: 'ready' })); add('cloudLibrary.list', () => ({ items: [], nextCursor: null }));
  add('registry.list', () => ({ registries: [] })); add('registry.defaults.get', () => ({ channel: 'stable', autoUpdate: 'notify', checkIntervalHours: 12, allowSideload: false }));
  add('plugins.packages.receipts', () => ({ receipts: [] })); add('plugins.jobs.list', () => ({ jobs: [] })); add('plugins.updates.list', () => ({ updates: [], checkedAt: DEMO_AT, errors: [] }));
  add('plugins.settings.get', (p) => { const pluginId = bounded(p.pluginId, 'pluginId'); const settings = Object.hasOwn(state.pluginSettings, pluginId) ? state.pluginSettings[pluginId] : undefined; if (!settings) throw new DemoRpcError('not_found', 'Unknown plugin.'); return { pluginId, ...settings }; });
  add('plugins.settings.set', (p) => { const pluginId = bounded(p.pluginId, 'pluginId'); const settings = Object.hasOwn(state.pluginSettings, pluginId) ? state.pluginSettings[pluginId] : undefined; if (!settings) throw new DemoRpcError('not_found', 'Unknown plugin.'); const values = objectParams(p.values); settings.values = { enabled: values.enabled === true, volume: Math.max(0, Math.min(100, safeNumber(values.volume, 50))) }; return { pluginId, ...settings }; });
  add('plugins.health', (p) => ({ pluginId: p.pluginId, ...(state.pluginHealth[String(p.pluginId)] ?? { ok: false, error: 'No simulated service.', latencyMs: 0 }) }));
  add('plugins.action.execute', () => fail('Plugin effects require the real host. Demo log actions remain available.'));
  add('plugins.toggle', (p) => { const plugin = snapshot.plugins.find((v) => v.descriptor.id === p.pluginId); if (!plugin) throw new DemoRpcError('not_found', 'Unknown plugin.'); plugin.enabled = p.enabled === true; return { ok: true }; });
  for (const operation of ['enable', 'disable', 'install.set', 'uninstall']) add(`plugins.${operation}`, (p) => { const plugin = snapshot.plugins.find((v) => v.descriptor.id === p.pluginId); if (!plugin) throw new DemoRpcError('not_found', 'Unknown plugin.'); if (operation === 'enable' || operation === 'disable') plugin.enabled = operation === 'enable'; else plugin.installed = operation === 'install.set' && p.installed === true; return { ok: true }; });
  add('sync.v2.stagePluginSettings', () => null);
  add('automation.set_enablement', (p) => { if (!Array.isArray(p.updates) || p.updates.length > 200) throw new DemoRpcError('invalid_params', 'Invalid enablement batch.'); let updated = 0; const missing: Array<{ kind: string; id: string }> = []; for (const value of p.updates) { const update = objectParams(value); const kind = bounded(update.kind, 'kind'); const id = bounded(update.id, 'id'); const row = rows(kind).find((r) => r.id === id); if (!row) missing.push({ kind, id }); else { row.enabled = update.enabled === true; updated++; } } return { updated, missing }; });
  add('widgets.status', () => ({ state: 'ready', port: 17452, error: null }));
  add('widgets.copyObsUrl', () => ({ ok: true, simulated: true })); add('widgets.copyCloudOverlayUrl', () => ({ ok: true, simulated: true }));
  add('bundles.assets.list', () => ({ items: [], nextCursor: null }));
  add('bundles.overlays.list', () => ({ items: [...overlays.values()].map((o) => ({ id: o.id, kind: o.document.kind, name: o.document.title, revision: o.revision, status: o.status, createdAt: new Date(DEMO_AT).toISOString(), updatedAt: new Date(DEMO_AT).toISOString(), elementCount: o.document.elements.length })), nextCursor: null }));
  add('bundles.overlays.read', (p) => { const row = overlays.get(String(p.id)); if (!row) throw new DemoRpcError('not_found', 'Overlay not found.'); return row; });
  for (const operation of ['create', 'revise']) add(`bundles.overlays.${operation}`, (p) => { const document = clone(objectParams(p.document)) as unknown as OverlayDocument; try { if (validateOverlay(document).length) throw new Error(); } catch { throw new DemoRpcError('invalid_params', 'Invalid overlay document.'); } const id = operation === 'create' ? `00000000-0000-4000-8000-${String(++overlaySequence).padStart(12, '0')}` : bounded(p.id, 'id'); const before = overlays.get(id); if (operation === 'revise' && (!before || before.revision !== p.expectedRevision)) throw new DemoRpcError('revision_conflict', 'Overlay changed.'); if (overlays.size >= 20) fail('Demo overlay limit reached.'); const row = { id, document, revision: (before?.revision ?? 0) + 1, status: 'draft' }; overlays.set(id, row); return { id, revision: row.revision, status: row.status }; });
  add('bundles.overlays.delete', (p) => { overlays.delete(String(p.id)); return { id: p.id, deleted: true }; });
  const c = state.collaboration!;
  add('collaboration.status', () => c.status);
  add('collaboration.configure', (p) => { if (p.expectedRevision !== c.status.configurationRevision) throw new DemoRpcError('revision_conflict', 'Configuration changed.'); const configuration = objectParams(p.configuration ?? p); c.status.configuration.enabled = configuration.enabled === true; c.status.configurationRevision = String(BigInt(c.status.configurationRevision) + 1n); c.status.gateway.running = c.status.configuration.enabled; return c.status; });
  for (const [key, property] of [['invites', 'invitations'], ['requests', 'requests'], ['principals', 'principals'], ['sessions', 'sessions'], ['audit', 'audit']] as const) add(`collaboration.${key}.list`, () => ({ [key === 'audit' ? 'entries' : property]: c[property], nextAfter: key === 'audit' ? 0 : null }));
  add('collaboration.invites.create', (p) => { if (c.invitations.length >= 10) fail('Demo invitation limit reached.'); const grant = clone(objectParams(p.grant)) as unknown as typeof c.issued extends never ? never : typeof c.requests[number]['grant']; c.invitations.push({ inviteId: `demo-invite-${c.invitations.length}`, grant, expiresAt: DEMO_AT + 3600000, state: 'created' }); return { ...c.issued, inviteId: c.invitations.at(-1)!.inviteId }; });
  add('collaboration.invites.revoke', (p) => { const row = c.invitations.find((r) => r.inviteId === p.id); if (row) row.state = 'revoked'; return { ok: true }; });
  for (const operation of ['approve', 'reject']) add(`collaboration.requests.${operation}`, (p) => { const row = c.requests.find((r) => r.requestId === p.id); if (!row || row.state !== 'pending') throw new DemoRpcError('not_found', 'Request is no longer pending.'); row.state = operation === 'approve' ? 'approved' : 'rejected'; c.status.pendingRequests = c.requests.filter((r) => r.state === 'pending').length; if (operation === 'approve') c.principals.push({ principalId: 'demo-reviewer', clientLabel: row.clientLabel, clientKind: row.clientKind, grant: clone(row.grant), grantRevision: '1', revoked: false }); return { ok: true }; });
  add('collaboration.principals.revoke', (p) => { const row = c.principals.find((r) => r.principalId === p.id); if (row) row.revoked = true; return { ok: true }; });
  add('collaboration.sessions.revoke', () => ({ ok: true }));
  add('teams.accessible.list', () => ({ items: [] })); add('teams.hosts.list', () => ({ items: [] })); add('teams.devices.list', () => ({ items: [] }));
  add('teams.host.status', () => ({ state: 'stopped', hostId: null, lastError: null }));
  for (const method of ['teams.hosts.create', 'teams.invitations.redeem']) add(method, () => fail('Cloud teams and cross-device joins require a matching native host and backend. This demo supports local invitation and permission review in Collaboration.'));
  // Feed internally provisions definitions; the hidden Rankings product remains gated.
  const rankings = new Map<string, Record<string, unknown>>();
  add('rankings.list', () => ({ rankings: [...rankings.values()] })); add('rankings.create', (p) => { if (rankings.size >= 16) fail('Demo ranking limit reached.'); rankings.set(bounded(p.id, 'id'), p); return { ranking: p }; }); add('rankings.update', (p) => { const definition = objectParams(p.definition); const id = bounded(definition.id, 'id'); if (!rankings.has(id)) throw new DemoRpcError('not_found', 'Unknown ranking.'); rankings.set(id, definition); return { ranking: definition }; }); add('rankings.snapshot', (p) => ({ id: p.id, entries: [], total: 0, revision: 0 }));
  add('ui.feed.backfill', (p) => { const events = (state.feedJournal ?? []).filter((row) => row.cursor > safeNumber(p.after)).slice(0, 60); return { oldestCursor: state.feedJournal?.[0]?.cursor ?? 0, latestCursor: ticks, depth: state.feedJournal?.length ?? 0, gap: false, truncated: false, events }; });
  const wire = createDemoTransport(methods);
  function step(): boolean {
    if (disposed || !state.liveStatus.connected || ticks >= 300 || wire.diagnostics().queuedResponses > 20) return false;
    if (ticks === 0) wire.emitTopic('live.connected', state.liveStatus);
    ticks += 1;
    const kind = ticks % 3 === 0 ? 'gift' : ticks % 3 === 2 ? 'like' : 'chat';
    const event: UiEvent = { kind, author: 'luna', nickname: 'Luna', text: kind === 'chat' ? 'Hello TikTools!' : kind === 'gift' ? 'Rose' : '5 likes', msgId: `demo-message-${ticks}`, correlationId: `demo-message-${ticks}`, feedCursor: ticks, roomId: 'demo-room', creatorId: 'demo_creator', sourceType: kind, points: safeNumber(state.leaderboard[0]!.points) + 1, pointsDelta: 1, level: 2, ...(kind === 'gift' ? { giftDetails: { name: 'Rose', count: 1, diamonds: 1, repeatEnd: true } } : {}), ...(kind === 'like' ? { likeCount: 5, likeUserTotal: ticks * 5 } : {}) };
    state.leaderboard[0]!.points = event.points; state.leaderboard[0]!.level = Math.floor(Number(event.points) / Math.max(10, safeNumber(state.pointsConfig.pointsPerLevel, 100))) + 1; event.level = Number(state.leaderboard[0]!.level);
    if (kind === 'chat') totals.chats += 1; if (kind === 'gift') { totals.gifts += 1; totals.diamonds += 1; } if (kind === 'like') totals.likes += 5;
    (state.feedJournal ??= []).push({ cursor: ticks, event: event as unknown as Record<string, unknown> }); if (state.feedJournal.length > 60) state.feedJournal.shift();
    wire.emitTopic('live.ui-event', { event, cursor: ticks, sequence: ticks });
    wire.emitTopic('points.changed', { uniqueId: 'luna', delta: 1, totalPoints: event.points, level: event.level });
    wire.emitTopic('room.stats', { viewers: 24, totalUsers: 100, topViewers: [] });
    const envelope = sample(kind === 'chat' ? 'tiktok.chat' : kind === 'gift' ? 'tiktok.gift' : 'tiktok.like');
    for (const trigger of snapshot.events) {
      const now = DEMO_AT + ticks * 1000;
      if (!trigger.enabled || trigger.trigger !== envelope.type || !matchesAllFilters(trigger.filters, envelope) || now - (cooldowns.get(trigger.id) ?? 0) < trigger.cooldownMs) continue;
      cooldowns.set(trigger.id, now);
      const ids = trigger.runMode === 'random' ? trigger.actionIds.slice(ticks % Math.max(1, trigger.actionIds.length), ticks % Math.max(1, trigger.actionIds.length) + 1) : trigger.actionIds;
      for (const id of ids) { const action = snapshot.actions.find((a) => a.id === id && a.enabled); if (action) wire.emitTopic('automation.run.completed', { run: runAction(action, trigger.id, false, envelope) }); }
    }
    return true;
  }
  return { ...wire, state, step, get paused() { return paused; }, setPaused(value: boolean) { paused = value; }, tick() { return !paused && step(); }, simulateError(value: boolean) { logError = value; },
    diagnostics: () => ({ ...wire.diagnostics(), ticks, paused, disposed, calls: state.calls.length, events: state.feedJournal?.length ?? 0, runs: state.automationRuns.length }),
    wasSuccessful(method: string) { return committed.has(method); },
    pluginInstalled() { return snapshot.plugins.some((plugin) => plugin.installed); },
    dispose() { disposed = true; paused = true; wire.transport.detach(); committed.clear(); versions.clear(); cooldowns.clear(); state.calls = []; state.feedJournal = []; state.automationRuns = []; },
  };
}
