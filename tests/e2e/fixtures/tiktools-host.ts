/**
 * Reusable Playwright fake of the TikTools native host.
 *
 * Installed via `page.addInitScript()` BEFORE `page.goto()`, at the exact
 * public frontend boundary: `window.ipc.postMessage()` out,
 * `window.__webview_on_message__()` back. Behavior is data-driven through
 * the mutable `window.__TIKTOOLS_E2E_STATE__` object so specs can change
 * host responses mid-test with `page.evaluate()`.
 *
 * Unexpected RPC methods fail loudly: the dispatcher records them in
 * `state.unhandled` and answers with a JSON-RPC error, and
 * `assertNoUnhandledCalls()` fails the test at teardown.
 */

import type { Page } from '@playwright/test';
import type { CollaborationStatus, Invitation, InvitationSecret, PairingRequest, Principal, Session, AuditEntry } from '../../../src/web/entities/collaboration/api/types.ts';

export interface E2EHostState {
  collaboration?: {
    status: CollaborationStatus;
    invitations: Invitation[]; issued: InvitationSecret; requests: PairingRequest[];
    principals: Principal[]; sessions: Session[]; audit: AuditEntry[];
  };
  /** Methods announced through `rpc.discover`; none by default, so optional Cloud areas stay hidden. */
  announcedMethods?: string[];
  /** Runtime globals served by `globals.list` and written by `globals.set`. */
  globals?: Record<string, string>;
  /** Behavior snapshot served for `automation.snapshot`. */
  automationSnapshot: Record<string, unknown>;
  automationVersions?: Record<string, string>;
  automationDelayMs?: number;
  automationRuns: Array<Record<string, unknown>>;
  pointsConfig: Record<string, unknown>;
  leaderboard: Array<Record<string, unknown>>;
  creator: Record<string, unknown> | null;
  recentCreators: Array<Record<string, unknown>>;
  gifts: Array<Record<string, unknown>>;
  processors: Array<Record<string, unknown>>;
  /** `plugins.settings.get/set` store, keyed by plugin id. */
  pluginSettings: Record<
    string,
    { schema: Record<string, unknown>; uiHints?: Record<string, unknown>; values: Record<string, unknown> }
  >;
  /** `plugins.options` store, keyed by source id. */
  pluginOptions: Record<string, { options: Array<{ value: string; label: string }>; selected?: string | null }>;
  /** `plugins.health` results, keyed by plugin id. */
  pluginHealth: Record<string, { ok: boolean; latencyMs: number; error?: string | null; reachable?: boolean; gameplayReady?: boolean }>;
  pluginTokenCandidates?: Record<string, Array<{ id: string; label: string }>>;
  pluginSetupEndpoints?: Record<string, string>;
  pluginSetupHealth?: E2EHostState['pluginHealth'];
  /** `plugins.action.execute` results, keyed by action type. */
  actionOutcomes: Record<
    string,
    { ok: boolean; summary: string; logs?: string[]; error?: string | null }
  >;
  /** `app.state.*` key/value store. */
  appState: Record<string, string>;
  /** Authoritative `live.status`/`live.connect`/`live.disconnect` result. */
  liveStatus: {
    connected: boolean;
    uniqueId: string | null;
    roomId: string | null;
    connectionId: string | null;
    native: boolean;
  };
  /** Page served for `live.channels` (read-only LIVE browse). */
  liveMonitor?: Record<string, unknown>;
  feedJournal?: Array<{ cursor: number; event: Record<string, unknown> }>;
  liveChannels: { rooms: Array<Record<string, unknown>>; nextOffset?: number };
  /** Methods forced to fail: method -> message, or `{ code, message }` to
   *  exercise a specific JSON-RPC error code. */
  failures: Record<string, string | { code: string; message: string }>;
  /** Artificial delay (ms) for `plugins.action.execute` responses. */
  actionDelayMs: number;
  /** Every RPC call received, in order. */
  calls: Array<{ method: string; params: Record<string, unknown>; meta?: Record<string, unknown> }>;
  /** Calls with no fake handler — must stay empty. */
  unhandled: string[];
  /** Legacy `type:` messages (plugin-ui-open/close), in order. */
  legacyMessages: Array<Record<string, unknown>>;
  frontendReady: boolean;
}

