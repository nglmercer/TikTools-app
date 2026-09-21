/**
 * Web host side of the isolated plugin UI broker.
 *
 * This is the counterpart of the plugin-side client
 * (`plugins/sonicboom/ui/src/broker.ts`) and the native desktop broker
 * (`crates/tiktools-desktop/src/plugin_webview/broker.rs`). The three sides
 * share the versioned envelope `{apiVersion: 1, id, ...}` by convention,
 * pinned by interop tests — never by shared imports, since the plugin
 * package must stay buildable on its own.
 *
 * Two hosts exist because two runtimes exist:
 *
 * - On desktop, plugin pages open in native windows (`openPluginUi` asks
 *   the host to open one). The page talks to the Rust broker through the
 *   injected `window.tiktools` surface; this module is not involved.
 * - On web (development, CI, the Playwright UI suite), the plugin page
 *   loads in a sandboxed iframe (`PluginFrame`) and talks `postMessage`
 *   to `PluginWebviewHost` below, which answers through a `BrokerBackend`.
 *
 * Ownership: the host binds one plugin id at mount and injects it into
 * every downstream call. Frame-supplied `pluginId` values are rejected
 * when they disagree, so a plugin page can only read its own
 * settings/options and execute its own actions.
 */

import type { ControlClient } from '../platform/control-client.ts';

export const BROKER_API_VERSION = 1;

export type BrokerMethod =
  | 'settings.get'
  | 'settings.set'
  | 'actions.execute'
  | 'options.get'
  | 'events.subscribe'
  | 'events.unsubscribe'
  | 'host.locale'
  | 'host.theme';

const BROKER_METHODS: readonly string[] = [
  'settings.get',
  'settings.set',
  'actions.execute',
  'options.get',
  'events.subscribe',
  'events.unsubscribe',
  'host.locale',
  'host.theme',
];

export interface BrokerRequest {
  apiVersion: number;
  id: string;
  method: BrokerMethod;
  params: Record<string, unknown>;
}

export interface BrokerOkResponse {
  apiVersion: number;
  id: string;
  ok: true;
  result: unknown;
}

export interface BrokerErrorResponse {
  apiVersion: number;
  id: string;
  ok: false;
  error: string;
}

export type BrokerResponse = BrokerOkResponse | BrokerErrorResponse;

export interface BrokerEvent {
  apiVersion: number;
  event: string;
  data: unknown;
}

/** Downstream operations the host performs for one bound plugin. */
export interface BrokerBackend {
  getSettings(): Promise<Record<string, unknown>>;
  setSettings(values: Record<string, unknown>): Promise<Record<string, unknown>>;
  executeAction(action: string, config: Record<string, unknown>): Promise<unknown>;
  getOptions(source: string, refresh: boolean): Promise<{ options: unknown[]; selected: string | null }>;
  getLocale(): Promise<string>;
  getTheme(): Promise<string>;
}

/**
 * Identifier rule for plugin and page ids in frame URLs. Mirrors the
 * desktop `is_valid_ui_id` (the layer that consumes these URLs): a
 * letter/underscore start, ASCII alphanumerics plus `.`/`_`/`-`, at most
 * 128 characters. Manifest-valid ids are a subset, so every installed
 * plugin passes; anything shaped like traversal or injection fails here
 * before a URL is ever built.
 */
const UI_ID_PATTERN = /^[A-Za-z_][A-Za-z0-9._-]{0,127}$/;

/** Basename rule for webview entry documents: HTML only, no directories. */
const UI_ENTRY_PATTERN = /^[A-Za-z0-9_-][A-Za-z0-9._-]*\.html$/;

/**
 * Document URL for one inline plugin frame, or null when the current host
 * cannot serve plugin assets (plain browser without the desktop shell).
 *
 * On desktop the scheme is served by the main window's shared asset
 * server; only the entry file name travels (the server confines every
 * request to the entry's own directory), and the page id rides in the
 * fragment for the plugin router.
 *
 * Tests point `window.__TIKTOOLS_PLUGIN_UI_BASE__` at a fixture route;
 * production never sets it.
 */
export function pluginUiUrl(pluginId: string, entry: string, pageId: string): string | null {
  if (!UI_ID_PATTERN.test(pluginId)) return null;
  if (entry.includes('\\') || entry.includes('\0')) return null;
  const segments = entry.split('/');
  if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) {
    return null;
  }
  const file = segments[segments.length - 1] ?? '';
  if (file.length > 128 || !UI_ENTRY_PATTERN.test(file)) return null;
  const page = pageId.trim();
  if (!UI_ID_PATTERN.test(page)) return null;
  const scope =
    typeof window === 'undefined' ? undefined : (window as unknown as Record<string, unknown>);
  const override = scope?.['__TIKTOOLS_PLUGIN_UI_BASE__'];
  if (typeof override === 'string' && override.length > 0) {
    const base = override.endsWith('/') ? override : `${override}/`;
    return `${base}${pluginId}/${file}#page=${encodeURIComponent(page)}`;
  }
  if (!isDesktopHost()) return null;
  return `tiktools-plugin://app/${pluginId}/${file}#page=${encodeURIComponent(page)}`;
}

