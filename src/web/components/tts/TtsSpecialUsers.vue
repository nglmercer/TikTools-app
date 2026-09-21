<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { ActionOptionItem } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import {
  TTS_LIMITS,
  TTS_SPEED_PITCH_UNSUPPORTED,
  clampNumber,
  normalizeHandle,
  type TtsSettings,
} from '../../tts/tts-policy.ts';
import { voiceOptions } from './tts-voice-options.ts';

type TtsSpecialUsersProps = {
  locale: Locale;
  settings: TtsSettings;
  voices: ActionOptionItem[];
  onSettingsChange: (next: TtsSettings) => void;
};

/** Per-handle voice overrides card: add/patch/remove special users. */
export const TtsSpecialUsers = defineVueComponent<TtsSpecialUsersProps>(
  ['locale', 'settings', 'voices', 'onSettingsChange'],
  (props) => {
  const newSpecialHandle = ref('');

  const update = (patch: Partial<TtsSettings>): void => {
    props.onSettingsChange({ ...props.settings, ...patch });
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

  return () => {
  const settings = props.settings;
  const voiceList = voiceOptions(props.voices, settings.defaultVoice);
  return (
    <section class="tts-card">
      <h4 class="tts-card__title">Special Users</h4>
      <p class="tts-hint">{t(props.locale, 'ttsSpecialUsersNote')}</p>
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
  );
  };
  },
);

export default TtsSpecialUsers;
</script>
