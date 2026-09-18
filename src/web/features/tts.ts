import { computed, ref, type ComputedRef, type Ref } from 'vue';

import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import { normalizeOptionsFrom } from '../../automation/plugins/declarative.ts';
import type { ActionOptionItem } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import {
  decideTts,
  defaultTtsSettings,
  firstAvailableVoice,
  parseTtsSettings,
  sanitizeTtsSettings,
  serializeTtsSettings,
  ttsFingerprint,
  TtsDeduper,
  ttsSettingsKey,
  type TtsLogEntry,
  type TtsSettings,
} from '../tts/tts-policy.ts';
import type { PluginActionOutcome } from './plugins.ts';

export interface TtsCallbacks {
  executeAction: (
    actionType: string,
    config: Record<string, string | number | boolean>,
    live: boolean,
  ) => Promise<PluginActionOutcome>;
  refreshOptions: (source: string) => void;
  adjustPoints: (uniqueId: string, delta: number) => void;
  leaderboardPointsFor: (handle: string) => number | undefined;
}

type TtsPendingEntry = {
  pluginId: string;
  actionType: string;
  source: 'tester' | 'auto';
  text: string;
  voice: string;
};

type OutputPendingEntry = {
  pluginId: string;
  actionType: string;
  source: string;
  device: string;
};

