<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import type { ActionOptionItem } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import {
  TTS_LIMITS,
  TTS_SPEED_PITCH_UNSUPPORTED,
  clampNumber,
  type TtsCommentMode,
  type TtsLogEntry,
  type TtsSettings,
} from '../../tts/tts-policy.ts';
import { voiceOptions } from './tts-voice-options.ts';
import { TtsAllowedUsers } from './TtsAllowedUsers.vue';
import { TtsOutputsCard } from './TtsOutputsCard.vue';
import { TtsSpecialUsers } from './TtsSpecialUsers.vue';
import { TtsVoiceTester } from './TtsVoiceTester.vue';

type TtsSettingsPanelProps = {
  locale: Locale;
  settings: TtsSettings;
  voices: ActionOptionItem[];
  voicesError?: string;
  speaking: boolean;
  logs: TtsLogEntry[];
  onSettingsChange: (next: TtsSettings) => void;
  onRefreshVoices: () => void;
  onSpeak: (text: string, voice: string) => void;
  /** False when the section declares no outputs source: the card hides. */
  outputsSupported: boolean;
  /** Server output options; undefined while the first fetch is in flight. */
  outputs?: ActionOptionItem[];
  /** Server-reported active output; the authoritative selection. */
  outputsSelected?: string;
  /** Fetch failure; hides the selector but never the panel. */
  outputsError?: string;
  /** Device currently being switched to; disables the selector. */
  outputsPending?: string;
  /** Last switch failure; the selector keeps showing server state. */
  outputError?: string;
  onSelectOutput?: (device: string) => void;
  onRefreshOutputs?: () => void;
};

/**
 * Host-owned TTS settings panel for `tts` plugin page sections. Renders
 * only host controls over manifest-declared voice sources; no
 * plugin-provided markup or script is involved.
 */