/** True inside the desktop WebView, where native plugin windows exist. */
export function isDesktopHost(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.ipc?.postMessage === 'function'
  );
}

/**
 * Asks the desktop host to open a plugin page in an isolated native
 * window. No-op (false) outside the desktop WebView.
 */
export function openPluginUi(pluginId: string, pageId: string): boolean {
  if (!isDesktopHost()) return false;
  window.ipc?.postMessage(JSON.stringify({ type: 'plugin-ui-open', pluginId, pageId }));
  return true;
}

/** Asks the desktop host to close an isolated plugin window. */
export function closePluginUi(pluginId: string, pageId: string): boolean {
  if (!isDesktopHost()) return false;
  window.ipc?.postMessage(JSON.stringify({ type: 'plugin-ui-close', pluginId, pageId }));
  return true;
}

/** Builds a backend from a JSON-RPC control client, scoped to one plugin. */
export function controlBackend(
  pluginId: string,
  control: ControlClient,
  locale: () => string,
  theme: () => string,
): BrokerBackend {
  return {
    async getSettings(): Promise<Record<string, unknown>> {
      const result = await control.call<{ values?: Record<string, unknown> }>(
        'plugins.settings.get',
        { pluginId },
      );
      return result.values ?? {};
    },
    async setSettings(values: Record<string, unknown>): Promise<Record<string, unknown>> {
      const result = await control.call<{ values?: Record<string, unknown> }>(
        'plugins.settings.set',
        { pluginId, values },
      );
      return result.values ?? {};
    },
    async executeAction(action: string, config: Record<string, unknown>): Promise<unknown> {
      // Broker execution is always live: the plugin UI's execute buttons
      // (voice tests, output switches) must take real effect. Ownership
      // scoping still confines the call to the mounted plugin.
      return control.call('plugins.action.execute', {
        actionType: action,
        config,
        live: true,
        pluginId,
      });
    },
    async getOptions(
      source: string,
      refresh: boolean,
    ): Promise<{ options: unknown[]; selected: string | null }> {
      const result = await control.call<{ options?: unknown[]; selected?: string | null }>(
        'plugins.options',
        { source, refresh, pluginId },
      );
      return {
        options: Array.isArray(result.options) ? result.options : [],
        selected: typeof result.selected === 'string' ? result.selected : null,
      };
    },
    async getLocale(): Promise<string> {
      return locale();
    },
    async getTheme(): Promise<string> {
      return theme();
    },
  };
}

export interface PluginWebviewHostOptions {
  pluginId: string;
  backend: BrokerBackend;
  /** Posts an envelope to the mounted frame. */
  postToFrame: (envelope: BrokerResponse | BrokerEvent) => void;
  /** Subscribes to host event topics; returns an unsubscribe function. */
  subscribeTopic?: (topic: string, listener: (data: unknown) => void) => () => void;
  onError?: (message: string) => void;
}

const MAX_ID_LENGTH = 128;
const MAX_TOPIC_LENGTH = 128;
const MAX_ACTION_LENGTH = 256;
const MAX_SOURCE_LENGTH = 512;

/**
 * Answers broker requests from one sandboxed plugin frame. The frame
 * element is owned by the caller (`PluginFrame`); this class only needs a
 * way to post back and the frame's `contentWindow` to authenticate
 * inbound messages by source.
 */
export class PluginWebviewHost {
  private readonly options: PluginWebviewHostOptions;
  private readonly frameWindow: WindowProxy | null;
  private readonly topics = new Map<string, () => void>();

  constructor(frameWindow: WindowProxy | null, options: PluginWebviewHostOptions) {
    this.frameWindow = frameWindow;
    this.options = options;
  }

  dispose(): void {
    for (const unsubscribe of this.topics.values()) unsubscribe();
    this.topics.clear();
  }

  /** Pushes one host event to the frame when it subscribed to the topic. */
  pushEvent(event: string, data: unknown): void {
    if (!this.topics.has(event)) return;
    this.options.postToFrame({ apiVersion: BROKER_API_VERSION, event, data });
  }