export function baseHostState(overrides: Partial<E2EHostState> = {}): E2EHostState {
  return {
    automationSnapshot: {
      actions: [],
      events: [],
      plugins: [],
      actionTypes: [],
      eventTypes: [],
      pluginTemplates: [],
      pluginPages: [],
      translations: {},
    },
    automationRuns: [],
    pointsConfig: {
      currencyName: 'Points',
      pointsPerCoin: 1.0,
      pointsPerCoinEnabled: true,
      pointsPerShare: 3.0,
      pointsPerShareEnabled: true,
      pointsPerChat: 1.0,
      pointsPerChatEnabled: true,
      pointsPerLike: 0.1,
      pointsPerLikeEnabled: true,
      pointsPerFollow: 5.0,
      pointsPerFollowEnabled: true,
      pointsPerJoin: 0.5,
      pointsPerJoinEnabled: false,
      subBonusMultiplier: 0.0,
      pointsPerLevel: 100,
    },
    leaderboard: [],
    creator: null,
    recentCreators: [],
    gifts: [],
    processors: [],
    pluginSettings: {},
    pluginOptions: {},
    pluginHealth: {},
    actionOutcomes: {},
    appState: {},
    liveStatus: {
      connected: false,
      uniqueId: null,
      roomId: null,
      connectionId: null,
      native: true,
    },
    liveChannels: { rooms: [] },
    failures: {},
    actionDelayMs: 0,
    calls: [],
    unhandled: [],
    legacyMessages: [],
    frontendReady: false,
    ...overrides,
  };
}

