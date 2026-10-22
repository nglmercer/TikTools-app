<script lang="tsx">
import TtsIcon from './TtsIcon.vue';
import TtsToggle from './TtsToggle.vue';
import { defineVueComponent } from '../vue/component.ts';
import type { ActionOptionItem } from '../types.ts';
import { t, type Locale } from '../i18n/index.ts';
import {
  TTS_LIMITS,
  TTS_SPEED_PITCH_UNSUPPORTED,
  clampNumber,
  type TtsCommentMode,
  type TtsLogEntry,
  type TtsSettings,
} from '../../../shared/tts/tts-policy.ts';
import { voiceOptions } from './tts-voice-options.ts';
import { TtsAllowedUsers } from './TtsAllowedUsers.vue';
import { TtsOutputsCard } from './TtsOutputsCard.vue';
import { TtsSpecialUsers } from './TtsSpecialUsers.vue';
import { TtsTextProcessing } from './TtsTextProcessing.vue';
import { TtsVoiceTester } from './TtsVoiceTester.vue';

type TtsSettingsPanelProps = {
  locale: Locale;
  settings: TtsSettings;
  voices: ActionOptionItem[];
  allVoices?: ActionOptionItem[];
  providers?: ActionOptionItem[];
  refreshing?: boolean;
  catalogStatus?: string;
  providerReady?: boolean;
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
 * SonicBoom TTS settings panel. Runs inside the isolated plugin UI (never
 * the main TikTools document) and talks to the host only through the
 * restricted broker (`settings.*`, `actions.*`, `options.*`).
 */
export const TtsSettingsPanel = defineVueComponent<TtsSettingsPanelProps>(
  ['allVoices', 'providers', 'refreshing', 'catalogStatus', 'providerReady', 'locale', 'settings', 'voices', 'voicesError', 'speaking', 'logs', 'onSettingsChange', 'onRefreshVoices', 'onSpeak', 'outputsSupported', 'outputs', 'outputsSelected', 'outputsError', 'outputsPending', 'outputError', 'onSelectOutput', 'onRefreshOutputs'],
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
      const locale = props.locale;
      const voiceList = voiceOptions(props.voices);

      return (
        <div>
          <div class="tts-grid tts-grid--top">
            <section class="tts-card">
              <h4 class="tts-card__title"><TtsIcon name="voice"/>{t(locale, 'ttsGeneralSettings')}</h4>
              <TtsToggle label={t(locale, 'ttsEnabled')} hint={t(locale, 'ttsEnabled')} checked={settings.enabled} onChange={(checked: boolean) => update({ enabled: checked })}/>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-language">{t(locale, 'ttsLanguage')}</label>
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
              {(props.providers?.length ?? 0) > 1 && <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-provider">{t(locale, 'ttsProvider')}</label>
                <select id="tts-provider" class="tts-select" value={settings.providerId} onChange={event => update({ providerId: (event.currentTarget as HTMLSelectElement).value })}>
                  {!props.providers?.some(p => p.value === settings.providerId) && <option value={settings.providerId} disabled>{settings.providerId} — {t(locale, 'ttsProviderUnavailable')}</option>}
                  {props.providers?.map(p => <option key={p.value} value={p.value} disabled={p.disabled}>{p.label.replace('(loading)', `(${t(locale, 'ttsProviderLoading')})`).replace('(not_installed)', `(${t(locale, 'ttsProviderMissing')})`).replace('(error)', `(${t(locale, 'ttsProviderError')})`)}</option>)}
                </select>
              </div>}
              {props.providerReady === false && <p role="status">{t(locale, 'ttsProviderUnavailable')} ({settings.providerId})</p>}
              {props.catalogStatus && <p role="status">{props.catalogStatus}</p>}
              {props.voices.length === 0 && <p role="status">{t(locale, 'ttsNoProviderVoices')}</p>}
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-default-voice">{t(locale, 'ttsDefaultVoice')}</label>
                <select
                  id="tts-default-voice"
                  class="tts-select"
                  value={settings.defaultVoice}
                  onChange={(event) => update({ defaultVoice: (event.currentTarget as HTMLSelectElement).value })}
                >
                  <option value="">{t(locale, 'ttsAutoFirstAvailable')}</option>
                  {voiceList.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              </div>
              <TtsToggle label={t(locale, 'ttsRandomVoice')} hint={t(locale, 'ttsRandomHint')} checked={settings.randomVoice} onChange={(checked: boolean) => update({ randomVoice: checked })}/>
              <details class="tts-subgroup" title={t(locale, 'ttsSpeedPitchNote')}><summary><TtsIcon name="sliders"/>{t(locale, 'ttsVoiceAdvanced')}<TtsIcon name="chevron"/></summary><div class="tts-subgroup__body"><div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-speed">
                  {t(locale, 'ttsDefaultSpeed')} <span class="tts-value">{settings.defaultSpeed.toFixed(2)}×</span>
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
                  {t(locale, 'ttsDefaultPitch')} <span class="tts-value">{settings.defaultPitch.toFixed(2)}×</span>
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
                <p class="tts-hint">{t(locale, 'ttsSpeedPitchNote')}</p>
              )}
              </div></details><div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-volume">
                  {t(locale, 'ttsVolume')} <span class="tts-value">{Math.round(settings.volume * 100)}%</span>
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

            <TtsVoiceTester
              refreshing={props.refreshing}
              unavailable={props.providerReady === false || props.voices.length === 0}
              locale={locale}
              voices={props.voices}
              voicesError={props.voicesError}
              speaking={props.speaking}
              logs={props.logs}
              defaultVoice={settings.defaultVoice}
              onSpeak={props.onSpeak}
              onRefreshVoices={props.onRefreshVoices}
            />
            <TtsOutputsCard
              locale={locale}
              outputsSupported={props.outputsSupported}
              outputs={props.outputs}
              outputsError={props.outputsError}
              outputsSelected={props.outputsSelected}
              outputsPending={props.outputsPending}
              outputError={props.outputError}
              onSelectOutput={props.onSelectOutput}
              onRefreshOutputs={props.onRefreshOutputs}
            />
            <TtsAllowedUsers
              locale={locale}
              settings={settings}
              onSettingsChange={props.onSettingsChange}
            />

            <section class="tts-card">
              <h4 class="tts-card__title"><TtsIcon name="message"/>{t(locale, 'ttsCommentTypes')}</h4>
              <label class="tts-check" title={t(locale, 'ttsAnyComment')}>
<input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'any'} onChange={() => setCommentMode('any')} />
                {t(locale, 'ttsAnyComment')}
              </label>
              <label class="tts-check" title={t(locale, 'ttsStartsWithDot')}>
<input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'dot'} onChange={() => setCommentMode('dot')} />
                {t(locale, 'ttsStartsWithDot')}
              </label>
              <label class="tts-check" title={t(locale, 'ttsStartsWithSlash')}>
