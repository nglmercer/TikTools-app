/**
 * Deterministic frontend states for E2E specs. Every fixture avoids random
 * values, current timestamps, network images, and external HTTP so
 * screenshots stay stable across runs.
 */

import { baseHostState, type E2EHostState } from './tiktools-host.ts';
import {
  OUTPUT_ACTION,
  SONICBOOM_ID,
  SPEAK_ACTION,
  OUTPUTS_SOURCE,
  VOICES_SOURCE,
  formPage,
  listPage,
  outputOptions,
  sonicboomConnectionPage,
  sonicboomSettings,
  sonicboomStatus,
  sonicboomTtsPage,
  sonicboomWebviewUi,
  textPage,
  voiceOptions,
} from './plugins.ts';

const FIXED_AT = 1_760_000_000_000;

export function emptyState(): E2EHostState {
  return baseHostState();
}

function withSonicboom(
  pages: Array<Record<string, unknown>>,
  uis: Array<Record<string, unknown>> = [],
): E2EHostState {
  return baseHostState({
    automationSnapshot: {
      actions: [],
      events: [],
      plugins: [sonicboomStatus()],
      actionTypes: [],
      eventTypes: [],
      pluginTemplates: [],
      pluginPages: pages,
      pluginUis: uis,
      translations: {},
    },
    pluginSettings: { [SONICBOOM_ID]: sonicboomSettings() },
    pluginOptions: {
      [VOICES_SOURCE]: voiceOptions(),
      [OUTPUTS_SOURCE]: outputOptions(),
    },
    pluginHealth: { [SONICBOOM_ID]: { ok: true, latencyMs: 12 } },
    actionOutcomes: {
      [SPEAK_ACTION]: { ok: true, summary: 'spoken', logs: ['POST /speak 200'] },
      [OUTPUT_ACTION]: { ok: true, summary: 'switched' },
    },
  });
}

export function sonicboomConnectedState(): E2EHostState {
  return withSonicboom([sonicboomConnectionPage(), sonicboomTtsPage()]);
}

export function sonicboomDisconnectedState(): E2EHostState {
  const state = withSonicboom([sonicboomConnectionPage(), sonicboomTtsPage()]);
  state.pluginHealth[SONICBOOM_ID] = { ok: false, latencyMs: 0, error: 'connection refused' };
  return state;
}

export function ttsPageState(): E2EHostState {
  return withSonicboom([sonicboomTtsPage()]);
}

/** Webview-mode TTS page: legacy nav entry plus the typed UI descriptor. */
export function webviewPageState(): E2EHostState {
  return withSonicboom([sonicboomTtsPage()], [sonicboomWebviewUi()]);
}

export function listPageState(): E2EHostState {
  return withSonicboom([listPage()]);
}

export function formPageState(): E2EHostState {
  return withSonicboom([formPage()]);
}

export function connectionPageState(): E2EHostState {
  return withSonicboom([sonicboomConnectionPage()]);
}

export function textPageState(): E2EHostState {
  return withSonicboom([textPage()]);
}

export function actionFailureState(): E2EHostState {
  const state = withSonicboom([sonicboomTtsPage()]);
  state.actionOutcomes[SPEAK_ACTION] = {
    ok: false,
    summary: 'synthesis failed',
    error: 'voice not found',
  };
  return state;
}

export function optionFailureState(): E2EHostState {
  const state = withSonicboom([listPage()]);
  state.failures['plugins.options'] = 'option source exploded';
  return state;
}

export function connectedCreatorState(): E2EHostState {
  const state = emptyState();
  state.creator = {
    uniqueId: 'demo_creator',
    roomId: 'room-1',
    nickname: 'Demo Creator',
    avatarUrl: null,
    title: 'Demo stream',
    lastConnected: FIXED_AT,
    connectCount: 3,
  };
  state.recentCreators = [
    {
      uniqueId: 'demo_creator',
      roomId: 'room-1',
      nickname: 'Demo Creator',
      avatarUrl: null,
      title: 'Demo stream',
      lastConnected: FIXED_AT,
      connectCount: 3,
    },
  ];
  state.leaderboard = [
    {
      uniqueId: 'alice',
      nickname: 'Alice',
      points: 120,
      level: 2,
      isSubscriber: false,
      totalChats: 40,
      totalCoins: 10,
      totalLikes: 5,
      totalShares: 1,
      firstSeen: FIXED_AT,
      lastSeen: FIXED_AT,
    },
  ];
  return state;
}