const INIT_SCRIPT = `
(() => {
  // The dispatcher closes over this object identity: writeHostState mutates
  // it in place (never replaces it) so mid-test changes take effect.
  const state = window.__TIKTOOLS_E2E_STATE__;
  const calls = state.calls;

  const respond = (id, result, error, meta) => {
    const response = { jsonrpc: '2.0', id };
    if (error !== undefined) response.error = error;
    else response.result = result;
    if (meta !== undefined) response.meta = meta;
    const raw = JSON.stringify({ type: 'rpc-response', response });
    queueMicrotask(() => {
      if (typeof window.__webview_on_message__ === 'function') {
        window.__webview_on_message__(raw);
      }
    });
  };

  const fail = (id, method, message) => {
    respond(id, undefined, { code: 'e2e', message: message || ('fake host failure: ' + method) });
  };

  const behaviorRows = (kind) => {
    if (kind !== 'action' && kind !== 'event') throw { code: 'invalid_params', message: 'Unknown behavior family' };
    return state.automationSnapshot[kind === 'action' ? 'actions' : 'events'];
  };
  const behaviorRevision = (kind, id) => {
    const versions = state.automationVersions ||= {};
    const key = kind + ':' + id;
    if (versions[key] === undefined) versions[key] = behaviorRows(kind).some((r) => r.id === id) ? '1' : '0';
    return versions[key];
  };
  const behaviorMeta = (kind, id) => ({ kind: 'automation', scope: kind, id, revision: behaviorRevision(kind, id) });
  const behaviorEnvelope = (value, versions) => ({ __e2e_value: value, __e2e_meta: { resourceVersions: versions } });
  const writeBehavior = (method, params, meta) => {
    const kind = params.kind || 'event'; const id = params.id || params.record?.id;
    const rows = behaviorRows(kind); const index = rows.findIndex((r) => r.id === id);
    const before = behaviorRevision(kind, id);
    if (meta?.expectedRevision !== before) throw { code: 'revision_conflict', message: 'Host record changed', data: { resourceVersions: [behaviorMeta(kind, id)] } };
    if (method === 'create' && index >= 0) throw { code: 'revision_conflict', message: 'Record already exists' };
    if (['update', 'enable', 'disable'].includes(method) && index < 0) throw { code: 'automation_not_found', message: 'Record was deleted', data: { resourceVersions: [behaviorMeta(kind, id)] } };
    const affected = [];
    if (method !== 'delete' || index >= 0) state.automationVersions[kind + ':' + id] = String(BigInt(before) + 1n);
    if (method === 'delete') {
      if (index >= 0) rows.splice(index, 1);
      if (kind === 'action') for (const event of behaviorRows('event')) {
        if (!event.actionIds.includes(id)) continue;
        const previous = behaviorRevision('event', event.id);
        event.actionIds = event.actionIds.filter((value) => value !== id);
        state.automationVersions['event:' + event.id] = String(BigInt(previous) + 1n);
        affected.push(behaviorMeta('event', event.id));
      }
      return behaviorEnvelope({ id, deleted: index >= 0 }, [behaviorMeta(kind, id), ...affected]);
    }
    const record = method === 'enable' || method === 'disable' ? { ...rows[index], enabled: method === 'enable' } : { ...params.record, id };
    if (index < 0) rows.push(record); else rows[index] = record;
    return behaviorEnvelope(record, [behaviorMeta(kind, id)]);
  };

  const handlers = {
    // The fake host serves automation.snapshot from memory, so it reports
    // the same non-persistent feature set a memory-only host would.
    'system.info': () => ({ name: 'tiktools', features: { persistence: false } }),
    'rpc.discover': () => ({ methods: (state.announcedMethods ?? []).map((name) => ({ name })) }),
    // The fake host is memory-backed and has no Owner library; answer the
    // optional read with a capability error instead of leaving it unknown.
    'automation.library.snapshot': () => {
      throw { code: 'capability_unavailable', message: 'The fake host has no Owner library.' };
    },
    'collaboration.status': () => state.collaboration.status,
    'collaboration.configure': (params) => {
      const c = state.collaboration;
      if (params.expectedRevision !== undefined && params.expectedRevision !== c.status.configurationRevision) throw { code: 'revision_conflict', message: 'Configuration changed' };
      const { expectedRevision, ...configuration } = params;
      c.status.configuration = configuration;
      c.status.configurationRevision = (BigInt(c.status.configurationRevision) + 1n).toString();
      c.status.gateway.running = configuration.enabled;
      c.status.gateway.configurationRevision = c.status.configurationRevision;
      if (!configuration.enabled) {
        for (const session of c.sessions) session.revoked = true;
        for (const invite of c.invitations) if (invite.state === 'created') invite.state = 'revoked';
        for (const request of c.requests) if (['pending', 'approved'].includes(request.state)) request.state = 'revoked';
        c.status.activeSessions = 0; c.status.pendingRequests = 0;
        c.status.gateway.connections = 0; c.status.gateway.sessionConnections = {};
      }
      return c.status;
    },
    'collaboration.invites.create': (params) => {
      const c = state.collaboration;
      c.invitations.push({ inviteId: c.issued.inviteId, expiresAt: c.issued.expiresAt, grant: params.grant, state: 'created' });
      return c.issued;
    },
    'collaboration.invites.list': () => ({ invitations: state.collaboration.invitations, nextAfter: null }),
    'collaboration.invites.revoke': (params) => { const row = state.collaboration.invitations.find((v) => v.inviteId === params.id); if (row) row.state = 'revoked'; return { ok: true }; },
    'collaboration.requests.list': () => ({ requests: state.collaboration.requests.filter((v) => v.state === 'pending'), nextAfter: null }),
    'collaboration.requests.approve': (params) => { const row = state.collaboration.requests.find((v) => v.requestId === params.id); if (row) row.state = 'approved'; state.collaboration.status.pendingRequests = 0; return { ok: true }; },
    'collaboration.requests.reject': (params) => { const row = state.collaboration.requests.find((v) => v.requestId === params.id); if (row) row.state = 'rejected'; return { ok: true }; },
    'collaboration.principals.list': () => ({ principals: state.collaboration.principals, nextAfter: null }),
    'collaboration.principals.get': (params) => state.collaboration.principals.find((v) => v.principalId === params.id),
    'collaboration.principals.revoke': (params) => {
      const c = state.collaboration; const row = c.principals.find((v) => v.principalId === params.id); if (row) row.revoked = true;
      for (const session of c.sessions) if (session.principalId === params.id) session.revoked = true;
      return { ok: true };
    },
    'collaboration.grants.set': (params) => {
      const row = state.collaboration.principals.find((v) => v.principalId === params.principalId);
      if (!row || params.expectedRevision !== row.grantRevision) throw { code: 'revision_conflict', message: 'Grant changed' };
      row.grant = params.grant; row.grantRevision = (BigInt(row.grantRevision) + 1n).toString(); return { ok: true };
    },
    'collaboration.sessions.list': () => ({ sessions: state.collaboration.sessions, nextAfter: null }),
    'collaboration.sessions.revoke': (params) => {
      const c = state.collaboration; const row = c.sessions.find((v) => v.sessionId === params.id); if (row) row.revoked = true;
      c.status.gateway.connections -= c.status.gateway.sessionConnections[params.id] || 0;
      delete c.status.gateway.sessionConnections[params.id];
      c.status.activeSessions = c.sessions.filter((v) => !v.revoked).length; return { ok: true };
    },
    'collaboration.audit.list': () => ({ entries: state.collaboration.audit, nextAfter: state.collaboration.audit.at(-1)?.cursor ?? 0 }),
    'automation.snapshot': () => behaviorEnvelope(state.automationSnapshot,
      ['action', 'event'].flatMap((kind) => behaviorRows(kind).map((r) => behaviorMeta(kind, r.id)))),
    'automation.get': (params) => {
      const kind = params.kind || 'event'; const record = behaviorRows(kind).find((r) => r.id === params.id);
      if (!record) throw { code: 'automation_not_found', message: 'Record was deleted', data: { resourceVersions: [behaviorMeta(kind, params.id)] } };
      return behaviorEnvelope(record, [behaviorMeta(kind, params.id)]);
    },
    'automation.create': (params, meta) => writeBehavior('create', params, meta),
    'automation.update': (params, meta) => writeBehavior('update', params, meta),
    'automation.delete': (params, meta) => writeBehavior('delete', params, meta),
    'automation.enable': (params, meta) => writeBehavior('enable', params, meta),
    'automation.disable': (params, meta) => writeBehavior('disable', params, meta),
    'automation.runs': () => ({ runs: state.automationRuns }),
    'ui.projection': (params) => ({ version: 1, generation: 0, revision: params.revision }),
    'points.config.get': () => state.pointsConfig,
    'points.config.set': (params) => {
      state.pointsConfig = { ...state.pointsConfig, ...(params || {}) };
      return state.pointsConfig;
    },
    'points.leaderboard': () => ({ viewers: state.leaderboard }),
    'points.adjust': (params) => {
      const viewer = state.leaderboard.find((v) => v.uniqueId === params.uniqueId);
      if (viewer) viewer.points += params.delta || 0;
      return { ok: true };
    },
    'points.reset': () => {
      for (const viewer of state.leaderboard) viewer.points = 0;
      return { ok: true };
    },
    'creators.get': () => ({ creator: state.creator }),
    'creators.recent': () => ({ creators: state.recentCreators }),
    'gifts.list': () => ({ gifts: state.gifts }),
    'globals.list': () => ({ globals: { ...(state.globals ?? {}) } }),
    'globals.set': (params) => {
      (state.globals ||= {})[params.key] = params.value;
      return { ok: true };
    },
    'processors.status': () => ({ processors: state.processors }),
    'widgets.status': () => ({ state: 'missing', port: 17452, error: null }),
    'app.state.get': () => ({ state: { ...state.appState } }),
    'app.state.set': (params) => {
      state.appState[params.key] = params.value;
      return { ok: true };
    },
    'plugins.packages.receipts': () => ({ receipts: [] }),
    'plugins.jobs.list': () => ({ jobs: [] }),
    'plugins.updates.list': () => ({ updates: [], checkedAt: 0, errors: [] }),
    // Registry sources (Plugins › Explore availability, Settings › Advanced).
    // Feed rankings provision themselves on connect; keep definitions in memory.
    'rankings.list': () => ({ rankings: Object.values(state.rankings ??= {}) }),
    'rankings.create': (params) => {
      (state.rankings ??= {})[params.id] = params;
      return { ranking: params };
    },
    'rankings.update': (params) => {
      (state.rankings ??= {})[params.definition.id] = params.definition;
      return { ranking: params.definition };
    },
    'rankings.snapshot': (params) => ({ id: params.id, entries: [], total: 0, revision: 0 }),
    'registry.list': () => ({ registries: [] }),
    'registry.defaults.get': () => ({ channel: 'stable', autoUpdate: 'notify', checkIntervalHours: 12, allowSideload: true }),
    'app.update.status': () => ({
      currentVersion: '4.1.0', availableVersion: null, channel: 'stable', state: 'idle',
      installationType: 'development', installSupported: false, checkSupported: false,
      downloadedBytes: 0, totalBytes: null, progress: null, releaseNotes: null, lastChecked: null, error: null,
    }),
    'app.update.preferences.get': () => ({ channel: 'stable', automaticChecks: false, notifications: false, lastChecked: 0, lastAttempt: 0 }),
    'plugins.settings.get': (params) => {
      const stored = state.pluginSettings[params.pluginId];
      if (!stored) throw new Error('unknown plugin settings: ' + params.pluginId);
      return { pluginId: params.pluginId, ...stored };
    },
    'plugins.settings.set': (params) => {
      const stored = state.pluginSettings[params.pluginId];
      if (!stored) throw new Error('unknown plugin settings: ' + params.pluginId);
      stored.values = { ...(params.values || {}) };
      return { pluginId: params.pluginId, ...stored };
    },
    'plugins.options': (params) => {
      const stored = state.pluginOptions[params.source];
      if (!stored) throw new Error('unknown option source: ' + params.source);
      return { source: params.source, options: stored.options, selected: stored.selected ?? null };
    },
    'plugins.health': (params) => {
      const stored = state.pluginHealth[params.pluginId];
      if (!stored) return { pluginId: params.pluginId, ok: false, latencyMs: 0, error: 'no fake health' };
      return { pluginId: params.pluginId, ...stored };
    },
    'plugins.action.execute': (params) => {
      const stored = state.actionOutcomes[params.actionType];
      const outcome = !stored
        ? { actionType: params.actionType, ok: true, summary: 'fake ok', logs: [], durationMs: 1 }
        : {
            actionType: params.actionType,
            ok: stored.ok,
            summary: stored.summary,
            logs: stored.logs || [],
            durationMs: 1,
            error: stored.error ?? null,
          };
      if (state.actionDelayMs > 0) {
        return { __e2e_delay_ms: state.actionDelayMs, __e2e_value: outcome };
      }
      return outcome;
    },
    'plugins.token.import': (params) => {
      const id = String(params.pluginId);
      const candidates = state.pluginTokenCandidates?.[id] ?? [];
      if (params.mode === 'auto') return { pluginId: id, ok: false, error: candidates.length ? null : 'No configuration found. Select the game folder.', candidates };
      if (params.mode === 'candidate' && !candidates.some((candidate) => candidate.id === params.candidateId)) return { pluginId: id, ok: false, error: 'Configuration unavailable.' };
      const settings = state.pluginSettings[id];
      if (settings) {
        settings.values.apiToken = '••••••••';
        if (state.pluginSetupEndpoints?.[id]) settings.values.serverUrl = state.pluginSetupEndpoints[id];
      }
      state.pluginHealth[id] = state.pluginSetupHealth?.[id] ?? { ok: true, latencyMs: 1 };
      return { pluginId: id, ok: true, error: null };
    },
    'plugins.token.provision': (params) => ({ pluginId: params.pluginId, ok: true, error: null }),
    'system.health': () => ({ liveIngress: { received: 12, decoded: 12, accepted: 11, processed: 11, effects: { queued: 0, dropped: 0 } }, liveDedup: { duplicates: 1 }, liveTotals: { unknownMethods: {} } }),
    'ui.feed.backfill': (params) => {
      const journal = state.feedJournal || [];
      const selected = journal.filter((entry) => entry.cursor > params.after).slice(0, 300);
      return { oldestCursor: journal[0]?.cursor || 0, latestCursor: journal.at(-1)?.cursor || 0, depth: journal.length, gap: false, truncated: (selected.at(-1)?.cursor || params.after) < (journal.at(-1)?.cursor || 0), events: selected };
    },
    'live.monitor.status': () => state.liveMonitor || { enabled: false, phase: 'stopped', uniqueId: '', pollIntervalSeconds: 30, checks: 0, failures: 0, lastCheckedAt: null, nextCheckAt: null, lastError: null },
    'live.monitor.start': (params) => (state.liveMonitor = { enabled: true, phase: 'waiting', uniqueId: params.uniqueId, pollIntervalSeconds: params.pollIntervalSeconds || 30, checks: 1, failures: 0, lastCheckedAt: Date.now(), nextCheckAt: Date.now() + 30000, lastError: null }),
    'live.monitor.stop': () => (state.liveMonitor = { ...state.liveMonitor, enabled: false, phase: 'stopped' }),
    'live.status': () => state.liveStatus,
    'live.connect': (params) => {
      state.liveStatus = {
        connected: true,
        uniqueId: params.uniqueId || null,
        roomId: params.roomId || null,
        connectionId: 'connection-1',
        native: true,
      };
      return state.liveStatus;
    },
    'live.disconnect': () => {
      state.liveStatus = {
        connected: false,
        uniqueId: null,
        roomId: null,
        connectionId: null,
        native: true,
      };
      return state.liveStatus;
    },
    'live.channels': () => ({
      rooms: state.liveChannels.rooms,
      nextOffset: state.liveChannels.nextOffset,
      fetchedAt: Date.now(),
      source: 'tiktok-live-search',
    }),
  };

  window.ipc = {
    postMessage(raw) {
      let message;
      try {
        message = JSON.parse(raw);
      } catch {
        return;
      }
      if (message && message.type === 'frontend-ready') {
        state.frontendReady = true;
        return;
      }
      // Legacy page messages (plugin UI open/close) are recorded for
      // assertions; the real desktop host acts on them natively.
      if (message && typeof message.type === 'string' && message.jsonrpc === undefined) {
        state.legacyMessages.push(message);
        return;
      }
      if (!message || message.jsonrpc !== '2.0' || message.method === undefined) return;
      calls.push({ method: message.method, params: message.params || {}, ...(message.meta ? { meta: message.meta } : {}) });
      const forced = state.failures[message.method];
      if (forced !== undefined) {
        const forcedCode = typeof forced === 'object' && forced ? forced.code : 'e2e';
        const forcedMessage = typeof forced === 'object' && forced ? forced.message : forced;
        respond(message.id, undefined, {
          code: forcedCode,
          message: forcedMessage || ('fake host failure: ' + message.method),
        });
        return;
      }
      const handler = handlers[message.method];
      if (!handler) {
        state.unhandled.push(message.method);
        fail(message.id, message.method, 'Unhandled TikTools RPC in Playwright host: ' + message.method);
        return;
      }
      try {
        const result = handler(message.params || {}, message.meta);
        if (result && result.__e2e_meta) {
          const packet = JSON.parse(JSON.stringify(result));
          const delay = ['automation.create', 'automation.update', 'automation.delete', 'automation.enable', 'automation.disable'].includes(message.method) ? state.automationDelayMs || 0 : 0;
          if (delay) setTimeout(() => respond(message.id, packet.__e2e_value, undefined, packet.__e2e_meta), delay);
          else respond(message.id, packet.__e2e_value, undefined, packet.__e2e_meta);
        } else if (result && typeof result.__e2e_delay_ms === 'number') {
          setTimeout(() => respond(message.id, result.__e2e_value), result.__e2e_delay_ms);
        } else {
          respond(message.id, result);
        }
      } catch (error) {
        respond(message.id, undefined, { code: error && typeof error.code === 'string' ? error.code : 'e2e', message: error && error.message ? error.message : String(error), ...(error && error.data ? { data: error.data } : {}) });
      }
    },
  };
})();
`;

