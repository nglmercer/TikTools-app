import { expect, test } from 'bun:test';
import { computed, ref } from 'vue';

import type { ActionOptionItem } from '../../shared/messages.ts';
import { defaultTtsSettings } from '../tts/tts-policy.ts';
import type { ControlClient } from '../platform/control-client.ts';
import type { PluginActionOutcome } from './plugins.ts';
import { useTts } from './tts.ts';

const SOURCE = 'plugin-action-options:sonicboom.server.set-output-device:device';

function stubControl(): ControlClient {
  return {
    attach: () => {},
    detach: () => {},
    call: async <T,>() => ({ state: {} }) as T,
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
}

function setup() {
  const actionOptions = ref<Record<string, ActionOptionItem[]>>({});
  const pluginPages = computed(() => []);
  const refreshed: string[] = [];
  let resolveExecution!: (outcome: PluginActionOutcome) => void;
  const executed: Array<{ actionType: string; config: Record<string, string | number | boolean> }> =
    [];
  const tts = useTts(stubControl(), actionOptions, pluginPages, {
    executeAction: (actionType, config, live) => {
      executed.push({ actionType, config });
      expect(live).toBe(true);
      return new Promise<PluginActionOutcome>((resolve) => {
        resolveExecution = resolve;
      });
    },
    refreshOptions: (source) => {
      refreshed.push(source);
    },
    adjustPoints: () => {},
    leaderboardPointsFor: () => undefined,
  });
  return {
    tts,
    refreshed,
    executed,
    settle: (outcome: PluginActionOutcome) => {
      resolveExecution(outcome);
    },
  };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

const outcome = (overrides: Partial<PluginActionOutcome>): PluginActionOutcome => ({
  actionType: 'sonicboom.server.set-output-device',
  ok: true,
  summary: 'switched',
  logs: [],
  durationMs: 1,
  ...overrides,
});

test('successful output switch force-refreshes the server selection', async () => {
  const { tts, refreshed, executed, settle } = setup();
  tts.handleTtsOutputSelect(
    'sonicboom.server',
    'sonicboom.server.set-output-device',
    'device',
    'Speakers',
    SOURCE,
  );
  // Pending device disables the selector while the switch is in flight.
  expect(tts.ttsOutputPending.value).toEqual({ 'sonicboom.server': 'Speakers' });
  expect(executed).toEqual([
    { actionType: 'sonicboom.server.set-output-device', config: { device: 'Speakers' } },
  ]);
  settle(outcome({ ok: true }));
  await flush();
  expect(tts.ttsOutputPending.value).toEqual({});
  expect(tts.ttsOutputErrors.value).toEqual({});
  // The post-mutation re-read is what moves the selector to the fresh
  // server-reported selection (host invalidates + bypasses its cache).
  expect(refreshed).toEqual([SOURCE]);
});

test('failed output switch keeps the last confirmed selection and shows the error', async () => {
  const { tts, refreshed, settle } = setup();
  tts.handleTtsOutputSelect(
    'sonicboom.server',
    'sonicboom.server.set-output-device',
    'device',
    'InvalidDevice',
    SOURCE,
  );
  expect(tts.ttsOutputPending.value).toEqual({ 'sonicboom.server': 'InvalidDevice' });
  settle(outcome({ ok: false, summary: 'Unknown audio output device.', error: 'Unknown audio output device.' }));
  await flush();
  expect(tts.ttsOutputPending.value).toEqual({});
  expect(tts.ttsOutputErrors.value).toEqual({
    'sonicboom.server': 'Unknown audio output device.',
  });
  // No re-read on failure: the selector keeps the last confirmed server
  // value instead of flashing to the rejected device.
  expect(refreshed).toEqual([]);
});

function setupSpeech() {
  const actionOptions = ref<Record<string, ActionOptionItem[]>>({});
  const pluginPages = computed(() => []);
  const resolvers: Array<(outcome: PluginActionOutcome) => void> = [];
  const rejecters: Array<(error: unknown) => void> = [];
  const tts = useTts(stubControl(), actionOptions, pluginPages, {
    executeAction: (actionType, config, live) => {
      expect(live).toBe(true);
      void actionType;
      void config;
      return new Promise<PluginActionOutcome>((resolve, reject) => {
        resolvers.push(resolve);
        rejecters.push(reject);
      });
    },
    refreshOptions: () => {},
    adjustPoints: () => {},
    leaderboardPointsFor: () => undefined,
  });
  return { tts, resolvers, rejecters };
}

const speechOutcome = (
  actionType: string,
  overrides: Partial<PluginActionOutcome> = {},
): PluginActionOutcome => ({
  actionType,
  ok: true,
  summary: 'spoken',
  logs: [],
  durationMs: 1,
  ...overrides,
});

test('concurrent identical action types keep their own text and voice', async () => {
  const { tts, resolvers } = setupSpeech();
  const actionType = 'sonicboom.server.speak';
  tts.handleTtsSpeak('sonicboom.server', actionType, 'speech A', 'voice-a');
  tts.handleTtsSpeak('sonicboom.server', actionType, 'speech B', 'voice-b');
  expect(resolvers).toHaveLength(2);
  expect(tts.ttsSpeaking.value).toEqual({ 'sonicboom.server': true });
  // Second completes first: each log must carry its own request's data.
  resolvers[1]?.(speechOutcome(actionType, { summary: 'ok-b' }));
  await flush();
  expect(tts.ttsSpeaking.value).toEqual({ 'sonicboom.server': true });
  resolvers[0]?.(speechOutcome(actionType, { summary: 'ok-a' }));
  await flush();
  expect(tts.ttsSpeaking.value).toEqual({ 'sonicboom.server': false });
  const logs = tts.ttsLogs.value['sonicboom.server'] ?? [];
  expect(logs).toHaveLength(2);
  expect(logs[0]).toMatchObject({ text: 'speech B', voice: 'voice-b', summary: 'ok-b' });
  expect(logs[1]).toMatchObject({ text: 'speech A', voice: 'voice-a', summary: 'ok-a' });
});

test('speaking stays true until the last concurrent request settles', async () => {
  const { tts, resolvers, rejecters } = setupSpeech();
  const actionType = 'sonicboom.server.speak';
  tts.handleTtsSpeak('sonicboom.server', actionType, 'one', 'v1');
  tts.handleTtsSpeak('sonicboom.server', actionType, 'two', 'v2');
  tts.handleTtsSpeak('sonicboom.server', actionType, 'three', 'v3');
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(true);
  resolvers[0]?.(speechOutcome(actionType));
  await flush();
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(true);
  rejecters[1]?.(new Error('boom'));
  await flush();
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(true);
  resolvers[2]?.(speechOutcome(actionType, { ok: false, summary: 'bad', error: 'bad' }));
  await flush();
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(false);
  const logs = tts.ttsLogs.value['sonicboom.server'] ?? [];
  expect(logs.map((entry) => entry.text)).toEqual(['one', 'two', 'three']);
  expect(logs[1]?.ok).toBe(false);
  expect(logs[2]?.ok).toBe(false);
});

test('concurrent speech across plugins tracks speaking independently', async () => {
  const { tts, resolvers } = setupSpeech();
  const actionType = 'sonicboom.server.speak';
  tts.handleTtsSpeak('plugin-a', actionType, 'hello a', 'va');
  tts.handleTtsSpeak('plugin-b', actionType, 'hello b', 'vb');
  expect(tts.ttsSpeaking.value).toEqual({ 'plugin-a': true, 'plugin-b': true });
  resolvers[0]?.(speechOutcome(actionType));
  await flush();
  expect(tts.ttsSpeaking.value).toEqual({ 'plugin-a': false, 'plugin-b': true });
  resolvers[1]?.(speechOutcome(actionType));
  await flush();
  expect(tts.ttsSpeaking.value).toEqual({ 'plugin-a': false, 'plugin-b': false });
  expect(tts.ttsLogs.value['plugin-a']?.[0]).toMatchObject({ text: 'hello a', voice: 'va' });
  expect(tts.ttsLogs.value['plugin-b']?.[0]).toMatchObject({ text: 'hello b', voice: 'vb' });
});

test('100+ sequential executions all resolve with correct logs', async () => {
  const { tts, resolvers } = setupSpeech();
  const actionType = 'sonicboom.server.speak';
  const total = 120;
  for (let i = 0; i < total; i += 1) {
    tts.handleTtsSpeak('sonicboom.server', actionType, `line ${i}`, `voice-${i % 3}`);
  }
  expect(resolvers).toHaveLength(total);
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(true);
  // Resolve out of order: no active request may become untraceable.
  for (let i = total - 1; i >= 0; i -= 1) {
    resolvers[i]?.(speechOutcome(actionType, { summary: `ok-${i}` }));
    await flush();
    if (i > 0) expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(true);
  }
  expect(tts.ttsSpeaking.value['sonicboom.server']).toBe(false);
  const logs = tts.ttsLogs.value['sonicboom.server'] ?? [];
  // The visible log keeps the last 50; every resolution still produced one.
  expect(logs).toHaveLength(50);
  expect(logs[0]).toMatchObject({ text: 'line 49', summary: 'ok-49' });
  expect(logs[logs.length - 1]).toMatchObject({ text: 'line 0', summary: 'ok-0' });
});

test('settings changes update instantly but persist debounced per plugin', async () => {
  const calls: Array<{ key: string; value: string }> = [];
  const control: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async <T,>(method: string, params?: Record<string, unknown>) => {
      if (method === 'app.state.set') {
        calls.push({
          key: String(params?.key ?? ''),
          value: String(params?.value ?? ''),
        });
        return undefined as T;
      }
      return { state: {} } as T;
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  let now = 0;
  let timerId = 0;
  const timers = new Map<number, { at: number; cb: () => void }>();
  const tts = useTts(
    control,
    ref<Record<string, ActionOptionItem[]>>({}),
    computed(() => []),
    {
      executeAction: async (actionType) => speechOutcome(actionType),
      refreshOptions: () => {},
      adjustPoints: () => {},
      leaderboardPointsFor: () => undefined,
    },
    {
      debounceMs: 200,
      setTimeoutFn: (cb, ms) => {
        timerId += 1;
        timers.set(timerId, { at: now + ms, cb });
        return timerId as unknown as ReturnType<typeof setTimeout>;
      },
      clearTimeoutFn: (id) => {
        timers.delete(id as unknown as number);
      },
    },
  );
  const base = defaultTtsSettings();
  // Rapid slider drag: 20 input events across two plugins.
  for (let i = 0; i < 10; i += 1) {
    tts.handleTtsSettingsChange('plugin-a', { ...base, volume: 0.1 + i * 0.01 });
    tts.handleTtsSettingsChange('plugin-b', { ...base, volume: 0.5 + i * 0.01 });
  }
  // Local state is instant.
  expect(tts.ttsSettings.value['plugin-a']?.volume).toBeCloseTo(0.19, 5);
  expect(tts.ttsSettings.value['plugin-b']?.volume).toBeCloseTo(0.59, 5);
  // Nothing persisted yet.
  expect(calls).toHaveLength(0);
  now += 200;
  for (const [, timer] of [...timers].sort((a, b) => a[1].at - b[1].at)) timer.cb();
  await flush();
  // One coalesced trailing write per plugin, not one per input event.
  expect(calls).toHaveLength(2);
  const byKey = new Map(calls.map((entry) => [entry.key, entry.value]));
  expect(byKey.get('tts.settings:plugin-a')).toContain('0.19');
  expect(byKey.get('tts.settings:plugin-b')).toContain('0.59');
});

test('flush persists pending settings immediately with final state', async () => {
  const calls: Array<{ key: string; value: string }> = [];
  const control: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async <T,>(method: string, params?: Record<string, unknown>) => {
      if (method === 'app.state.set') {
        calls.push({ key: String(params?.key ?? ''), value: String(params?.value ?? '') });
        return undefined as T;
      }
      return { state: {} } as T;
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const tts = useTts(
    control,
    ref<Record<string, ActionOptionItem[]>>({}),
    computed(() => []),
    {
      executeAction: async (actionType) => speechOutcome(actionType),
      refreshOptions: () => {},
      adjustPoints: () => {},
      leaderboardPointsFor: () => undefined,
    },
    { debounceMs: 10_000 },
  );
  const base = defaultTtsSettings();
  tts.handleTtsSettingsChange('plugin-a', { ...base, volume: 0.77 });
  expect(calls).toHaveLength(0);
  tts.flushTtsSettings('plugin-a');
  await flush();
  expect(calls).toHaveLength(1);
  expect(calls[0]?.key).toBe('tts.settings:plugin-a');
  expect(calls[0]?.value).toContain('0.77');
  // Flushing again without new edits writes nothing.
  await tts.flushAllTtsSettings();
  await flush();
  expect(calls).toHaveLength(1);
});

test('teardown flush delivers the final value even when teardown is immediate', async () => {
  const delivered: Array<{ key: string; value: string }> = [];
  let releaseWrite!: () => void;
  const writeGate = new Promise<void>((resolve) => {
    releaseWrite = resolve;
  });
  const control: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async <T,>(method: string, params?: Record<string, unknown>) => {
      if (method === 'app.state.set') {
        // Slow host: the RPC only completes when the test releases it,
        // simulating teardown racing an in-flight write.
        await writeGate;
        delivered.push({ key: String(params?.key ?? ''), value: String(params?.value ?? '') });
        return undefined as T;
      }
      return { state: {} } as T;
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const tts = useTts(
    control,
    ref<Record<string, ActionOptionItem[]>>({}),
    computed(() => []),
    {
      executeAction: async (actionType) => speechOutcome(actionType),
      refreshOptions: () => {},
      adjustPoints: () => {},
      leaderboardPointsFor: () => undefined,
    },
    { debounceMs: 10_000 },
  );
  const base = defaultTtsSettings();
  // Change, then immediately tear down: the debounced timer (10s) never
  // fires, so only the awaited flush can deliver the value.
  tts.handleTtsSettingsChange('plugin-a', { ...base, volume: 0.42 });
  tts.handleTtsSettingsChange('plugin-b', { ...base, volume: 0.84 });
  const tornDown = tts.flushAllTtsSettings();
  let settled = false;
  void tornDown.then(() => {
    settled = true;
  });
  await flush();
  // Still gated: the flush promise must not resolve before the host write.
  expect(settled).toBe(false);
  expect(delivered).toHaveLength(0);
  releaseWrite();
  await tornDown;
  expect(settled).toBe(true);
  expect(delivered).toHaveLength(2);
  const byKey = new Map(delivered.map((entry) => [entry.key, entry.value]));
  expect(byKey.get('tts.settings:plugin-a')).toContain('0.42');
  expect(byKey.get('tts.settings:plugin-b')).toContain('0.84');
});

test('output switch ignores empty devices and concurrent switches', () => {
  const { tts, refreshed, executed } = setup();
  tts.handleTtsOutputSelect(
    'sonicboom.server',
    'sonicboom.server.set-output-device',
    'device',
    '   ',
    SOURCE,
  );
  expect(executed).toEqual([]);
  tts.handleTtsOutputSelect(
    'sonicboom.server',
    'sonicboom.server.set-output-device',
    'device',
    'Speakers',
    SOURCE,
  );
  tts.handleTtsOutputSelect(
    'sonicboom.server',
    'sonicboom.server.set-output-device',
    'device',
    'Other',
    SOURCE,
  );
  expect(executed).toHaveLength(1);
  expect(tts.ttsOutputPending.value).toEqual({ 'sonicboom.server': 'Speakers' });
  expect(refreshed).toEqual([]);
});
