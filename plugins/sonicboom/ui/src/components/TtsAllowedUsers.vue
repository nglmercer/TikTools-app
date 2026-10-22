<script lang="tsx">
import { ref } from 'vue';
import TtsIcon from './TtsIcon.vue';
import TtsToggle from './TtsToggle.vue';
import { defineVueComponent } from '../vue/component.ts';

import { t, type Locale } from '../i18n/index.ts';
import {
  TTS_LIMITS,
  clampNumber,
  normalizeHandle,
  type TtsSettings,
} from '../../../shared/tts/tts-policy.ts';

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
      <h4 class="tts-card__title"><TtsIcon name="users"/>{t(locale, 'ttsAllowedUsers')}</h4>
      <TtsToggle label={t(locale, 'ttsAllUsers')} hint={t(locale, 'ttsAudienceHint')} checked={settings.allowAllUsers} onChange={(checked: boolean) => update({ allowAllUsers: checked })}/>
      {!settings.allowAllUsers && <div class="tts-dependent-group"><TtsToggle label={t(locale, 'ttsFollowers')} hint={t(locale, 'ttsFollowers')} checked={settings.allowFollowers} onChange={(checked: boolean) => update({ allowFollowers: checked })}/>
      <TtsToggle label={t(locale, 'ttsSubscribers')} hint={t(locale, 'ttsSubscribers')} checked={settings.allowSubscribers} onChange={(checked: boolean) => update({ allowSubscribers: checked })}/>
      <TtsToggle label={t(locale, 'ttsModerators')} hint={t(locale, 'ttsModerators')} checked={settings.allowModerators} onChange={(checked: boolean) => update({ allowModerators: checked })}/>
      <TtsToggle label={t(locale, 'ttsTeamMembers')} hint={t(locale, 'ttsTeamHint')} checked={settings.allowTeamMembers} onChange={(checked: boolean) => update({ allowTeamMembers: checked })}/>
      {settings.allowTeamMembers && (<div class="tts-row tts-row--stack">
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
      </div>)}
      <TtsToggle label={t(locale, 'ttsTopGifters')} hint={t(locale, 'ttsGifterHint')} checked={settings.allowTopGifters} onChange={(checked: boolean) => update({ allowTopGifters: checked })}/>
      {settings.allowTopGifters && (<div class="tts-row tts-row--stack">
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
      </div>)}
      <p class="tts-hint">{t(locale, 'ttsRolesNote')}</p>
      <TtsToggle label={t(locale, 'ttsAllowedUsersList')} hint={t(locale, 'ttsListHint')} checked={settings.allowListedUsers} onChange={(checked: boolean) => update({ allowListedUsers: checked })}/>
      {settings.allowListedUsers && <div class="tts-dependent-group"><div class="tts-add-row">
        <input
          class="tts-input"
          type="text"
          aria-label={t(locale, 'ttsAllowedUsersList')} placeholder="@handle"
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
      </div>}
      </div>}
      {settings.allowAllUsers && <p class="tts-hint">{t(locale, 'ttsAudienceHint')}</p>}
    </section>
  );
  };
  },
);

export default TtsAllowedUsers;
</script>
