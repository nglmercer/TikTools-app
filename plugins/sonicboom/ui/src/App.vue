<script lang="tsx">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { defineVueComponent } from './vue/component.ts';

import {
  classifyTtsOutcomeOrigin,
  defaultTtsSettings,
  firstAvailableVoice,
  formatBudgetMs,
  sanitizeTtsSettings,
  ttsBudgetMs,
  type TtsLogEntry,
  type TtsSettings,
} from '../../shared/tts/tts-policy.ts';
import { PluginBroker } from './broker.ts';
import { TtsSettingsPanel } from './components/TtsSettingsPanel.vue';
import { outputsTarget } from './components/source.ts';
import { t, type Locale } from './i18n/index.ts';
import type { ActionOptionItem } from './types.ts';
import TtsPlayback from './components/TtsPlayback.vue';
import TtsIcon from './components/TtsIcon.vue';
import TtsToggle from './components/TtsToggle.vue';

const SPEAK_ACTION = 'sonicboom.server.speak';
const PROVIDERS_SOURCE = 'plugin-action-options:sonicboom.server.speak:provider';
const VOICES_SOURCE = 'plugin-action-options:sonicboom.server.speak:voice';
const OUTPUTS_SOURCE = 'plugin-action-options:sonicboom.server.set-output-device:device';

/** Coalescing window for settings persistence (sliders emit many events). */
const SETTINGS_DEBOUNCE_MS = 200;

/**
 * SonicBoom plugin UI root. Owns the broker client, TTS settings, voice /
 * output option state, speech dispatch, and logs. This component runs in
 * the isolated plugin WebView (desktop) or a sandboxed opaque-origin
 * iframe (development): it can reach the host only through `PluginBroker`
 * (the native `window.tiktools` surface or the `postMessage` frame
 * transport), never through the main window's privileged bridge, the DOM
 * parent, or dynamic imports.
 */