<input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'slash'} onChange={() => setCommentMode('slash')} />
                {t(locale, 'ttsStartsWithSlash')}
              </label>
              <label class="tts-check" title={t(locale, 'ttsStartsWithCommand')}>
<input type="radio" name="tts-comment-mode" checked={settings.commentMode === 'command'} onChange={() => setCommentMode('command')} />
                {t(locale, 'ttsStartsWithCommand')}
              </label>
              {settings.commentMode === 'command' && (<div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-command">{t(locale, 'ttsCommand')}</label>
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
              </div>)}
              {settings.commentMode !== 'any' && <TtsToggle label={t(locale, 'ttsStripCommand')} hint={t(locale, 'ttsStripCommand')} checked={settings.stripCommand} onChange={(checked: boolean) => update({ stripCommand: checked })}/>}
            </section>

            <TtsTextProcessing
              locale={locale}
              settings={settings}
              onSettingsChange={props.onSettingsChange}
            />

            <section class="tts-card">
              <h4 class="tts-card__title"><TtsIcon name="coins"/>{t(locale, 'ttsChargePoints')}</h4>
              <label class="tts-check" title={t(locale, 'ttsFree')}>
<input type="radio" name="tts-points-mode" checked={!settings.chargePoints} onChange={() => update({ chargePoints: false })} />
                {t(locale, 'ttsFree')}
              </label>
              <label class="tts-check" title={t(locale, 'ttsChargePerMessage')}>
<input type="radio" name="tts-points-mode" checked={settings.chargePoints} onChange={() => update({ chargePoints: true })} />
                {t(locale, 'ttsChargePerMessage')}
              </label>
              {settings.chargePoints && (<div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-points-cost">{t(locale, 'ttsPointsCost')}</label>
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
              </div>)}
              <p class="tts-hint">{t(locale, 'ttsCostNote')}</p>
            </section>
          </div>

          <div class="tts-grid tts-grid--bottom">
            <TtsSpecialUsers
              locale={locale}
              settings={settings}
              voices={props.allVoices ?? props.voices}
              providers={props.providers}
              onSettingsChange={props.onSettingsChange}
            />


          </div>
        </div>
      );
    };
  },
);

export default TtsSettingsPanel;
</script>
