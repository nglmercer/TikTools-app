/**
 * Restricted broker client for the SonicBoom plugin UI.
 *
 * The plugin UI NEVER receives the host's privileged `window.ipc` bridge.
 * It talks to the host through `window.tiktools` only:
 *
 * - Inside the desktop child WebView, the host injects `window.tiktools`
 *   natively (the initialization script captures and deletes `window.ipc`
 *   before page scripts run). Plugin identity is captured by the native
 *   host — JavaScript never supplies a `pluginId`.
 * - Inside a development/CI iframe (opaque origin via `sandbox`), the same
 *   calls travel over `postMessage` using the versioned envelope below,
 *   and the parent shim fills in the plugin identity it mounted.
 *
 * Allowed operations (anything else fails closed):
 * `settings.get`, `settings.set`, `actions.execute`, `options.get`,
 * `events.subscribe`, `events.unsubscribe`, `host.locale`, `host.theme`.
 *
 * This protocol is mirrored by the host shim
 * (`src/web/plugin-ui/plugin-webview-host.ts`) and the desktop broker
 * (`crates/tiktools-desktop/src/plugin_webview/broker.rs`). The three
 * sides are pinned by interop tests, not by shared imports: the plugin
 * package must stay buildable on its own.
 */

import type { ActionOptionItem } from './types.ts';

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

export interface BrokerRequest {
  apiVersion: number;
  id: string;
  method: BrokerMethod;
  params: Record<string, unknown>;
}

export type BrokerResponse =
  | { apiVersion: number; id: string; ok: true; result: unknown }
  | { apiVersion: number; id: string; ok: false; error: string };

export interface BrokerEvent {
  apiVersion: number;
  event: string;
  data: unknown;
}

export interface ActionOutcome {
  actionType: string;
  ok: boolean;
  summary: string;
  logs: string[];
  durationMs: number;
  error?: string | null;
}

export interface OptionsResult {
  options: ActionOptionItem[];
  selected: string | null;
}

export type Unsubscribe = () => void;

/** Native `window.tiktools` surface injected by the desktop host. */
export interface NativeTikTools {
  settings: {
    get(): Promise<Record<string, unknown>>;
    set(values: Record<string, unknown>): Promise<Record<string, unknown>>;
  };
  actions: {
    execute(action: string, config: Record<string, unknown>): Promise<ActionOutcome>;
  };
  options: {
    get(source: string, refresh?: boolean): Promise<OptionsResult>;
  };
  events: {
    subscribe(topics: string[], listener: (event: string, data: unknown) => void): Unsubscribe;
  };
  host: {
    locale(): Promise<string>;
    theme(): Promise<string>;
  };
}

function nativeApi(): NativeTikTools | undefined {
  const candidate = (window as unknown as { tiktools?: NativeTikTools }).tiktools;
  if (!candidate || typeof candidate !== 'object') return undefined;
  // Fail closed on a partial injection: every namespace must exist.
  if (
    !candidate.settings ||
    !candidate.actions ||
    !candidate.options ||
    !candidate.events ||
    !candidate.host
  ) {
    return undefined;
  }
  return candidate;
}

/**
 * Broker client used by the plugin UI. Prefers the native injection and
 * falls back to the versioned `postMessage` transport (development
 * iframe). Responses are correlated by request id; unknown envelopes and
 * version mismatches are dropped.
 */
export class PluginBroker {
  private readonly native: NativeTikTools | undefined;
  private sequence = 0;
  private readonly pending = new Map<
    string,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();
  private readonly listeners = new Map<string, Set<(data: unknown) => void>>();
  private readonly onMessage = (event: MessageEvent): void => {
    this.handleMessage(event.data);
  };

  constructor() {
    this.native = nativeApi();
    if (!this.native) window.addEventListener('message', this.onMessage);
  }

  dispose(): void {
    window.removeEventListener('message', this.onMessage);
    for (const [, entry] of this.pending) entry.reject(new Error('broker disposed'));
    this.pending.clear();
    this.listeners.clear();
  }

  usesNativeTransport(): boolean {
    return this.native !== undefined;
  }

