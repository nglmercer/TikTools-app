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
import type { VariableComparison } from '../../../src/web/features/edit-runtime-globals/model/variable-sync-adapter.ts';
import type { RuntimeVariable } from '../../../src/web/entities/runtime-global/lib/runtime-variable.ts';

export interface E2EHostState {
  /** Methods announced through `rpc.discover`; none by default, so optional Cloud areas stay hidden. */
  announcedMethods?: string[];
  /** Cloud account and v2 workspace served to the sync panel (`cloud.status`, `sync.status`, `sync.v2.*`). */
  sync?: {
    signedIn: boolean;
    enabled: boolean;
    lastSyncedAt?: number | null;
    entries: Array<Record<string, unknown>>;
    resolved: string[];
  };
  /** Cloud catalog served by `catalog.items` / `catalog.mine` / `catalog.item` (wire items). */
  catalog?: {
    items: Array<Record<string, unknown>>;
    mine?: Array<Record<string, unknown>>;
    library?: Array<Record<string, unknown>>;
    dataState?: 'ready' | 'noActiveGeneration';
    error?: { code: string; message: string; data?: unknown };
  };
  /** Runtime globals served by `globals.list` and written by `globals.set`. */
  globals?: Record<string, string>;
  typedGlobals?: Record<string, RuntimeVariable>;
  typedGlobalsRevision?: string;
  variableComparisons?: VariableComparison[];
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
  const catalogPage = (items, params) => {
    if (state.catalog?.error) throw state.catalog.error;
    const query = String(params.query || '').toLowerCase();
    const rows = (items || []).filter((item) => (!params.kind || item.identity.kind === params.kind)
      && (!query || String(item.presentation?.title || '').toLowerCase().includes(query)));
    return { items: rows, nextCursor: null, ...(state.catalog?.dataState ? { dataState: state.catalog.dataState } : {}) };
  };

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
    'system.info': () => ({ name: 'tiktools', features: { persistence: false } }),

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
