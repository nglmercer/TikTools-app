<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';

import type { ActionOptionItem } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import { outputOptions, outputsCardState } from './tts-outputs.ts';

type TtsOutputsCardProps = {
  locale: Locale;
  /** False when the section declares no outputs source: the card hides. */
  outputsSupported: boolean;
  /** Server output options; undefined while the first fetch is in flight. */
  outputs?: ActionOptionItem[];
  /** Fetch failure; hides the selector but never the panel. */
  outputsError?: string;
  /** Server-reported active output; the authoritative selection. */
  outputsSelected?: string;
  /** Device currently being switched to; disables the selector. */
  outputsPending?: string;
  /** Last switch failure; the selector keeps showing server state. */
  outputError?: string;
  onSelectOutput?: (device: string) => void;
  onRefreshOutputs?: () => void;
};

/**
 * Server-side audio output selector. The select always shows server
 * state: the live selection while a switch is in flight, else the
 * server-reported active output. Failures never persist locally — the
 * selector falls back to the last confirmed server value.
 */
export const TtsOutputsCard = defineVueComponent<TtsOutputsCardProps>(
  ['locale', 'outputsSupported', 'outputs', 'outputsError', 'outputsSelected', 'outputsPending', 'outputError', 'onSelectOutput', 'onRefreshOutputs'],
  (props) => {
  return () => {
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
  },
);

export default TtsOutputsCard;
</script>
