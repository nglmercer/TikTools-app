<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import { t, type Locale } from '../../i18n.ts';
import {
  TTS_LIMITS,
  clampNumber,
  normalizeHandle,
  type TtsSettings,
} from '../../tts/tts-policy.ts';

type TtsAllowedUsersProps = {
  locale: Locale;
  settings: TtsSettings;
  onSettingsChange: (next: TtsSettings) => void;
};

/** Allowed-users allowlist card: role toggles, numeric bounds, and the handle list. */
export const TtsAllowedUsers = defineVueComponent<TtsAllowedUsersProps>(
  ['locale', 'settings', 'onSettingsChange'],
  (props) => {
  const newAllowedHandle = ref('');

  const update = (patch: Partial<TtsSettings>): void => {
    props.onSettingsChange({ ...props.settings, ...patch });
  };

  const updateNumber = (key: 'minTeamLevel' | 'topGifterCount', raw: string): void => {
    const value = Number(raw);
    if (!Number.isFinite(value)) return;
    switch (key) {
      case 'minTeamLevel':
        update({ minTeamLevel: Math.round(clampNumber(value, TTS_LIMITS.minTeamLevel, TTS_LIMITS.maxTeamLevel, 0)) });
        break;
      case 'topGifterCount':
        update({ topGifterCount: Math.round(clampNumber(value, TTS_LIMITS.minTopGifterCount, TTS_LIMITS.maxTopGifterCount, 10)) });
        break;
    }
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

  return () => {
  const settings = props.settings;
  const locale = props.locale;
  return (
    <section class="tts-card">
      <h4 class="tts-card__title">{t(locale, 'ttsAllowedUsers')}</h4>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowAllUsers} onChange={(event) => update({ allowAllUsers: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsAllUsers')}
      </label>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowFollowers} onChange={(event) => update({ allowFollowers: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsFollowers')}
      </label>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowSubscribers} onChange={(event) => update({ allowSubscribers: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsSubscribers')}
      </label>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowModerators} onChange={(event) => update({ allowModerators: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsModerators')}
      </label>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowTeamMembers} onChange={(event) => update({ allowTeamMembers: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsTeamMembers')}
      </label>
      <div class="tts-row tts-row--stack">
        <label class="tts-label" for="tts-team-level">{t(locale, 'ttsMinTeamLevel')}</label>
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
        {t(locale, 'ttsTopGifters')}
      </label>
      <div class="tts-row tts-row--stack">
        <label class="tts-label" for="tts-top-n">{t(locale, 'ttsTopN')}</label>
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
      <p class="tts-hint">{t(locale, 'ttsRolesNote')}</p>
      <label class="tts-check">
        <input type="checkbox" checked={settings.allowListedUsers} onChange={(event) => update({ allowListedUsers: (event.currentTarget as HTMLInputElement).checked })} />
        {t(locale, 'ttsAllowedUsersList')}
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
        <button type="button" class="plg-btn plg-btn--sm" onClick={addAllowedUser}>{t(locale, 'ttsAdd')}</button>
      </div>
      {settings.allowedUsers.length > 0 && (
        <div class="tts-table-wrap">
          <table class="tts-table">
            <tbody>
              {settings.allowedUsers.map((handle) => (
                <tr key={handle}>
                  <td>@{handle}</td>
                  <td style="width: 64px; text-align: right;">
                    <button type="button" class="plg-btn plg-btn--sm plg-btn--danger" onClick={() => removeAllowedUser(handle)}>{t(locale, 'ttsRemove')}</button>
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

export default TtsAllowedUsers;
</script>
