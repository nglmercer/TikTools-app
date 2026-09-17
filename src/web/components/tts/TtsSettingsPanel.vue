<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { ActionOptionItem } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import {
  TTS_LIMITS,
  TTS_SPEED_PITCH_UNSUPPORTED,
  clampNumber,
  normalizeHandle,
  type TtsCommentMode,
  type TtsLogEntry,
  type TtsSettings,
} from '../../tts/tts-policy.ts';
import { outputOptions, outputsCardState } from './tts-outputs.ts';

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

function voiceOptions(voices: ActionOptionItem[], current: string): Array<{ value: string; label: string }> {
  const options = voices.map((voice) => ({ value: voice.value, label: voice.label || voice.value }));
  if (current && !options.some((option) => option.value === current)) {
    options.unshift({ value: current, label: current });
  }
  return options;
}

/**
 * Host-owned TTS settings panel for `tts` plugin page sections. Renders
 * only host controls over manifest-declared voice sources; no
 * plugin-provided markup or script is involved.
 */
export const TtsSettingsPanel = defineVueComponent<TtsSettingsPanelProps>(
  ['locale', 'settings', 'voices', 'voicesError', 'speaking', 'logs', 'onSettingsChange', 'onRefreshVoices', 'onSpeak', 'outputsSupported', 'outputs', 'outputsSelected', 'outputsError', 'outputsPending', 'outputError', 'onSelectOutput', 'onRefreshOutputs'],
  (props) => {
    const testerText = ref('Hello TikTok, this is a voice test.');
    const testerVoice = ref(props.settings.defaultVoice);
    const newSpecialHandle = ref('');
    const newAllowedHandle = ref('');
    let testerVoiceTouched = false;

    watch(() => props.settings.defaultVoice, (voice) => {
      if (!testerVoiceTouched) testerVoice.value = voice;
    });

    const update = (patch: Partial<TtsSettings>): void => {
      props.onSettingsChange({ ...props.settings, ...patch });
    };

    const updateNumber = (key: 'defaultSpeed' | 'defaultPitch' | 'volume' | 'pointsCost' | 'minTeamLevel' | 'topGifterCount', raw: string): void => {
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
        case 'minTeamLevel':
          update({ minTeamLevel: Math.round(clampNumber(value, TTS_LIMITS.minTeamLevel, TTS_LIMITS.maxTeamLevel, 0)) });
          break;
        case 'topGifterCount':
          update({ topGifterCount: Math.round(clampNumber(value, TTS_LIMITS.minTopGifterCount, TTS_LIMITS.maxTopGifterCount, 10)) });
          break;
      }
    };

    const addSpecialUser = (): void => {
      const handle = normalizeHandle(newSpecialHandle.value);
      if (!handle || props.settings.specialUsers.some((entry) => entry.handle === handle)) return;
      update({
        specialUsers: [
          ...props.settings.specialUsers,
          { handle, allowed: true, voice: '', speed: 1, pitch: 1 },
        ].slice(0, TTS_LIMITS.maxSpecialUsers),
      });
      newSpecialHandle.value = '';
    };

    const patchSpecialUser = (handle: string, patch: Partial<{ allowed: boolean; voice: string; speed: number; pitch: number }>): void => {
      update({
        specialUsers: props.settings.specialUsers.map((entry) =>
          entry.handle === handle ? { ...entry, ...patch } : entry,
        ),
      });
    };

    const removeSpecialUser = (handle: string): void => {
      update({ specialUsers: props.settings.specialUsers.filter((entry) => entry.handle !== handle) });
    };

    const addAllowedUser = (): void => {
      const handle = normalizeHandle(newAllowedHandle.value);
      if (!handle || props.settings.allowedUsers.includes(handle)) return;
      update({ allowedUsers: [...props.settings.allowedUsers, handle].slice(0, TTS_LIMITS.maxAllowedUsers) });
      newAllowedHandle.value = '';
    };

    const removeAllowedUser = (handle: string): void => {
      update({ allowedUsers: props.settings.allowedUsers.filter((entry) => entry !== handle) });
    };

    const setCommentMode = (mode: TtsCommentMode): void => {
      update({ commentMode: mode });
    };

    const speak = (): void => {
      const text = testerText.value.trim();
      if (!text || props.speaking) return;
      props.onSpeak(text, testerVoice.value.trim());
    };

    const formatTime = (at: number): string => {
      try {
        return new Date(at).toLocaleTimeString();
      } catch {
        return '';
      }
    };

    /**
     * Server-side audio output selector. The select always shows server
     * state: the live selection while a switch is in flight, else the
     * server-reported active output. Failures never persist locally — the
     * selector falls back to the last confirmed server value.
     */
    const renderOutputsCard = () => {
      const locale = props.locale;
      const state = outputsCardState({
        supported: props.outputsSupported,
        outputs: props.outputs,
        outputsError: props.outputsError,
      });
      if (state.kind === 'hidden') return null;
      const refresh = () => props.onRefreshOutputs?.();
      const refreshRow = (
        <div class="tts-row">
          <button type="button" class="plg-btn plg-btn--sm" onClick={refresh}>
            {t(locale, 'ttsRefreshOutputs')}
          </button>
        </div>
      );
      if (state.kind === 'loading') {
        return (
          <section class="tts-card">
            <h4 class="tts-card__title">{t(locale, 'ttsAudioOutput')}</h4>
            <span class="plg-group-note">{t(locale, 'ttsAudioOutputsLoading')}</span>
          </section>
        );
      }
      if (state.kind === 'unavailable') {
        return (
          <section class="tts-card">
            <h4 class="tts-card__title">{t(locale, 'ttsAudioOutput')}</h4>
            <div class="plg-alert" role="status">
              {state.unsupported ? t(locale, 'ttsAudioOutputsNoPlayback') : t(locale, 'ttsAudioOutputsUnavailable')}
            </div>
            {!state.unsupported && props.outputsError && (
              <p class="tts-hint">{props.outputsError}</p>
            )}
            {refreshRow}
          </section>
        );
      }
      if (state.kind === 'empty') {
        return (
          <section class="tts-card">
            <h4 class="tts-card__title">{t(locale, 'ttsAudioOutput')}</h4>
            <span class="plg-group-note">{t(locale, 'ttsAudioOutputsEmpty')}</span>
            {refreshRow}
          </section>
        );
      }
      const current = props.outputsPending || props.outputsSelected || '';
      const rows = outputOptions(props.outputs ?? [], current);
      const switching = !!props.outputsPending;
      return (
        <section class="tts-card">
          <h4 class="tts-card__title">{t(locale, 'ttsAudioOutput')}</h4>
          {props.outputError && <div class="plg-alert" role="status">{props.outputError}</div>}
          <div class="tts-row tts-row--stack">
            <label class="tts-label" for="tts-audio-output">{t(locale, 'ttsAudioOutput')}</label>
            <select
              id="tts-audio-output"
              class="tts-select"
              value={current}
              disabled={switching}
              onChange={(event) => props.onSelectOutput?.((event.currentTarget as HTMLSelectElement).value)}
            >
              {current === '' && <option value="">{t(locale, 'ttsAudioOutputChoose')}</option>}
              {rows.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          </div>
          {switching
            ? <p class="tts-hint">{t(locale, 'ttsAudioOutputSwitching')}</p>
            : <p class="tts-hint">{t(locale, 'ttsAudioOutputHint')}</p>}
          {refreshRow}
        </section>
      );
    };

    return () => {
      const settings = props.settings;
      const voiceList = voiceOptions(props.voices, settings.defaultVoice);
      const testerVoices = voiceOptions(props.voices, testerVoice.value);

      return (
        <div>
          <div class="tts-grid tts-grid--top">
            {renderOutputsCard()}
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
                <p class="tts-hint">Speed and pitch are stored for later. The current SonicBoom server does not apply them to synthesis.</p>
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

            <section class="tts-card">
              <h4 class="tts-card__title">Allowed Users</h4>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowAllUsers} onChange={(event) => update({ allowAllUsers: (event.currentTarget as HTMLInputElement).checked })} />
                All users
              </label>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowFollowers} onChange={(event) => update({ allowFollowers: (event.currentTarget as HTMLInputElement).checked })} />
                Followers
              </label>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowSubscribers} onChange={(event) => update({ allowSubscribers: (event.currentTarget as HTMLInputElement).checked })} />
                Subscribers
              </label>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowModerators} onChange={(event) => update({ allowModerators: (event.currentTarget as HTMLInputElement).checked })} />
                Moderators
              </label>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowTeamMembers} onChange={(event) => update({ allowTeamMembers: (event.currentTarget as HTMLInputElement).checked })} />
                Team members
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-team-level">Minimum team level</label>
                <input
                  id="tts-team-level"
                  class="tts-input"
                  type="number"
                  min={TTS_LIMITS.minTeamLevel}
                  max={TTS_LIMITS.maxTeamLevel}
                  value={settings.minTeamLevel}
                  disabled={!settings.allowTeamMembers}
                  onInput={(event) => updateNumber('minTeamLevel', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowTopGifters} onChange={(event) => update({ allowTopGifters: (event.currentTarget as HTMLInputElement).checked })} />
                Top gifters
              </label>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-top-n">Top N</label>
                <input
                  id="tts-top-n"
                  class="tts-input"
                  type="number"
                  min={TTS_LIMITS.minTopGifterCount}
                  max={TTS_LIMITS.maxTopGifterCount}
                  value={settings.topGifterCount}
                  disabled={!settings.allowTopGifters}
                  onInput={(event) => updateNumber('topGifterCount', (event.currentTarget as HTMLInputElement).value)}
                />
              </div>
              <p class="tts-hint">Follower, moderator, team, and top-gifter rules apply only when the host supplies authoritative role data.</p>
              <label class="tts-check">
                <input type="checkbox" checked={settings.allowListedUsers} onChange={(event) => update({ allowListedUsers: (event.currentTarget as HTMLInputElement).checked })} />
                Allowed users list
              </label>
              <div class="tts-add-row">
                <input
                  class="tts-input"
                  type="text"
                  placeholder="@handle"
                  value={newAllowedHandle.value}
                  onInput={(event) => { newAllowedHandle.value = (event.currentTarget as HTMLInputElement).value; }}
                  onKeydown={(event) => { if ((event as KeyboardEvent).key === 'Enter') addAllowedUser(); }}
                />
                <button type="button" class="plg-btn plg-btn--sm" onClick={addAllowedUser}>Add</button>
              </div>
              {settings.allowedUsers.length > 0 && (
                <div class="tts-table-wrap">
                  <table class="tts-table">
                    <tbody>
                      {settings.allowedUsers.map((handle) => (
                        <tr key={handle}>
                          <td>@{handle}</td>
                          <td style="width: 64px; text-align: right;">
                            <button type="button" class="plg-btn plg-btn--sm plg-btn--danger" onClick={() => removeAllowedUser(handle)}>Remove</button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>

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
              <p class="tts-hint">Messages are rejected when the viewer cannot cover the cost. Each spoken message is charged exactly once.</p>
            </section>
          </div>

          <div class="tts-grid tts-grid--bottom">
            <section class="tts-card">
              <h4 class="tts-card__title">Special Users</h4>
              <p class="tts-hint">Special-user settings override global defaults. Blocked users never speak, even when all users are allowed.</p>
              <div class="tts-add-row">
                <input
                  class="tts-input"
                  type="text"
                  placeholder="@handle"
                  value={newSpecialHandle.value}
                  onInput={(event) => { newSpecialHandle.value = (event.currentTarget as HTMLInputElement).value; }}
                  onKeydown={(event) => { if ((event as KeyboardEvent).key === 'Enter') addSpecialUser(); }}
                />
                <button type="button" class="plg-btn plg-btn--sm" onClick={addSpecialUser}>Add</button>
              </div>
              {settings.specialUsers.length === 0 ? (
                <span class="plg-group-note">No special users yet.</span>
              ) : (
                <div class="tts-table-wrap">
                  <table class="tts-table">
                    <thead>
                      <tr>
                        <th>Handle</th>
                        <th>Allowed</th>
                        <th>Voice</th>
                        <th>Speed</th>
                        <th>Pitch</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      {settings.specialUsers.map((entry) => (
                        <tr key={entry.handle}>
                          <td>@{entry.handle}</td>
                          <td>
                            <input
                              type="checkbox"
                              checked={entry.allowed}
                              onChange={(event) => patchSpecialUser(entry.handle, { allowed: (event.currentTarget as HTMLInputElement).checked })}
                            />
                          </td>
                          <td>
                            <select
                              class="tts-select"
                              value={entry.voice}
                              onChange={(event) => patchSpecialUser(entry.handle, { voice: (event.currentTarget as HTMLSelectElement).value })}
                            >
                              <option value="">Default</option>
                              {voiceList.map((option) => (
                                <option key={option.value} value={option.value}>{option.label}</option>
                              ))}
                            </select>
                          </td>
                          <td>
                            <input
                              class="tts-input"
                              style="width: 72px;"
                              type="number"
                              min={TTS_LIMITS.minSpeed}
                              max={TTS_LIMITS.maxSpeed}
                              step="0.05"
                              value={entry.speed}
                              disabled={TTS_SPEED_PITCH_UNSUPPORTED}
                              onInput={(event) => {
                                const value = Number((event.currentTarget as HTMLInputElement).value);
                                if (Number.isFinite(value)) patchSpecialUser(entry.handle, { speed: clampNumber(value, TTS_LIMITS.minSpeed, TTS_LIMITS.maxSpeed, 1) });
                              }}
                            />
                          </td>
                          <td>
                            <input
                              class="tts-input"
                              style="width: 72px;"
                              type="number"
                              min={TTS_LIMITS.minPitch}
                              max={TTS_LIMITS.maxPitch}
                              step="0.05"
                              value={entry.pitch}
                              disabled={TTS_SPEED_PITCH_UNSUPPORTED}
                              onInput={(event) => {
                                const value = Number((event.currentTarget as HTMLInputElement).value);
                                if (Number.isFinite(value)) patchSpecialUser(entry.handle, { pitch: clampNumber(value, TTS_LIMITS.minPitch, TTS_LIMITS.maxPitch, 1) });
                              }}
                            />
                          </td>
                          <td style="text-align: right;">
                            <button type="button" class="plg-btn plg-btn--sm plg-btn--danger" onClick={() => removeSpecialUser(entry.handle)}>Remove</button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>

            <section class="tts-card">
              <h4 class="tts-card__title">Voice Tester</h4>
              {props.voicesError && <div class="plg-alert" role="status">{props.voicesError}</div>}
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-tester-voice">Voice</label>
                <select
                  id="tts-tester-voice"
                  class="tts-select"
                  value={testerVoice.value}
                  onChange={(event) => { testerVoiceTouched = true; testerVoice.value = (event.currentTarget as HTMLSelectElement).value; }}
                >
                  <option value="">Auto (default voice)</option>
                  {testerVoices.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              </div>
              <div class="tts-row tts-row--stack">
                <label class="tts-label" for="tts-tester-text">Text</label>
                <textarea
                  id="tts-tester-text"
                  class="tts-textarea"
                  value={testerText.value}
                  maxlength={TTS_LIMITS.maxCommentLength}
                  onInput={(event) => { testerText.value = (event.currentTarget as HTMLTextAreaElement).value; }}
                />
              </div>
              <div class="tts-row">
                <button
                  type="button"
                  class="plg-btn plg-btn--primary plg-btn--sm"
                  disabled={props.speaking || !testerText.value.trim()}
                  onClick={speak}
                >
                  {props.speaking ? 'Speaking…' : 'Play'}
                </button>
                <button type="button" class="plg-btn plg-btn--sm" onClick={props.onRefreshVoices}>
                  Refresh voices
                </button>
                <span class="tts-pill">{props.voices.length} voices</span>
              </div>
              <p class="tts-hint">401/403 errors mean the server rejected the credentials — open Connection and use Get API token with your admin login to mint one. Each log line shows the exact request path and whether a credential was attached (never the value itself).</p>
              <h4 class="tts-card__title">TTS logs</h4>
              {props.logs.length === 0 ? (
                <span class="plg-group-note">Nothing spoken yet.</span>
              ) : (
                <div class="tts-log" role="log">
                  {props.logs.map((entry) => (
                    <span key={entry.id} class={entry.ok ? 'tts-log__line--ok' : 'tts-log__line--err'}>
                      [{formatTime(entry.at)}] [{entry.source}] {entry.ok ? 'ok' : 'error'} {entry.voice ? `voice=${entry.voice} ` : ''}{entry.summary}
                    </span>
                  ))}
                </div>
              )}
            </section>
          </div>
        </div>
      );
    };
  },
);

export default TtsSettingsPanel;
</script>
