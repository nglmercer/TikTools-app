<script lang="tsx">
import { inject, type InjectionKey } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { TtsSettingsNode as TtsSettingsNodeType } from '../../../plugin-ui/index.ts';
import { outputsTarget } from './tts-outputs.ts';
import { i18nText, t } from '../../i18n.ts';
import type { TtsLogEntry, TtsSettings } from '../../tts/tts-policy.ts';
import type { PluginUiContext } from '../../plugin-ui/PluginUiContext.ts';
import { TtsSettingsPanel } from './TtsSettingsPanel.vue';

/**
 * Domain state for `tts-settings` nodes, provided by the composition root
 * (the legacy page wrapper today). The generic renderer never provides or
 * consumes this — it only renders the injected component.
 */
export interface TtsNodeState {
  settingsFor: (pluginId: string) => TtsSettings | undefined;
  speakingFor: (pluginId: string) => boolean;
  logsFor: (pluginId: string) => TtsLogEntry[];
  voicesFor: (contributionId: string) => {
    source: string;
    actionType: string;
    outputsSource?: string;
  };
  onSettingsChange: (pluginId: string, next: TtsSettings) => void;
  onSpeak: (pluginId: string, actionType: string, text: string, voice: string) => void;
  onOutputSelect?: (
    pluginId: string,
    actionType: string,
    field: string,
    device: string,
    source: string,
  ) => void;
  outputPendingFor: (pluginId: string) => string | undefined;
  outputErrorFor: (pluginId: string) => string | undefined;
}

export const TtsNodeStateKey: InjectionKey<TtsNodeState> = Symbol('tts-node-state');

type TtsSettingsNodeProps = {
  node: TtsSettingsNodeType;
  context: PluginUiContext;
  nodeKey: string;
};

/**
 * Host-owned adapter binding a `tts-settings` declarative node to the TTS
 * domain panel. Registered by the composition root in
 * `context.customNodes` — never imported by the generic renderer.
 */
export const TtsSettingsNode = defineVueComponent<TtsSettingsNodeProps>(
  ['node', 'context', 'nodeKey'],
  (props) => {
    return () => {
      const context = props.context;
      const locale = context.locale;
      const pluginId = context.pluginId;
      const state = inject(TtsNodeStateKey, undefined);
      const title = props.node.title ? i18nText(locale, props.node.title) : '';
      if (!state) {
        return (
          <section class="plg-stack">
            <span class="plg-group-note">{t(locale, 'pluginListEmpty')}</span>
          </section>
        );
      }
      const binding = state.voicesFor(props.node.contribution);
      const settings = state.settingsFor(pluginId);
      if (!binding.actionType || !settings) {
        return (
          <section class="plg-stack">
            <span class="plg-group-note">{t(locale, 'pluginListEmpty')}</span>
          </section>
        );
      }
      const options = binding.source ? (context.options.values[binding.source] ?? []) : [];
      const voicesError = binding.source ? context.options.errors[binding.source] : undefined;
      const outputsSource = binding.outputsSource;
      const outputTarget = outputsSource ? outputsTarget(outputsSource) : undefined;
      const onOutputSelect = state.onOutputSelect;
      return (
        <section>
          {title && (
            <h3 class="plg-topbar__title" style="padding: 0 16px;">
              {title}
            </h3>
          )}
          <TtsSettingsPanel
            locale={locale}
            settings={settings}
            voices={options}
            voicesError={voicesError}
            speaking={state.speakingFor(pluginId)}
            logs={state.logsFor(pluginId)}
            onSettingsChange={(next) => state.onSettingsChange(pluginId, next)}
            onRefreshVoices={() => {
              if (binding.source) context.options.get(binding.source, true);
            }}
            onSpeak={(text, voice) => state.onSpeak(pluginId, binding.actionType, text, voice)}
            outputsSupported={!!outputsSource}
            outputs={outputsSource ? context.options.values[outputsSource] : undefined}
            outputsSelected={outputsSource ? context.options.selected[outputsSource] : undefined}
            outputsError={outputsSource ? context.options.errors[outputsSource] : undefined}
            outputsPending={state.outputPendingFor(pluginId)}
            outputError={state.outputErrorFor(pluginId)}
            onSelectOutput={
              outputTarget && onOutputSelect && outputsSource
                ? (device) =>
                    onOutputSelect(
                      pluginId,
                      outputTarget.actionType,
                      outputTarget.field,
                      device,
                      outputsSource,
                    )
                : undefined
            }
            onRefreshOutputs={
              outputsSource ? () => context.options.get(outputsSource, true) : undefined
            }
          />
        </section>
      );
    };
  },
);

export default TtsSettingsNode;
</script>