  private handleMessage(data: unknown): void {
    if (!data || typeof data !== 'object' || Array.isArray(data)) return;
    const envelope = data as Record<string, unknown>;
    if (envelope.apiVersion !== BROKER_API_VERSION) return;
    // Pushed event from the host.
    if (typeof envelope.event === 'string') {
      const watchers = this.listeners.get(envelope.event);
      if (watchers) {
        for (const listener of [...watchers]) listener(envelope.data);
      }
      return;
    }
    // Request/response correlation.
    if (typeof envelope.id !== 'string') return;
    const entry = this.pending.get(envelope.id);
    if (!entry) return;
    this.pending.delete(envelope.id);
    if (envelope.ok === true) entry.resolve(envelope.result);
    else entry.reject(new Error(typeof envelope.error === 'string' ? envelope.error : 'broker error'));
  }

  private post(method: BrokerMethod, params: Record<string, unknown>): Promise<unknown> {
    if (this.native) return this.callNative(method, params);
    this.sequence += 1;
    const id = `broker-${this.sequence}`;
    const request: BrokerRequest = { apiVersion: BROKER_API_VERSION, id, method, params };
    return new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      // Vue reactivity wraps state in Proxies, which structuredClone
      // rejects: send a plain-JSON copy across the frame boundary.
      let wire: BrokerRequest;
      try {
        wire = JSON.parse(JSON.stringify(request)) as BrokerRequest;
      } catch {
        this.pending.delete(id);
        reject(new Error('broker request is not serializable'));
        return;
      }
      window.parent.postMessage(wire, '*');
    });
  }

  private callNative(method: BrokerMethod, params: Record<string, unknown>): Promise<unknown> {
    const native = this.native;
    if (!native) return Promise.reject(new Error('no native broker'));
    switch (method) {
      case 'settings.get':
        return native.settings.get() as Promise<unknown>;
      case 'settings.set':
        return native.settings.set((params.values ?? {}) as Record<string, unknown>) as Promise<unknown>;
      case 'actions.execute':
        return native.actions.execute(
          String(params.action ?? ''),
          (params.config ?? {}) as Record<string, unknown>,
        ) as Promise<unknown>;
      case 'options.get':
        return native.options.get(
          String(params.source ?? ''),
          params.refresh === true,
        ) as Promise<unknown>;
      case 'host.locale':
        return native.host.locale() as Promise<unknown>;
      case 'host.theme':
        return native.host.theme() as Promise<unknown>;
      case 'events.subscribe':
      case 'events.unsubscribe':
        return Promise.reject(new Error(`${method} uses subscribe(), not post()`));
    }
  }

  async getSettings(): Promise<Record<string, unknown>> {
    return (await this.post('settings.get', {})) as Record<string, unknown>;
  }

  async setSettings(values: Record<string, unknown>): Promise<Record<string, unknown>> {
    return (await this.post('settings.set', { values })) as Record<string, unknown>;
  }

  async executeAction(action: string, config: Record<string, unknown>): Promise<ActionOutcome> {
    return (await this.post('actions.execute', { action, config })) as ActionOutcome;
  }

  async getOptions(source: string, refresh = false): Promise<OptionsResult> {
    const result = (await this.post('options.get', { source, refresh })) as Partial<OptionsResult>;
    return {
      options: Array.isArray(result.options) ? result.options : [],
      selected: typeof result.selected === 'string' ? result.selected : null,
    };
  }

  async getLocale(): Promise<string> {
    return String(await this.post('host.locale', {}));
  }

  async getTheme(): Promise<string> {
    return String(await this.post('host.theme', {}));
  }

  /**
   * Subscribes to host-pushed plugin events (speech state, backend logs).
   * The native transport registers directly; the postMessage transport
   * notifies the parent and routes pushed events to local listeners.
   */
  subscribe(event: string, listener: (data: unknown) => void): Unsubscribe {
    if (this.native) {
      return this.native.events.subscribe([event], (name, data) => {
        if (name === event) listener(data);
      });
    }
    let watchers = this.listeners.get(event);
    if (!watchers) {
      watchers = new Set();
      this.listeners.set(event, watchers);
      void this.post('events.subscribe', { topics: [event] }).catch(() => undefined);
    }
    watchers.add(listener);
    return () => {
      const current = this.listeners.get(event);
      if (!current) return;
      current.delete(listener);
      if (current.size === 0) {
        this.listeners.delete(event);
        void this.post('events.unsubscribe', { topics: [event] }).catch(() => undefined);
      }
    };
  }
}
