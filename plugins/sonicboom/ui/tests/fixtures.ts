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
  outputs: Array<{ value: string; label: string }>;
  outputsSelected: string | null;
  speakOutcome: { ok: boolean; summary: string; logs: string[]; error?: string | null };
  speakDelayMs: number;
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
    voices: [
      { value: 'M1', label: 'M1' },
      { value: 'F2', label: 'F2' },
      { value: 'N3', label: 'N3' },
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
        return { ...state.settings };
      },
    },
    actions: {
      execute: async (action, config) => {
        calls.push({ method: 'actions.execute', params: { action, config } });
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
        if (source.endsWith(':voice')) {
          return { options: [...state.voices], selected: null };
        }
        return { options: [...state.outputs], selected: state.outputsSelected };
      },
    },
    events: {
      subscribe: () => () => undefined,
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
