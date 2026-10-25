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

export interface E2EHostState {
  /** Behavior snapshot served for `automation.snapshot`. */
  automationSnapshot: Record<string, unknown>;
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
  calls: Array<{ method: string; params: Record<string, unknown> }>;
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

  const respond = (id, result, error) => {
    const response = { jsonrpc: '2.0', id };
    if (error !== undefined) response.error = error;
    else response.result = result;
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

  const handlers = {
    'automation.snapshot': () => state.automationSnapshot,
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
    'globals.list': () => ({ globals: {} }),
    'processors.status': () => ({ processors: state.processors }),
    'widgets.status': () => ({ state: 'missing', port: 17452, error: null }),
    'app.state.get': () => ({ state: { ...state.appState } }),
    'app.state.set': (params) => {
      state.appState[params.key] = params.value;
      return { ok: true };
    },
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
      calls.push({ method: message.method, params: message.params || {} });
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
        const result = handler(message.params || {});
        if (result && typeof result.__e2e_delay_ms === 'number') {
          setTimeout(() => respond(message.id, result.__e2e_value), result.__e2e_delay_ms);
        } else {
          respond(message.id, result);
        }
      } catch (error) {
        fail(message.id, message.method, error && error.message ? error.message : String(error));
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
