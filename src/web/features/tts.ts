import { computed, ref, type ComputedRef, type Ref } from 'vue';

import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import { legacyTtsContributions, type TtsContribution } from '../../plugin-ui/index.ts';
import type { ActionOptionItem, PluginSettingValues } from '../../shared/messages.ts';
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
    config: PluginSettingValues,
    live: boolean,
  ) => Promise<PluginActionOutcome>;
  refreshOptions: (source: string) => void;
  adjustPoints: (uniqueId: string, delta: number) => void;
  leaderboardPointsFor: (handle: string) => number | undefined;
}

/** Request-local speech context. Never looked up later by actionType. */
type TtsPendingEntry = {
  pluginId: string;
  actionType: string;
  source: 'tester' | 'auto';
  text: string;
  voice: string;
};

/** Request-local output-switch context. Never looked up later by actionType. */
type OutputPendingEntry = {
  pluginId: string;
  source: string;
  device: string;
};

export interface TtsOptions {
  /** Debounce window for settings persistence (ms). Defaults to 200. */
  debounceMs?: number;
  /** Timer injection for tests. */
  setTimeoutFn?: (cb: () => void, ms: number) => ReturnType<typeof setTimeout>;
  clearTimeoutFn?: (id: ReturnType<typeof setTimeout>) => void;
  /**
   * Explicit TTS contributions (future `plugins.ui.describe`). When
   * provided, auto-TTS and speech dispatch key off these instead of the
   * legacy adapter-derived contributions — never off UI sections.
   */
  contributions?: ComputedRef<TtsContribution[]>;
}

/** Default coalescing window for TTS settings persistence. */
export const TTS_SETTINGS_DEBOUNCE_MS = 200;

type PendingPersist = {
  timer: ReturnType<typeof setTimeout> | undefined;
  latest: TtsSettings;
  /** Monotonic revision: guards against stale out-of-order writes. */
  revision: number;
};

