<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { ActionOptionItem } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import {
  TTS_LIMITS,
  type TtsLogEntry,
} from '../../tts/tts-policy.ts';
import { voiceOptions } from './tts-voice-options.ts';

type TtsVoiceTesterProps = {
  locale: Locale;
  voices: ActionOptionItem[];
  voicesError?: string;
  speaking: boolean;
  logs: TtsLogEntry[];
  defaultVoice: string;
  onSpeak: (text: string, voice: string) => void;
  onRefreshVoices: () => void;
};

/** Voice tester card with spoken-message log. The tester voice follows the default voice until touched. */
export const TtsVoiceTester = defineVueComponent<TtsVoiceTesterProps>(
  ['locale', 'voices', 'voicesError', 'speaking', 'logs', 'defaultVoice', 'onSpeak', 'onRefreshVoices'],
  (props) => {
  const testerText = ref('Hello TikTok, this is a voice test.');
  const testerVoice = ref(props.defaultVoice);
  let testerVoiceTouched = false;

  watch(() => props.defaultVoice, (voice) => {
    if (!testerVoiceTouched) testerVoice.value = voice;
  });

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

  return () => {
  const testerVoices = voiceOptions(props.voices, testerVoice.value);
  return (
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
      <p class="tts-hint">{t(props.locale, 'ttsAuthNote')}</p>
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
  );
  };
  },
);

export default TtsVoiceTester;
</script>
