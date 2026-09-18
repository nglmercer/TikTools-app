import { expect, test } from 'bun:test';
import { computed, ref } from 'vue';

import type { ActionOptionItem } from '../../shared/messages.ts';
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