export interface ConsoleCapture {
  errors: string[];
  assertClean(): void;
}

/** Installs the fake host and returns console/pageerror capture. */
export async function installFakeHost(
  page: Page,
  state: E2EHostState,
  options: { locale?: string; theme?: string } = {},
): Promise<ConsoleCapture> {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => {
    errors.push(error instanceof Error ? error.message : String(error));
  });
  await page.addInitScript(
    ({ initial, locale, theme }) => {
      (window as unknown as { __TIKTOOLS_E2E_STATE__: unknown }).__TIKTOOLS_E2E_STATE__ = initial;
      try {
        if (locale) localStorage.setItem('tiktok-live-locale', locale);
        if (theme) localStorage.setItem('tiktok-live-theme', theme);
      } catch {
        // Storage may be unavailable; the app tolerates that.
      }
    },
    { initial: state, locale: options.locale, theme: options.theme },
  );
  await page.addInitScript(INIT_SCRIPT);
  return {
    errors,
    assertClean(): void {
      if (errors.length > 0) {
        throw new Error(`Unexpected browser console/page errors:\n${errors.join('\n')}`);
      }
    },
  };
}

/** Reads the current fake-host state from the page. */
export async function readHostState(page: Page): Promise<E2EHostState> {
  return page.evaluate(() => {
    const w = window as unknown as { __TIKTOOLS_E2E_STATE__: E2EHostState };
    return w.__TIKTOOLS_E2E_STATE__;
  });
}