/** Host-owned TTS: per-plugin settings, speech queue, and auto chat TTS. */
export function useTts(
  control: ControlClient,
  actionOptions: Ref<Record<string, ActionOptionItem[]>>,
  pluginPages: ComputedRef<PluginPageDescriptor[]>,
  callbacks: TtsCallbacks,
  options?: TtsOptions,
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
  /**
   * Active speech count per plugin id. An active operation is always
   * represented until it settles — never evicted by queue size. The
   * `ttsSpeaking` flag derives from `count > 0`.
   */
  const activeSpeech = new Map<string, number>();
  let ttsLogSequence = 0;

  const debounceMs = options?.debounceMs ?? TTS_SETTINGS_DEBOUNCE_MS;
  const scheduleFn =
    options?.setTimeoutFn ?? ((cb: () => void, ms: number) => setTimeout(cb, ms));
  const cancelFn =
    options?.clearTimeoutFn ?? ((id: ReturnType<typeof setTimeout>) => clearTimeout(id));
  /** Per-plugin debounced persistence state (never a global debounce). */
  const pendingPersist = new Map<string, PendingPersist>();
  /** Per-plugin serialized write chain so host writes cannot complete out of order. */
  const persistChain = new Map<string, Promise<void>>();
  /** Last revision known to have been handed to the host, per plugin. */
  const persistedRevision = new Map<string, number>();

  /**
   * TTS activation targets. Explicit contributions win when the host
   * provides them; otherwise the legacy adapter derives them from v3
   * pages. This module never walks UI sections itself — the walk lives
   * in the adapter (documented migration step toward `plugins.ui.describe`).
   */
  const ttsSections: ComputedRef<
    Array<{ pluginId: string; actionType: string; voicesSource: string }>
  > = computed(() => {
    const contributions: TtsContribution[] =
      options?.contributions?.value ?? legacyTtsContributions(pluginPages.value);
    return contributions.map((contribution) => ({
      pluginId: contribution.pluginId,
      actionType: contribution.actionType,
      voicesSource: contribution.voicesFrom,
    }));
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

  const setSpeakingFromCount = (pluginId: string): void => {
    const count = activeSpeech.get(pluginId) ?? 0;
    ttsSpeaking.value = { ...ttsSpeaking.value, [pluginId]: count > 0 };
  };

  const incrementSpeaking = (pluginId: string): void => {
    activeSpeech.set(pluginId, (activeSpeech.get(pluginId) ?? 0) + 1);
    setSpeakingFromCount(pluginId);
  };

  const decrementSpeaking = (pluginId: string): void => {
    const next = Math.max(0, (activeSpeech.get(pluginId) ?? 0) - 1);
    if (next === 0) activeSpeech.delete(pluginId);
    else activeSpeech.set(pluginId, next);
    setSpeakingFromCount(pluginId);
  };

  const resolveSpeechOutcome = (pending: TtsPendingEntry, outcome: PluginActionOutcome): void => {
    const lines = outcome.logs.length > 0 ? ` ${outcome.logs.slice(0, 3).join(' · ')}` : '';
    appendTtsLog(pending.pluginId, {
      ok: outcome.ok,
      source: pending.source,
      text: pending.text.slice(0, 160),
      voice: pending.voice,
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
    // Request-local closure state: completion is correlated with the exact
    // request that initiated it, so concurrent calls sharing one actionType
    // can never cross-associate results.
    const pending: TtsPendingEntry = { pluginId, actionType, source, text, voice };
    incrementSpeaking(pluginId);
    void callbacks
      .executeAction(
        actionType,
        {
          text,
          voice,
          language,
          playNow,
        },
        true,
      )
      .then((outcome) => {
        resolveSpeechOutcome(pending, outcome);
      })
      .catch((failure: unknown) => {
        resolveSpeechOutcome(pending, {
          actionType,
          ok: false,
          summary: errorMessage(failure),
          logs: [],
          durationMs: 0,
          error: errorMessage(failure),
        });
      })
      .finally(() => {
        decrementSpeaking(pluginId);
      });
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

  const persistSettingsNow = (
    pluginId: string,
    clean: TtsSettings,
    revision: number,
  ): Promise<void> => {
    const previous = persistChain.get(pluginId) ?? Promise.resolve();
    const next = previous
      .then(() =>
        control.call('app.state.set', {
          key: ttsSettingsKey(pluginId),
          value: serializeTtsSettings(clean),
        }),
      )
      .then(() => {
        // Only the newest handed-off revision counts as persisted; older
        // chained writes completing later cannot mark newer state stale.
        if ((persistedRevision.get(pluginId) ?? 0) < revision) {
          persistedRevision.set(pluginId, revision);
        }
      })
      .catch((failure: unknown) => {
        console.warn(`app.state.set failed: ${errorMessage(failure)}`);
      });
    persistChain.set(pluginId, next);
    void next.then(() => {
      if (persistChain.get(pluginId) === next) persistChain.delete(pluginId);
    });
    return next;
  };

  /**
   * Flush one plugin's debounced write immediately (teardown/page change).
   * Resolves when the queued write settles so callers can persist before
   * transport shutdown.
   */
  const flushTtsSettings = (pluginId: string): Promise<void> => {
    const entry = pendingPersist.get(pluginId);
    if (!entry) return Promise.resolve();
    if (entry.timer !== undefined) {
      cancelFn(entry.timer);
      entry.timer = undefined;
    }
    pendingPersist.delete(pluginId);
    return persistSettingsNow(pluginId, entry.latest, entry.revision);
  };

  /**
   * Flush every pending debounced write (component/app teardown). Awaits
   * every queued write so the final values reach the host even when
   * teardown follows a change immediately.
   */
  const flushAllTtsSettings = (): Promise<void> => {
    const pending = [...pendingPersist.keys()].map((pluginId) => flushTtsSettings(pluginId));
    return Promise.all(pending).then(() => undefined);
  };

  const handleTtsSettingsChange = (pluginId: string, next: TtsSettings): void => {
    const clean = sanitizeTtsSettings(next);
    ttsDirty.add(pluginId);
    // Local reactive state updates instantly; persistence is debounced and
    // coalesced per plugin so slider drags emit one trailing write.
    ttsSettings.value = { ...ttsSettings.value, [pluginId]: clean };
    const existing = pendingPersist.get(pluginId);
    if (existing?.timer !== undefined) cancelFn(existing.timer);
    const revision = (existing?.revision ?? persistedRevision.get(pluginId) ?? 0) + 1;
    const entry: PendingPersist = { timer: undefined, latest: clean, revision };
    entry.timer = scheduleFn(() => {
      pendingPersist.delete(pluginId);
      entry.timer = undefined;
      persistSettingsNow(pluginId, entry.latest, entry.revision);
    }, debounceMs);
    pendingPersist.set(pluginId, entry);
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

  const handleOutputResult = (pending: OutputPendingEntry, outcome: PluginActionOutcome): void => {
    const rest = { ...ttsOutputPending.value };
    delete rest[pending.pluginId];
    ttsOutputPending.value = rest;
    if (outcome.ok) {
      const outputErrors = { ...ttsOutputErrors.value };
      delete outputErrors[pending.pluginId];
      ttsOutputErrors.value = outputErrors;
      callbacks.refreshOptions(pending.source);
    } else {
      ttsOutputErrors.value = {
        ...ttsOutputErrors.value,
        [pending.pluginId]: outcome.error ?? outcome.summary,
      };
    }
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
    // Closure-local context: the outcome is correlated with the request that
    // initiated it. The per-plugin pending flag remains the concurrency
    // guard against simultaneous switches for one plugin.
    const pending: OutputPendingEntry = { pluginId, source, device };
    ttsOutputPending.value = { ...ttsOutputPending.value, [pluginId]: device };
    const errors = { ...ttsOutputErrors.value };
    delete errors[pluginId];
    ttsOutputErrors.value = errors;
    void callbacks
      .executeAction(actionType, { [field]: device }, true)
      .then((outcome) => handleOutputResult(pending, outcome))
      .catch((failure: unknown) => {
        const message = errorMessage(failure);
        handleOutputResult(pending, {
          actionType,
          ok: false,
          summary: message,
          logs: [],
          durationMs: 0,
          error: message,
        });
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
    flushTtsSettings,
    flushAllTtsSettings,
    handleTtsSpeak,
    handleTtsOutputSelect,
    runAutoTts,
    refresh,
  };
}