export const SonicBoomApp = defineVueComponent<Record<string, never>>([], () => {
  const broker = new PluginBroker();
  const locale = ref<Locale>('en');
  const fullSettings = ref<Record<string, unknown>>({});
  const settings = ref<TtsSettings>(defaultTtsSettings());
  const providers = ref<ActionOptionItem[]>([]);
  const catalog = ref<ActionOptionItem[]>([]);
  const voices = computed(() => catalog.value.filter(v => v.value.startsWith(`${settings.value.providerId}:`)));
  const ready = computed(() => providers.value.some(p => p.value === settings.value.providerId && !p.disabled));
  const refreshing = ref(false);
  const connectionEpoch = ref(0);
  const catalogStatus = ref('');
  let catalogEpoch = 0;
  const voicesError = ref<string | undefined>(undefined);
  const outputs = ref<ActionOptionItem[] | undefined>(undefined);
  const outputsSelected = ref<string | undefined>(undefined);
  const outputsError = ref<string | undefined>(undefined);
  const outputsPending = ref<string | undefined>(undefined);
  const outputError = ref<string | undefined>(undefined);
  const speaking = ref(false);
  const manualPlayNow = ref(false);
  const logs = ref<TtsLogEntry[]>([]);
  const bootError = ref('');
  let activeSpeech = 0;
  let logSequence = 0;
  const diagnostic = ref<Record<string, unknown>>({});
  let unsubscribeDiagnostic: (() => void) | undefined;
  let unsubscribeSettings: (() => void) | undefined;
  let ownSettingsEvents = 0;
  let connectionCheckPending = false;
  let disposed = false;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const readTtsSettings = (values: Record<string, unknown>): TtsSettings => {
    const raw = values.tts;
    const stored = raw && typeof raw === 'object' && !Array.isArray(raw) ? raw as Partial<TtsSettings> : {};
    return sanitizeTtsSettings({
      ...defaultTtsSettings(),
      defaultVoice: typeof values.defaultVoice === 'string' ? values.defaultVoice : '',
      language: typeof values.defaultLanguage === 'string' ? values.defaultLanguage : 'en',
      ...stored,
    });
  };

  let saveChain = Promise.resolve();
  const persistSettings = (): Promise<void> => {
    const snapshot = { ...settings.value };
    const identity = serverIdentity(fullSettings.value);
    const operation = saveChain.then(async () => {
      const latest = await broker.getSettings();
      if (identity !== serverIdentity(latest)) {
        catalogEpoch += 1;
        catalog.value = []; providers.value = []; refreshing.value = false;
        connectionEpoch.value++; fullSettings.value = latest; settings.value = readTtsSettings(latest);
        voicesError.value = t(locale.value, 'ttsConnectionChanged');
        return;
      }
      const previous = latest.tts && typeof latest.tts === 'object' && !Array.isArray(latest.tts) ? latest.tts : {};
      const values = { ...latest, tts: { ...previous, ...snapshot } };
      ownSettingsEvents += 1;
      try { await broker.setSettings(values); }
      catch (failure) { ownSettingsEvents = Math.max(0, ownSettingsEvents - 1); throw failure; }
      fullSettings.value = values;
    });
    saveChain = operation.catch((failure: unknown) => {
      bootError.value = failure instanceof Error ? failure.message : String(failure);
    });
    return operation;
  };

  const schedulePersist = (): void => {
    if (saveTimer !== undefined) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = undefined;
      void persistSettings().catch(() => undefined);
    }, SETTINGS_DEBOUNCE_MS);
  };

  const handleSettingsChange = (next: TtsSettings): void => {
    const providerChanged = next.providerId !== settings.value.providerId;
    if (providerChanged) {
      next = { ...next, defaultVoice: '', specialUsers: next.specialUsers.map(u => u.providerId ? u : ({ ...u, voice: '' })) };
      catalogStatus.value = t(locale.value, 'ttsProviderChanged');
    }
    settings.value = sanitizeTtsSettings(next);
    if (providerChanged) {
      if (saveTimer !== undefined) { clearTimeout(saveTimer); saveTimer = undefined; }
      settings.value.voiceCatalogRevision = crypto.randomUUID();
      void persistSettings().catch(() => undefined);
    } else schedulePersist();
  };

  const appendLog = (entry: Omit<TtsLogEntry, 'id' | 'at'>): void => {
    const next: TtsLogEntry = { ...entry, id: ++logSequence, at: Date.now() };
    logs.value = [...logs.value, next].slice(-50);
  };

  const serverIdentity = (values: Record<string, unknown>): string => JSON.stringify([values.serverUrl ?? '', values.apiToken ?? '']);
  const loadCatalog = async (refresh: boolean): Promise<void> => {
    if (refreshing.value) return;
    const epoch = ++catalogEpoch;
    refreshing.value = true;
    catalogStatus.value = '';
    try {
      await saveChain;
      if (disposed) return;
      const latest = await broker.getSettings();
      const identity = serverIdentity(latest);
      if (identity !== serverIdentity(fullSettings.value)) {
        connectionEpoch.value++;
        catalog.value = []; providers.value = [];
        settings.value = readTtsSettings(latest);
      }
      fullSettings.value = latest;
      if (refresh && saveTimer !== undefined) {
        clearTimeout(saveTimer); saveTimer = undefined;
        await persistSettings();
      }
      const [providerResult, voiceResult] = await Promise.allSettled([
        broker.getOptions(PROVIDERS_SOURCE, refresh), broker.getOptions(VOICES_SOURCE, refresh),
      ]);
      if (epoch !== catalogEpoch) return;
      const connected = await broker.getSettings();
      if (identity !== serverIdentity(connected)) {
        connectionEpoch.value++;
        catalog.value = []; providers.value = [];
        fullSettings.value = connected;
        settings.value = readTtsSettings(connected);
        throw new Error(t(locale.value, 'ttsConnectionChanged'));
      }
      if (providerResult.status === 'fulfilled') providers.value = providerResult.value.options;
      if (voiceResult.status === 'fulfilled') catalog.value = voiceResult.value.options;
      if (providerResult.status === 'rejected') throw providerResult.reason;
      if (voiceResult.status === 'rejected') throw voiceResult.reason;
      const selectionBefore = JSON.stringify(settings.value);
      const current = settings.value;
      const previousProvider = current.providerId;
      if (providers.value.length === 1 && !providers.value[0]?.disabled && providers.value[0]?.value !== current.providerId) {
        handleSettingsChange({ ...current, providerId: providers.value[0]!.value });
      }
      const qualify = (v: string): string => v && !v.includes(':') && settings.value.providerId === 'supertonic' ? `supertonic:${v}` : v;
      const valid = (v: string): string => voices.value.some(o => o.value === qualify(v)) ? qualify(v) : '';
      // Loading is temporary: keep saved voice preferences until a ready catalog can validate them.
      const reconcile = ready.value ? valid : qualify;
      settings.value = { ...settings.value, defaultVoice: reconcile(settings.value.defaultVoice), specialUsers: settings.value.specialUsers.map(u => {
        const provider = u.providerId || settings.value.providerId;
        const voice = u.voice && !u.voice.includes(':') && provider === 'supertonic' ? `supertonic:${u.voice}` : u.voice;
        const providerReady = providers.value.some(p => p.value === provider && !p.disabled);
        return { ...u, voice: !providerReady || catalog.value.some(v => v.value === voice && v.value.startsWith(`${provider}:`)) ? voice : '' };
      }) };
      if (refresh) settings.value.voiceCatalogRevision = crypto.randomUUID();
      if (refresh || selectionBefore !== JSON.stringify(settings.value) || saveTimer !== undefined) {
        if (saveTimer !== undefined) { clearTimeout(saveTimer); saveTimer = undefined; }
        await persistSettings();
      }
      if (epoch !== catalogEpoch) return;
      voicesError.value = undefined;
      catalogStatus.value = `${previousProvider !== settings.value.providerId ? `${t(locale.value, 'ttsProviderChanged')} ` : ''}${t(locale.value, 'ttsCatalogUpdated')}`;
    } catch (failure) {
      if (epoch === catalogEpoch) voicesError.value = `${t(locale.value, 'ttsReloadFailed')}: ${failure instanceof Error ? failure.message : String(failure)}`;
    } finally {
      if (epoch === catalogEpoch) refreshing.value = false;
    }
  };
  const refreshVoices = (): void => { void loadCatalog(true); };

  // Settings can change in the host connection editor while this WebView stays mounted.
  // This reads only broker-managed settings; catalogs still use host-owned option sources.
  const syncConnection = async (): Promise<void> => {
    if (disposed || connectionCheckPending) return;
    connectionCheckPending = true;
    try {
      const latest = await broker.getSettings();
      if (disposed || serverIdentity(latest) === serverIdentity(fullSettings.value)) return;
      catalogEpoch += 1; refreshing.value = false;
      catalog.value = []; providers.value = [];
      if (saveTimer !== undefined) { clearTimeout(saveTimer); saveTimer = undefined; }
      connectionEpoch.value++; fullSettings.value = latest; settings.value = readTtsSettings(latest);
      void loadCatalog(true);
    } catch { /* A transport interruption keeps the last known connection until retry. */ }
    finally { connectionCheckPending = false; }
  };
  const checkConnection = (): void => { void syncConnection(); };

  const refreshOutputs = (): void => {
    void broker
      .getOptions(OUTPUTS_SOURCE, true)
      .then((result) => {
        outputs.value = result.options;
        outputsSelected.value = result.selected ?? undefined;
        outputsError.value = undefined;
      })
      .catch((failure: unknown) => {
        outputsError.value = failure instanceof Error ? failure.message : String(failure);
      });
  };

  const handleSpeak = (text: string, voice: string): void => {
    const clean = text.trim().slice(0, 4_096);
    if (!clean || !ready.value || !voices.value.length) return;
    const current = settings.value;
    const resolvedVoice =
      voice.trim() ||
      current.defaultVoice.trim() ||
      firstAvailableVoice(voices.value.map((option) => option.value));
    if (!voices.value.some(v => v.value === resolvedVoice)) return;
    activeSpeech += 1;
    speaking.value = true;
    const budgetMs = ttsBudgetMs(current.providerId);
    const startedAt = Date.now();
    void broker.getSettings().then(latest => {
      if (serverIdentity(latest) !== serverIdentity(fullSettings.value)) {
        catalogEpoch += 1; catalog.value = []; providers.value = []; refreshing.value = false;
        connectionEpoch.value++; fullSettings.value = latest; settings.value = readTtsSettings(latest);
        throw new Error(t(locale.value, 'ttsConnectionChanged'));
      }
      if (settings.value.providerId !== current.providerId || !ready.value || !voices.value.some(v => v.value === resolvedVoice)) throw new Error(t(locale.value, 'ttsSelectionChanged'));
      return broker.executeAction(SPEAK_ACTION, {
        text: clean,
        provider: current.providerId,
        voice: resolvedVoice,
        language: current.language,
        playNow: manualPlayNow.value,
      });
    })
      .then((outcome) => {
        const lines = outcome.logs.length > 0 ? ` ${outcome.logs.slice(0, 3).join(' · ')}` : '';
        const reason = outcome.ok ? outcome.summary : (outcome.error ?? outcome.summary);
        // Redacted diagnostics: selected provider, expected budget, actual
        // elapsed time, and where a failure stopped. Never voice reference
        // audio, tokens, transcripts, or spoken text.
        const diag = `provider=${current.providerId} budget=${formatBudgetMs(budgetMs)} elapsed=${outcome.durationMs}ms origin=${classifyTtsOutcomeOrigin(reason)}`;
        appendLog({
          ok: outcome.ok,
          source: 'tester',
          text: '',
          voice: resolvedVoice,
          summary: `${diag} · ${reason}${lines}`.slice(0, 500),
        });
      })
      .catch((failure: unknown) => {
        const message = failure instanceof Error ? failure.message : String(failure);
        const diag = `provider=${current.providerId} budget=${formatBudgetMs(budgetMs)} elapsed=${Date.now() - startedAt}ms origin=${classifyTtsOutcomeOrigin(message)}`;
        appendLog({
          ok: false,
          source: 'tester',
          text: '',
          voice: resolvedVoice,
          summary: `${diag} · ${message}`.slice(0, 500),
        });
      })
      .finally(() => {
        activeSpeech = Math.max(0, activeSpeech - 1);
        speaking.value = activeSpeech > 0;
      });
  };

  const handleSelectOutput = (device: string): void => {
    const target = outputsTarget(OUTPUTS_SOURCE);
    if (!device.trim() || !target || outputsPending.value !== undefined) return;
    outputsPending.value = device;
    outputError.value = undefined;
    void broker
      .executeAction(target.actionType, { [target.field]: device })
      .then((outcome) => {
        outputsPending.value = undefined;
        if (outcome.ok) {
          outputError.value = undefined;
          refreshOutputs();
        } else {
          outputError.value = outcome.error ?? outcome.summary;
        }
      })
      .catch((failure: unknown) => {
        outputsPending.value = undefined;
        outputError.value = failure instanceof Error ? failure.message : String(failure);
      });
  };

  onMounted(() => {
    window.addEventListener('focus', checkConnection);
    unsubscribeDiagnostic = broker.subscribe('plugin.event', data => {
      if (!data || typeof data !== 'object') return;
      const envelope = data as { pluginId?: string; eventType?: string; event?: Record<string, unknown> };
      if (envelope.pluginId !== 'sonicboom.server' || envelope.eventType !== 'sonicboom.speech-status' || !envelope.event) return;
      const payload = envelope.event.data ?? envelope.event;
      if (payload && typeof payload === 'object') diagnostic.value = payload as Record<string, unknown>;
    });
    unsubscribeSettings = broker.subscribe('plugin.settings-changed', data => {
      if (!data || typeof data !== 'object' || (data as { pluginId?: string }).pluginId !== 'sonicboom.server') return;
      // The host emits one event per successful settings write. Consume our own
      // echoes, but refresh on external writes even when secrets remain redacted.
      if (ownSettingsEvents > 0) { ownSettingsEvents -= 1; return; }
      connectionEpoch.value++; catalogEpoch += 1; refreshing.value = false;
      catalog.value = []; providers.value = [];
      void loadCatalog(false);
    });
    void broker
      .getLocale()
      .then((value) => {
        locale.value = value === 'es' ? 'es' : 'en';
      })
      .catch(() => undefined);
    void broker
      .getTheme()
      .then((value) => {
        document.documentElement.dataset.theme = value === 'light' ? 'light' : 'dark';
      })
      .catch(() => {
        document.documentElement.dataset.theme = 'dark';
      });
    void broker
      .getSettings()
      .then((values) => {
        fullSettings.value = values;
        settings.value = readTtsSettings(values);
        void loadCatalog(false);
      })
      .catch((failure: unknown) => {
        bootError.value = failure instanceof Error ? failure.message : String(failure);
      });
    void broker
      .getOptions(OUTPUTS_SOURCE, false)
      .then((result) => {
        outputs.value = result.options;
        outputsSelected.value = result.selected ?? undefined;
      })
      .catch((failure: unknown) => {
        outputsError.value = failure instanceof Error ? failure.message : String(failure);
      });
  });

  onUnmounted(() => {
    disposed = true;
    window.removeEventListener('focus', checkConnection);
    unsubscribeSettings?.();
    unsubscribeDiagnostic?.();
    // Flush a pending debounced save before the broker goes away.
    if (saveTimer !== undefined) {
      clearTimeout(saveTimer);
      saveTimer = undefined;
      void persistSettings().catch(() => undefined);
    }
    catalogEpoch += 1;
    // Finish a pending settings write before closing its broker transport.
    void saveChain.finally(() => broker.dispose());
  });

  return () => {
    if (bootError.value && voices.value.length === 0) {
      return (
        <div class="tts-card">
          <div class="plg-alert" role="status">
            {bootError.value}
          </div>
        </div>
      );
    }
    return (
      <div class="tts-workspace">
      <header class="tts-workspace-heading"><div><span class="tts-eyebrow">SonicBoom · TTS</span><h2><TtsIcon name="voice"/>{t(locale.value, 'ttsWorkspaceTitle')}</h2><p>{t(locale.value, 'ttsWorkspaceLead')}</p></div><span class={['tts-health', { 'is-ready': ready.value }]}><span/>{t(locale.value, ready.value ? 'ttsReady' : 'ttsNotReady')}</span></header>
      <TtsPlayback broker={broker} locale={locale.value} enabled={settings.value.trackPlayback}
        connectionKey={String(connectionEpoch.value)}
        onToggle={(trackPlayback: boolean) => handleSettingsChange({ ...settings.value, trackPlayback })}/>
      <TtsSettingsPanel
        locale={locale.value}
        settings={settings.value}
        voices={voices.value}
        allVoices={catalog.value}
        providers={providers.value}
        catalogStatus={catalogStatus.value}
        refreshing={refreshing.value}
        providerReady={ready.value}
        voicesError={voicesError.value}
        speaking={speaking.value}
        logs={logs.value}
        onSettingsChange={handleSettingsChange}
        onRefreshVoices={refreshVoices}
        onSpeak={handleSpeak}
        outputsSupported={true}
        outputs={outputs.value}
        outputsSelected={outputsSelected.value}
        outputsError={outputsError.value}
        outputsPending={outputsPending.value}
        outputError={outputError.value}
        onSelectOutput={handleSelectOutput}
        onRefreshOutputs={refreshOutputs}
      />
      <details class="tts-card tts-advanced"><summary><TtsIcon name="sliders"/>{t(locale.value, 'ttsAdvanced')}</summary>
      <details><summary>{t(locale.value, 'ttsDiagnostics')}</summary>
        <p>{settings.value.providerId} · {ready.value ? t(locale.value, 'ttsReady') : t(locale.value, 'ttsNotReady')} · {settings.value.defaultVoice || '—'}</p>
        <p>{t(locale.value, 'ttsBackend')}: {String(diagnostic.value.connection ?? '—')} · {t(locale.value, 'ttsQueueDepth')}: {String(diagnostic.value.queueDepth ?? '—')}</p>
        <p>{String(diagnostic.value.stage ?? '—')} · {String(diagnostic.value.lastResult ?? '—')}</p>
        <button type="button" onClick={refreshVoices}>{t(locale.value, 'ttsReloadDiagnostics')}</button>
      </details>
      <TtsToggle label={t(locale.value, 'ttsManualInterrupt')} hint={t(locale.value, 'ttsManualInterrupt')} checked={manualPlayNow.value} onChange={(checked: boolean) => { manualPlayNow.value = checked; }}/>
      </details>
      </div>
    );
  };
});

export default SonicBoomApp;
</script>