/**
 * Writes back fake-host state mutated in Node (drivers mid-test changes).
 * Read with `readHostState`, mutate the plain object, write it back —
 * no code crosses the boundary, only JSON-serializable data. The write
 * mutates the live object in place (preserving `calls`/`unhandled` array
 * identity) because the dispatcher closes over them.
 */
export async function writeHostState(page: Page, state: E2EHostState): Promise<void> {
  await page.evaluate((next) => {
    const w = window as unknown as { __TIKTOOLS_E2E_STATE__: E2EHostState };
    const current = w.__TIKTOOLS_E2E_STATE__;
    const { calls, unhandled, ...rest } = next;
    Object.assign(current, rest);
    current.calls.length = 0;
    current.calls.push(...calls);
    current.unhandled.length = 0;
    current.unhandled.push(...unhandled);
  }, state);
}

/** Fails when the app called RPC methods the fake host does not implement. */
export async function assertNoUnhandledCalls(page: Page): Promise<void> {
  const state = await readHostState(page);
  if (state.unhandled.length > 0) {
    throw new Error(`Unhandled TikTools RPC calls in Playwright host: ${state.unhandled.join(', ')}`);
  }
}

/** Counts calls to one RPC method (persistence coalescing assertions). */
export async function countCalls(page: Page, method: string): Promise<number> {
  const state = await readHostState(page);
  return state.calls.filter((call) => call.method === method).length;
}

/** Emits a domain-event topic notification as if the host pushed it. */
export async function emitTopic(page: Page, topic: string, data: unknown = {}): Promise<void> {
  await page.evaluate(
    ({ topic, data }) => {
      window.__webview_on_message__?.(JSON.stringify({ method: 'event', params: { topic, data } }));
    },
    { topic, data },
  );
}

/** Emits a legacy host push message as if the host sent it. */
export async function emitPush(page: Page, message: Record<string, unknown>): Promise<void> {
  await page.evaluate((message) => {
    window.__webview_on_message__?.(JSON.stringify(message));
  }, message);
}

/** Lazy route modules can finish mounting after the document load event. */
export async function waitForFrontend(page: Page): Promise<void> {
  await page.waitForFunction(() => {
    const scope = window as unknown as { __TIKTOOLS_E2E_STATE__: E2EHostState };
    return scope.__TIKTOOLS_E2E_STATE__.frontendReady;
  });
}