  /**
   * Handles one inbound `message` event payload. Unknown sources,
   * versions, shapes, and methods are dropped or answered with an error;
   * nothing unrecognized ever reaches the backend.
   */
  async handleFrameMessage(data: unknown, source: MessageEventSource | null): Promise<void> {
    if (source !== this.frameWindow) return;
    if (!data || typeof data !== 'object' || Array.isArray(data)) return;
    const envelope = data as Record<string, unknown>;
    if (envelope.apiVersion !== BROKER_API_VERSION) return;
    const id =
      typeof envelope.id === 'string' && envelope.id.length > 0 && envelope.id.length <= MAX_ID_LENGTH
        ? envelope.id
        : null;
    if (!id) return;
    const method = envelope.method;
    if (typeof method !== 'string' || !BROKER_METHODS.includes(method)) {
      this.respond({ apiVersion: BROKER_API_VERSION, id, ok: false, error: `unknown broker method \`${String(method)}\`` });
      return;
    }
    const params = envelope.params;
    if (params !== undefined && (typeof params !== 'object' || params === null || Array.isArray(params))) {
      this.respond({ apiVersion: BROKER_API_VERSION, id, ok: false, error: 'broker params must be an object' });
      return;
    }
    const claimed = (params as Record<string, unknown> | undefined)?.pluginId;
    if (claimed !== undefined && claimed !== this.options.pluginId) {
      this.respond({
        apiVersion: BROKER_API_VERSION,
        id,
        ok: false,
        error: 'pluginId does not match the owning plugin',
      });
      return;
    }
    try {
      const result = await this.route(method as BrokerMethod, (params ?? {}) as Record<string, unknown>);
      if (result.effect === 'subscribed' || result.effect === 'unsubscribed') {
        this.applySubscription(result.topics, result.effect === 'subscribed');
      }
      this.respond({ apiVersion: BROKER_API_VERSION, id, ok: true, result: result.value });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.options.onError?.(message);
      this.respond({ apiVersion: BROKER_API_VERSION, id, ok: false, error: message });
    }
  }

  private respond(response: BrokerResponse): void {
    this.options.postToFrame(response);
  }

  private applySubscription(topics: string[], subscribe: boolean): void {
    for (const topic of topics) {
      if (subscribe) {
        if (this.topics.has(topic)) continue;
        const subscribeTopic = this.options.subscribeTopic;
        this.topics.set(
          topic,
          subscribeTopic ? subscribeTopic(topic, (data) => this.pushEvent(topic, data)) : () => undefined,
        );
      } else {
        this.topics.get(topic)?.();
        this.topics.delete(topic);
      }
    }
  }

  private async route(
    method: BrokerMethod,
    params: Record<string, unknown>,
  ): Promise<{ value: unknown; effect?: 'subscribed' | 'unsubscribed'; topics: string[] }> {
    const backend = this.options.backend;
    switch (method) {
      case 'settings.get':
        return { value: await backend.getSettings(), topics: [] };
      case 'settings.set': {
        const values = params.values;
        if (!values || typeof values !== 'object' || Array.isArray(values)) {
          throw new Error('settings.set needs a values object');
        }
        return { value: await backend.setSettings(values as Record<string, unknown>), topics: [] };
      }
      case 'actions.execute': {
        const action = params.action;
        if (typeof action !== 'string' || action.length === 0 || action.length > MAX_ACTION_LENGTH) {
          throw new Error('actions.execute needs an action name');
        }
        const config = params.config;
        if (config !== undefined && (typeof config !== 'object' || config === null || Array.isArray(config))) {
          throw new Error('actions.execute needs a config object');
        }
        return {
          value: await backend.executeAction(action, (config ?? {}) as Record<string, unknown>),
          topics: [],
        };
      }
      case 'options.get': {
        const source = params.source;
        if (typeof source !== 'string' || source.length === 0 || source.length > MAX_SOURCE_LENGTH) {
          throw new Error('options.get needs an option source');
        }
        return {
          value: await backend.getOptions(source, params.refresh === true),
          topics: [],
        };
      }
      case 'events.subscribe':
      case 'events.unsubscribe': {
        const topics = params.topics;
        if (!Array.isArray(topics) || topics.length === 0 || topics.length > 32) {
          throw new Error('events subscription needs 1..=32 topics');
        }
        for (const topic of topics) {
          if (typeof topic !== 'string' || topic.length === 0 || topic.length > MAX_TOPIC_LENGTH || !topic.startsWith('plugin.')) {
            throw new Error('event topics are plugin.* names');
          }
        }
        return {
          value: true,
          effect: method === 'events.subscribe' ? 'subscribed' : 'unsubscribed',
          topics: topics as string[],
        };
      }
      case 'host.locale':
        return { value: await backend.getLocale(), topics: [] };
      case 'host.theme':
        return { value: await backend.getTheme(), topics: [] };
    }
  }
}
