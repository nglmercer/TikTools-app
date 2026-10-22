<script lang="tsx">
import { ref } from 'vue';
import TtsGroup from './TtsGroup.vue';
import TtsToggle from './TtsToggle.vue';
import { defineVueComponent } from '../vue/component.ts';

import { t, type Locale } from '../i18n/index.ts';
import {
  sanitizeTtsText,
  type TtsEmoteSpeechMode,
  type TtsSettings,
  type TtsTextSanitizerSettings,
} from '../../../shared/tts/tts-policy.ts';

type TtsTextProcessingProps = {
  locale: Locale;
  settings: TtsSettings;
  onSettingsChange: (next: TtsSettings) => void;
};

/**
 * Text Processing card: the emote/emoji/whitespace cleanup applied to the
 * permitted spoken text, plus a live preview. The preview calls the exact
 * `sanitizeTtsText` the speech policy uses, so what the operator sees here is
 * what the backend will speak — the two can never disagree.
 */
export const TtsTextProcessing = defineVueComponent<TtsTextProcessingProps>(
  ['locale', 'settings', 'onSettingsChange'],
  (props) => {
    const previewInput = ref('Hello [laughcry][laughcry] world');

    const update = (patch: Partial<TtsTextSanitizerSettings>): void => {
      props.onSettingsChange({
        ...props.settings,
        sanitizer: { ...props.settings.sanitizer, ...patch },
      });
    };

    return () => {
      const locale = props.locale;
      const sanitizer = props.settings.sanitizer;
      const preview = sanitizeTtsText(previewInput.value, sanitizer);

      return (
        <TtsGroup title={t(locale, 'ttsTextProcessing')} icon="sparkle" expanded={false}>
          <TtsToggle label={t(locale, 'ttsRemoveEmoteCodes')} hint={t(locale, 'ttsEmoteHint')} checked={sanitizer.removeKnownEmotes} onChange={(checked: boolean) => update({ removeKnownEmotes: checked })}/>
          <TtsToggle label={t(locale, 'ttsRemoveUnicodeEmoji')} hint={t(locale, 'ttsRemoveUnicodeEmoji')} checked={sanitizer.removeUnicodeEmoji} onChange={(checked: boolean) => update({ removeUnicodeEmoji: checked })}/>
          <TtsToggle label={t(locale, 'ttsRemoveInvisibleCharacters')} hint={t(locale, 'ttsRemoveInvisibleCharacters')} checked={sanitizer.removeInvisibleCharacters} onChange={(checked: boolean) => update({ removeInvisibleCharacters: checked })}/>
          <TtsToggle label={t(locale, 'ttsCollapseWhitespace')} hint={t(locale, 'ttsCollapseWhitespace')} checked={sanitizer.collapseWhitespace} onChange={(checked: boolean) => update({ collapseWhitespace: checked })}/>
          {sanitizer.removeKnownEmotes && (<div class="tts-row tts-row--stack">
            <label class="tts-label" for="tts-emote-mode">{t(locale, 'ttsEmoteHandling')}</label>
            <select
              id="tts-emote-mode"
              class="tts-select"
              value={sanitizer.emoteSpeechMode}
              disabled={!sanitizer.removeKnownEmotes}
              onChange={(event) => update({ emoteSpeechMode: (event.currentTarget as HTMLSelectElement).value as TtsEmoteSpeechMode })}
            >
              <option value="remove">{t(locale, 'ttsEmoteRemove')}</option>
              <option value="describe">{t(locale, 'ttsEmoteDescribe')}</option>
            </select>
          </div>)}
          <TtsToggle label={t(locale, 'ttsSkipEmptyMessages')} hint={t(locale, 'ttsSkipEmptyMessages')} checked={sanitizer.skipEmptyMessages} onChange={(checked: boolean) => update({ skipEmptyMessages: checked })}/>
          <p class="tts-hint">{t(locale, 'ttsTextProcessingNote')}</p>

          <div class="tts-preview">
            <label class="tts-label" for="tts-preview-input">{t(locale, 'ttsPreviewOriginal')}</label>
            <input
              id="tts-preview-input"
              class="tts-input"
              type="text"
              value={previewInput.value}
              onInput={(event) => { previewInput.value = (event.currentTarget as HTMLInputElement).value; }}
            />
            <p class="tts-preview__result">
              <span class="tts-value">{t(locale, 'ttsPreviewResult')}</span>{' '}
              {preview || t(locale, 'ttsPreviewEmpty')}
            </p>
          </div>
        </TtsGroup>
      );
    };
  },
);

export default TtsTextProcessing;
</script>
