<script lang="tsx">
import { onMounted, onUnmounted, ref } from 'vue';
import { defineVueComponent } from './vue/component.ts';

import {
  defaultTtsSettings,
  firstAvailableVoice,
  sanitizeTtsSettings,
  type TtsLogEntry,
  type TtsSettings,
} from '../../shared/tts/tts-policy.ts';
import { PluginBroker } from './broker.ts';
import { TtsSettingsPanel } from './components/TtsSettingsPanel.vue';
import { outputsTarget } from './components/source.ts';
import type { Locale } from './i18n/index.ts';
import type { ActionOptionItem } from './types.ts';

const SPEAK_ACTION = 'sonicboom.server.speak';
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
  const voices = ref<ActionOptionItem[]>([]);
  const voicesError = ref<string | undefined>(undefined);
  const outputs = ref<ActionOptionItem[] | undefined>(undefined);
  const outputsSelected = ref<string | undefined>(undefined);
  const outputsError = ref<string | undefined>(undefined);
  const outputsPending = ref<string | undefined>(undefined);
  const outputError = ref<string | undefined>(undefined);
  const speaking = ref(false);
  const logs = ref<TtsLogEntry[]>([]);
  const bootError = ref('');
  let activeSpeech = 0;
  let logSequence = 0;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const readTtsSettings = (values: Record<string, unknown>): TtsSettings => {
    const raw = values.tts;
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return defaultTtsSettings();
    return sanitizeTtsSettings({
      ...defaultTtsSettings(),
      ...(raw as Partial<TtsSettings>),
    });
  };

  const persistSettings = (): void => {
    const values = { ...fullSettings.value, tts: { ...settings.value } };
    fullSettings.value = values;
    void broker.setSettings(values).catch((failure: unknown) => {
      bootError.value = failure instanceof Error ? failure.message : String(failure);
    });
  };

  const schedulePersist = (): void => {
    if (saveTimer !== undefined) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = undefined;
      persistSettings();
    }, SETTINGS_DEBOUNCE_MS);
  };

  const handleSettingsChange = (next: TtsSettings): void => {
    settings.value = sanitizeTtsSettings(next);
    schedulePersist();
  };

  const appendLog = (entry: Omit<TtsLogEntry, 'id' | 'at'>): void => {
    const next: TtsLogEntry = { ...entry, id: ++logSequence, at: Date.now() };
    logs.value = [...logs.value, next].slice(-50);
  };

  const refreshVoices = (): void => {
    void broker
      .getOptions(VOICES_SOURCE, true)
      .then((result) => {
        voices.value = result.options;
        voicesError.value = undefined;
      })
      .catch((failure: unknown) => {
        voicesError.value = failure instanceof Error ? failure.message : String(failure);
      });
  };

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
    if (!clean) return;
    const current = settings.value;
    const resolvedVoice =
      voice.trim() ||
      current.defaultVoice.trim() ||
      firstAvailableVoice(voices.value.map((option) => option.value));
    activeSpeech += 1;
    speaking.value = true;
    void broker
      .executeAction(SPEAK_ACTION, {
        text: clean,
        voice: resolvedVoice,
        language: current.language,
        playNow: true,
      })
      .then((outcome) => {
        const lines = outcome.logs.length > 0 ? ` ${outcome.logs.slice(0, 3).join(' · ')}` : '';
        appendLog({
          ok: outcome.ok,
          source: 'tester',
          text: clean.slice(0, 160),
          voice: resolvedVoice,
          summary: `${outcome.ok ? outcome.summary : (outcome.error ?? outcome.summary)}${lines}`.slice(
            0,
            500,
          ),
        });
      })
      .catch((failure: unknown) => {
        const message = failure instanceof Error ? failure.message : String(failure);
        appendLog({
          ok: false,
          source: 'tester',
          text: clean.slice(0, 160),
          voice: resolvedVoice,
          summary: message.slice(0, 500),
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
      })
      .catch((failure: unknown) => {
        bootError.value = failure instanceof Error ? failure.message : String(failure);
      });
    void broker
      .getOptions(VOICES_SOURCE, false)
      .then((result) => {
        voices.value = result.options;
      })
      .catch((failure: unknown) => {
        voicesError.value = failure instanceof Error ? failure.message : String(failure);
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
    // Flush a pending debounced save before the broker goes away.
    if (saveTimer !== undefined) {
      clearTimeout(saveTimer);
      saveTimer = undefined;
      persistSettings();
    }
    broker.dispose();
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
      <TtsSettingsPanel
        locale={locale.value}
        settings={settings.value}
        voices={voices.value}
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
    );
  };
});

export default SonicBoomApp;
</script>