export const TtsSettingsPanel = defineVueComponent<TtsSettingsPanelProps>(
  ['locale', 'settings', 'voices', 'voicesError', 'speaking', 'logs', 'onSettingsChange', 'onRefreshVoices', 'onSpeak', 'outputsSupported', 'outputs', 'outputsSelected', 'outputsError', 'outputsPending', 'outputError', 'onSelectOutput', 'onRefreshOutputs'],
  (props) => {
    const update = (patch: Partial<TtsSettings>): void => {
      props.onSettingsChange({ ...props.settings, ...patch });
    };

    const updateNumber = (key: 'defaultSpeed' | 'defaultPitch' | 'volume' | 'pointsCost', raw: string): void => {
      const value = Number(raw);
      if (!Number.isFinite(value)) return;
      switch (key) {
        case 'defaultSpeed':
          update({ defaultSpeed: clampNumber(value, TTS_LIMITS.minSpeed, TTS_LIMITS.maxSpeed, 1) });
          break;
        case 'defaultPitch':
          update({ defaultPitch: clampNumber(value, TTS_LIMITS.minPitch, TTS_LIMITS.maxPitch, 1) });
          break;
        case 'volume':
          update({ volume: clampNumber(value, TTS_LIMITS.minVolume, TTS_LIMITS.maxVolume, 1) });
          break;
        case 'pointsCost':
          update({ pointsCost: Math.round(clampNumber(value, TTS_LIMITS.minPointsCost, TTS_LIMITS.maxPointsCost, 0)) });
          break;
      }
    };

    const setCommentMode = (mode: TtsCommentMode): void => {
      update({ commentMode: mode });
    };

    return () => {
      const settings = props.settings;
      const voiceList = voiceOptions(props.voices, settings.defaultVoice);

      return (
        <div>
          <div class="tts-grid tts-grid--top">
            <TtsOutputsCard
              locale={props.locale}
              outputsSupported={props.outputsSupported}
              outputs={props.outputs}
              outputsError={props.outputsError}
              outputsSelected={props.outputsSelected}
              outputsPending={props.outputsPending}
              outputError={props.outputError}
              onSelectOutput={props.onSelectOutput}
              onRefreshOutputs={props.onRefreshOutputs}
            />
            <section class="tts-card">
              <h4 class="tts-card__title">General Settings</h4>
              <label class="tts-check">
                <input
                  type="checkbox"
                  checked={settings.enabled}
                  onChange={(event) => update({ enabled: (event.currentTarget as HTMLInputElement).checked })}
                />
                Enabled
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-language">Language</label>
                <input
                  id="tts-language"
                  class="tts-input"
                  type="text"
                  value={settings.language}
                  maxlength={TTS_LIMITS.maxLanguageLength}
                  placeholder="en"
                  onInput={(event) => update({ language: (event.currentTarget as HTMLInputElement).value.slice(0, TTS_LIMITS.maxLanguageLength) || 'en' })}
                />
              </div>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-default-voice">Default voice</label>
                <select
                  id="tts-default-voice"
                  class="tts-select"
                  value={settings.defaultVoice}
                  onChange={(event) => update({ defaultVoice: (event.currentTarget as HTMLSelectElement).value })}
                >
                  <option value="">Auto (first available)</option>
                  {voiceList.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              </div>
              <label class="tts-check">
                <input
                  type="checkbox"
                  checked={settings.randomVoice}
                  onChange={(event) => update({ randomVoice: (event.currentTarget as HTMLInputElement).checked })}
                />
                Random voice
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-speed">
                  Default speed <span class="tts-value">{settings.defaultSpeed.toFixed(2)}×</span>
                </label>
                <input
                  id="tts-speed"
                  class="tts-range"
                  type="range"
                  min={TTS_LIMITS.minSpeed}
                  max={TTS_LIMITS.maxSpeed}
                  step="0.05"
                  value={settings.defaultSpeed}
                  disabled={TTS_SPEED_PITCH_UNSUPPORTED}
                  onInput={(event) => updateNumber('defaultSpeed', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-pitch">
                  Default pitch <span class="tts-value">{settings.defaultPitch.toFixed(2)}×</span>
                </label>
                <input
                  id="tts-pitch"
                  class="tts-range"
                  type="range"
                  min={TTS_LIMITS.minPitch}
                  max={TTS_LIMITS.maxPitch}
                  step="0.05"
                  value={settings.defaultPitch}
                  disabled={TTS_SPEED_PITCH_UNSUPPORTED}
                  onInput={(event) => updateNumber('defaultPitch', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
              {TTS_SPEED_PITCH_UNSUPPORTED && (
                <p class="tts-hint">{t(props.locale, 'ttsSpeedPitchNote')}</p>
              )}
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-volume">
                  Volume <span class="tts-value">{Math.round(settings.volume * 100)}%</span>
                </label>
                <input
                  id="tts-volume"
                  class="tts-range"
                  type="range"
                  min={TTS_LIMITS.minVolume}
                  max={TTS_LIMITS.maxVolume}
                  step="0.01"
                  value={settings.volume}
                  onInput={(event) => updateNumber('volume', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
            </section>

            <TtsAllowedUsers
              locale={props.locale}
              settings={settings}
              onSettingsChange={props.onSettingsChange}
            />

            <section class="tts-card">
              <h4 class="tts-card__title">Comment Types</h4>
              <label class="tts-check">
                <input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'any'} onChange={() => setCommentMode('any')} />
                Any comment
              </label>
              <label class="tts-check">
                <input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'dot'} onChange={() => setCommentMode('dot')} />
                Starts with `.`
              </label>
              <label class="tts-check">
                <input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'slash'} onChange={() => setCommentMode('slash')} />
                Starts with `/`
              </label>
              <label class="tts-check">
                <input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'command'} onChange={() => setCommentMode('command')} />
                Starts with custom command
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-command">Command</label>
                <input
                  id="tts-command"
                  class="tts-input"
                  type="text"
                  value={settings.command}
                  maxlength={TTS_LIMITS.maxCommandLength}
                  disabled={settings.commentMode !== 'command'}
                  placeholder="!tts"
                  onInput={(event) => update({ command: (event.currentTarget as HTMLInputElement).value.slice(0, TTS_LIMITS.maxCommandLength) || '!tts' })}
                />
              </div>
              <label class="tts-check">
                <input type="checkbox" checked={settings.stripCommand} onChange={(event) => update({ stripCommand: (event.currentTarget as HTMLInputElement).checked })} />
                Strip command prefix before speaking
              </label>
            </section>

            <section class="tts-card">
              <h4 class="tts-card__title">Charge Points</h4>
              <label class="tts-check">
                <input type="radio" name="tts-points-mode" checked={!settings.chargePoints} onChange={() => update({ chargePoints: false })} />
                Free
              </label>
              <label class="tts-check">
                <input type="radio" name="tts-points-mode" checked={settings.chargePoints} onChange={() => update({ chargePoints: true })} />
                Charge points per message
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-points-cost">Points cost</label>
                <input
                  id="tts-points-cost"
                  class="tts-input"
                  type="number"
                  min={TTS_LIMITS.minPointsCost}
                  max={TTS_LIMITS.maxPointsCost}
                  value={settings.pointsCost}
                  disabled={!settings.chargePoints}
                  onInput={(event) => updateNumber('pointsCost', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
              <p class="tts-hint">{t(props.locale, 'ttsCostNote')}</p>
            </section>
          </div>

          <div class="tts-grid tts-grid--bottom">
            <TtsSpecialUsers
              locale={props.locale}
              settings={settings}
              voices={props.voices}
              onSettingsChange={props.onSettingsChange}
            />

            <TtsVoiceTester
              locale={props.locale}
              voices={props.voices}
              voicesError={props.voicesError}
              speaking={props.speaking}
              logs={props.logs}
              defaultVoice={settings.defaultVoice}
              onSpeak={props.onSpeak}
              onRefreshVoices={props.onRefreshVoices}
            />
          </div>
        </div>
      );
    };
  },
);

export default TtsSettingsPanel;
</script>
