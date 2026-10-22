/**
 * Fake native broker for the standalone SonicBoom UI suite.
 *
 * Installed via `page.addInitScript()` before boot, at the exact boundary
 * the desktop host owns: a `window.tiktools` object with the native
 * namespaces. Calls are recorded on `window.__SONICBOOM_CALLS__` and
 * behavior is driven by `window.__SONICBOOM_STATE__`, both readable from
 * the spec through the helpers below.
 */

import type { Page } from '@playwright/test';

export interface SonicBoomFakeState {
  settings: Record<string, unknown>;
  voices: Array<{ value: string; label: string }>;
  providers: Array<{ value: string; label: string; disabled?: boolean }>;
  catalogError?: string;
  catalogDelayMs?: number;
  outputs: Array<{ value: string; label: string }>;
  outputsSelected: string | null;
  speakOutcome: { ok: boolean; summary: string; logs: string[]; error?: string | null };
  speakDelayMs: number;
  playback?: { currentId: string | null; queueLength: number; playing: boolean; paused: boolean; durationSecs?: number | null; error?: string };
  locale: string;
  theme: string;
}

export interface SonicBoomCall {
  method: string;
  params: Record<string, unknown>;
}

export function baseFakeState(): SonicBoomFakeState {
  return {
    settings: {
      serverUrl: 'http://127.0.0.1:17842',
      apiToken: '',
      tts: { enabled: true, language: 'en', defaultVoice: 'M1' },
    },
    providers: [{ value: 'supertonic', label: 'Supertonic' }],
    voices: [
      { value: 'supertonic:M1', label: 'M1' },
      { value: 'supertonic:F2', label: 'F2' },
      { value: 'supertonic:N3', label: 'N3' },
    ],
    outputs: [
      { value: 'Speakers', label: 'Speakers' },
      { value: 'Headphones', label: 'Headphones' },
    ],
    outputsSelected: 'Speakers',
    speakOutcome: { ok: true, summary: 'spoken', logs: ['POST /api/tts/play 200'] },
    speakDelayMs: 0,
    locale: 'en',
    theme: 'dark',
  };
}

const INIT_SCRIPT = `
(() => {
  const state = window.__SONICBOOM_STATE__;
  const calls = window.__SONICBOOM_CALLS__;
  const settingsListeners = new Set();
  const diagnosticListeners = new Set();
  window.__SONICBOOM_EMIT_DIAGNOSTIC__ = (data) => { for (const listener of diagnosticListeners) listener('plugin.event', data); };
  window.__SONICBOOM_EMIT_SETTINGS__ = () => { for (const listener of settingsListeners) listener('plugin.settings-changed', { pluginId: 'sonicboom.server' }); };
  const delay = (ms, value) => new Promise((resolve) => setTimeout(() => resolve(value), ms));
  window.tiktools = {
    settings: {
      get: async () => {
        calls.push({ method: 'settings.get', params: {} });
        return { ...state.settings };
      },
      set: async (values) => {
        calls.push({ method: 'settings.set', params: { values } });
        state.settings = { ...values };
        window.__SONICBOOM_EMIT_SETTINGS__();
        return { ...state.settings };
      },
    },
    actions: {
      execute: async (action, config) => {
        calls.push({ method: 'actions.execute', params: { action, config } });
        if (action === 'sonicboom.server.playback-status') {
          const playback = state.playback || { currentId: null, queueLength: 0, playing: false, paused: false };
          return { actionType: action, ok: true, summary: JSON.stringify({ ...playback, state: playback.error ? 'error' : 'ready', observedAt: Date.now() }), logs: [], durationMs: 1 };
        }
        if (state.playback && state.speakOutcome.ok) {
          if (action === 'sonicboom.server.skip') { state.playback.currentId = 'next-track'; state.playback.queueLength = Math.max(0, state.playback.queueLength - 1); }
          if (action === 'sonicboom.server.pause') state.playback.paused = true;
          if (action === 'sonicboom.server.resume') state.playback.paused = false;
          if (action === 'sonicboom.server.stop') { state.playback.currentId = null; state.playback.queueLength = 0; state.playback.playing = false; }
        }
        const outcome = {
          actionType: action,
          ok: state.speakOutcome.ok,
          summary: state.speakOutcome.summary,
          logs: [...state.speakOutcome.logs],
          durationMs: 1,
          error: state.speakOutcome.error ?? null,
        };
        if (state.speakDelayMs > 0) return delay(state.speakDelayMs, outcome);
        return outcome;
      },
    },
    options: {
      get: async (source, refresh) => {
        calls.push({ method: 'options.get', params: { source, refresh: refresh === true } });
        if (source.endsWith(':provider') || source.endsWith(':voice')) {
          const result = state.catalogError ? { error: state.catalogError } : { options: [...(source.endsWith(':provider') ? state.providers : state.voices)], selected: null };
          return state.catalogDelayMs ? delay(state.catalogDelayMs, result) : result;
        }
        if (false) {
          return { options: [...state.voices], selected: null };
        }
        return { options: [...state.outputs], selected: state.outputsSelected };
      },
    },
    events: {
      subscribe: (topics, listener) => {
        if (topics.includes('plugin.settings-changed')) settingsListeners.add(listener);
        if (topics.includes('plugin.event')) diagnosticListeners.add(listener);
        return () => { settingsListeners.delete(listener); diagnosticListeners.delete(listener); };
      },
    },
    host: {
      locale: async () => state.locale,
      theme: async () => state.theme,
    },
  };
})();
`;

export interface ConsoleCapture {
  errors: string[];
  assertClean(): void;
}

export async function installFakeBroker(page: Page, state: SonicBoomFakeState): Promise<ConsoleCapture> {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => {
    errors.push(error instanceof Error ? error.message : String(error));
  });
  await page.addInitScript(
    ({ initial }: { initial: SonicBoomFakeState }) => {
      const w = window as unknown as {
        __SONICBOOM_STATE__: SonicBoomFakeState;
        __SONICBOOM_CALLS__: SonicBoomCall[];
      };
      w.__SONICBOOM_STATE__ = initial;
      w.__SONICBOOM_CALLS__ = [];
    },
    { initial: state },
  );
  await page.addInitScript(INIT_SCRIPT);
  return {
    errors,
    assertClean(): void {
      if (errors.length > 0) {
        throw new Error(`Unexpected UI console/page errors:\n${errors.join('\n')}`);
      }
    },
  };
}

export async function readBrokerCalls(page: Page): Promise<SonicBoomCall[]> {
  return page.evaluate(() => {
    const w = window as unknown as { __SONICBOOM_CALLS__: SonicBoomCall[] };
    return w.__SONICBOOM_CALLS__;
  });
}

export async function readBrokerState(page: Page): Promise<SonicBoomFakeState> {
  return page.evaluate(() => {
    const w = window as unknown as { __SONICBOOM_STATE__: SonicBoomFakeState };
    return w.__SONICBOOM_STATE__;
  });
}