/** Host-owned TTS: per-plugin settings, speech queue, and auto chat TTS. */
export function useTts(
  control: ControlClient,
  actionOptions: Ref<Record<string, ActionOptionItem[]>>,
  pluginPages: ComputedRef<PluginPageDescriptor[]>,
  callbacks: TtsCallbacks,
) {
  // Settings persist through app-state (`tts.settings:<pluginId>`); logs
  // and speaking flags are local.
  const ttsSettings = ref<Record<string, TtsSettings>>({});
  const ttsSpeaking = ref<Record<string, boolean>>({});
  const ttsLogs = ref<Record<string, TtsLogEntry[]>>({});
  /** Audio output switch in flight per plugin id (TTS output selector). */
  const ttsOutputPending = ref<Record<string, string>>({});
  /** Last audio output switch failure per plugin id. */
  const ttsOutputErrors = ref<Record<string, string>>({});
  const ttsDirty = new Set<string>();
  const ttsDeduper = new TtsDeduper({ windowMs: 1500 });
  const ttsPending: TtsPendingEntry[] = [];
  const outputPending: OutputPendingEntry[] = [];
  let ttsLogSequence = 0;

  /** Every host-owned TTS section across plugin pages, with voice sources. */
  const ttsSections: ComputedRef<
    Array<{ pluginId: string; actionType: string; voicesSource: string }>
  > = computed(() => {
    const found: Array<{ pluginId: string; actionType: string; voicesSource: string }> = [];
    const pages: PluginPageDescriptor[] = pluginPages.value;
    for (const page of pages) {
      for (const section of page.sections) {
        if (section.kind !== 'tts' || !section.actionType || !section.voicesFrom) continue;
        const voicesSource = normalizeOptionsFrom(section.voicesFrom);
        if (!voicesSource) continue;
        found.push({ pluginId: page.pluginId, actionType: section.actionType, voicesSource });
      }
    }
    return found;
  });

  const ttsSettingsFor = (pluginId: string): TtsSettings =>
    ttsSettings.value[pluginId] ?? defaultTtsSettings();

  const appendTtsLog = (pluginId: string, entry: Omit<TtsLogEntry, 'id' | 'at'>): void => {
    const next: TtsLogEntry = { ...entry, id: ++ttsLogSequence, at: Date.now() };
    ttsLogs.value = {
      ...ttsLogs.value,
      [pluginId]: [...(ttsLogs.value[pluginId] ?? []), next].slice(-50),
    };
  };

  const resolveSpeechOutcome = (outcome: PluginActionOutcome): void => {
    const pendingIndex = ttsPending.findIndex((entry) => entry.actionType === outcome.actionType);
    const pending = pendingIndex >= 0 ? ttsPending.splice(pendingIndex, 1)[0] : undefined;
    const pluginId = pending?.pluginId;
    if (!pluginId) return;
    const stillPending = ttsPending.some((entry) => entry.pluginId === pluginId);
    ttsSpeaking.value = { ...ttsSpeaking.value, [pluginId]: stillPending };
    const lines = outcome.logs.length > 0 ? ` ${outcome.logs.slice(0, 3).join(' · ')}` : '';
    appendTtsLog(pluginId, {
      ok: outcome.ok,
      source: pending?.source ?? 'tester',
      text: (pending?.text ?? '').slice(0, 160),
      voice: pending?.voice ?? '',
      summary: `${outcome.ok ? outcome.summary : (outcome.error ?? outcome.summary)}${lines}`.slice(
        0,
        500,
      ),
    });
  };

  const queueTtsSpeak = (
    pluginId: string,
    actionType: string,
    source: 'tester' | 'auto',
    text: string,
    voice: string,
    language: string,
    playNow: boolean,
  ): void => {
    ttsPending.push({ pluginId, actionType, source, text, voice });
    if (ttsPending.length > 100) ttsPending.splice(0, ttsPending.length - 100);
    ttsSpeaking.value = { ...ttsSpeaking.value, [pluginId]: true };
    void callbacks
      .executeAction(actionType, { text, voice, language, playNow }, true)
      .then(resolveSpeechOutcome);
  };

  /** Automatic chat TTS: one full-pipeline decision per TTS section.
   * Points are deducted at most once per claimed fingerprint, after the
   * deduper accepts the line and before the speech request is queued. */
  const runAutoTts = (
    author: string,
    text: string,
    points: number | undefined,
    isSubscriber: boolean | undefined,
  ): void => {
    const sections = ttsSections.value;
    if (sections.length === 0) return;
    for (const section of sections) {
      const settings = ttsSettingsFor(section.pluginId);
      if (!settings.enabled) continue;
      const availableVoices = (actionOptions.value[section.voicesSource] ?? []).map(
        (option) => option.value,
      );
      const decision = decideTts({
        comment: text,
        author: {
          handle: author,
          points: points ?? callbacks.leaderboardPointsFor(author),
          roles: isSubscriber === undefined ? {} : { isSubscriber },
        },
        settings,
        availableVoices,
      });
      if (!decision.speak) continue;
      if (!ttsDeduper.claim(ttsFingerprint(author, decision.spokenText))) continue;
      if (decision.pointsCost > 0) {
        callbacks.adjustPoints(author.trim().replace(/^@/, ''), -decision.pointsCost);
      }
      queueTtsSpeak(
        section.pluginId,
        section.actionType,
        'auto',
        decision.spokenText,
        decision.voice,
        decision.language,
        false,
      );
    }
  };

  const applyAppState = (state: Record<string, string>): void => {
    for (const [key, value] of Object.entries(state)) {
      if (!key.startsWith('tts.settings:')) continue;
      const pluginId = key.slice('tts.settings:'.length);
      if (!pluginId || ttsDirty.has(pluginId)) continue;
      ttsSettings.value = { ...ttsSettings.value, [pluginId]: parseTtsSettings(value) };
    }
  };

  control.onPush('app-state', (message) => {
    if (message.type !== 'app-state') return;
    applyAppState(message.state);
  });

  const refresh = async (): Promise<void> => {
    try {
      const result = await control.call<{ state: Record<string, string> }>('app.state.get', {});
      applyAppState(result.state);
    } catch (failure) {
      console.warn(`app.state.get failed: ${errorMessage(failure)}`);
    }
  };

  const handleTtsSettingsChange = (pluginId: string, next: TtsSettings): void => {
    const clean = sanitizeTtsSettings(next);
    ttsDirty.add(pluginId);
    ttsSettings.value = { ...ttsSettings.value, [pluginId]: clean };
    void control
      .call('app.state.set', { key: ttsSettingsKey(pluginId), value: serializeTtsSettings(clean) })
      .catch((failure: unknown) => {
        console.warn(`app.state.set failed: ${errorMessage(failure)}`);
      });
  };

  const handleTtsSpeak = (
    pluginId: string,
    actionType: string,
    text: string,
    voice: string,
  ): void => {
    const settings = ttsSettingsFor(pluginId);
    const clean = text.trim().slice(0, 4_096);
    if (!clean) return;
    // Same trailing fallback as automatic TTS: never send an empty voice
    // while a voice list is known (servers 400 on present-but-empty params).
    const section = ttsSections.value.find((entry) => entry.pluginId === pluginId);
    const availableVoices = section
      ? (actionOptions.value[section.voicesSource] ?? []).map((option) => option.value)
      : [];
    const resolvedVoice =
      voice.trim() || settings.defaultVoice.trim() || firstAvailableVoice(availableVoices);
    queueTtsSpeak(pluginId, actionType, 'tester', clean, resolvedVoice, settings.language, true);
  };

  /**
   * Runs the outputs switch action immediately (TTS audio output selector).
   * The device is sent verbatim; the result handler force-refreshes the
   * server selection on success and surfaces the error without persisting
   * on failure, so the selector always reflects server state.
   */
  const handleTtsOutputSelect = (
    pluginId: string,
    actionType: string,
    field: string,
    device: string,
    source: string,
  ): void => {
    if (!device.trim() || ttsOutputPending.value[pluginId] !== undefined) return;
    outputPending.push({ pluginId, actionType, source, device });
    if (outputPending.length > 20) outputPending.splice(0, outputPending.length - 20);
    ttsOutputPending.value = { ...ttsOutputPending.value, [pluginId]: device };
    const errors = { ...ttsOutputErrors.value };
    delete errors[pluginId];
    ttsOutputErrors.value = errors;
    void callbacks.executeAction(actionType, { [field]: device }, true).then((outcome) => {
      const outputIndex = outputPending.findIndex(
        (entry) => entry.actionType === outcome.actionType,
      );
      const output = outputIndex >= 0 ? outputPending.splice(outputIndex, 1)[0] : undefined;
      if (!output) return;
      const rest = { ...ttsOutputPending.value };
      delete rest[output.pluginId];
      ttsOutputPending.value = rest;
      if (outcome.ok) {
        const outputErrors = { ...ttsOutputErrors.value };
        delete outputErrors[output.pluginId];
        ttsOutputErrors.value = outputErrors;
        callbacks.refreshOptions(output.source);
      } else {
        ttsOutputErrors.value = {
          ...ttsOutputErrors.value,
          [output.pluginId]: outcome.error ?? outcome.summary,
        };
      }
    });
  };

  const ttsSettingsOrDefault = (pluginId: string): TtsSettings => ttsSettingsFor(pluginId);

  return {
    ttsSettings,
    ttsSpeaking,
    ttsLogs,
    ttsOutputPending,
    ttsOutputErrors,
    ttsSettingsOrDefault,
    handleTtsSettingsChange,
    handleTtsSpeak,
    handleTtsOutputSelect,
    runAutoTts,
    refresh,
  };
}
