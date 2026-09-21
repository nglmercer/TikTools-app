<script lang="tsx">
import { computed, onMounted, provide, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import type { JsonObject } from '../../automation/types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { adaptLegacyPage } from '../../plugin-ui/index.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { TtsSettingsNode, TtsNodeStateKey } from '../components/tts/TtsSettingsNode.vue';
import type { Locale } from '../i18n.ts';
import type { TtsLogEntry, TtsSettings } from '../tts/tts-policy.ts';
import type { PluginUiContext } from '../plugin-ui/PluginUiContext.ts';
import { PluginPage } from '../plugin-ui/PluginPage.vue';

type PluginPageViewProps = {
  locale: Locale;
  page: PluginPageDescriptor;
  pluginName: string;
  settingsState?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Record<string, ActionOptionItem[]>;
  actionOptionErrors: Record<string, string>;
  actionOptionSelected: Record<string, string>;
  ttsOutputPending?: string;
  ttsOutputError?: string;
  onTtsOutputSelect?: (pluginId: string, actionType: string, field: string, device: string, source: string) => void;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string, refresh?: boolean) => void;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onExecuteAction?: (actionType: string, config: Record<string, string | number | boolean>) => void;
  ttsSettings?: TtsSettings;
  ttsSpeaking?: boolean;
  ttsLogs?: TtsLogEntry[];
  onTtsSettingsChange?: (pluginId: string, next: TtsSettings) => void;
  onTtsSpeak?: (pluginId: string, actionType: string, text: string, voice: string) => void;
  supportsProvisioning?: boolean;
  provisionState?: { working: boolean; ok: boolean; message: string };
  onProvisionToken?: (id: string, username: string, password: string) => void;
};

/**
 * Backward-compatible wrapper for legacy schema-v3 plugin pages.
 *
 * The v3 descriptor is adapted once to the generic declarative contract
 * and rendered through the domain-free `<PluginPage>` + `<PluginNode>`
 * registry. The old per-kind switch (text/form/connection/list/tts) is
 * gone: generic nodes render via the registry, and the `tts-settings`
 * node resolves through the host-injected `customNodes` entry bound to
 * the TTS domain panel below. No other call site changes: props and
 * visuals are preserved.
 */
export const PluginPageView = defineVueComponent<PluginPageViewProps>(
  ['locale', 'page', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'actionOptionErrors', 'actionOptionSelected', 'ttsOutputPending', 'ttsOutputError', 'onTtsOutputSelect', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker', 'onExecuteAction', 'ttsSettings', 'ttsSpeaking', 'ttsLogs', 'onTtsSettingsChange', 'onTtsSpeak', 'supportsProvisioning', 'provisionState', 'onProvisionToken'],
  (props) => {
  const adapted = computed(() => adaptLegacyPage(props.page));
  const localState = ref<Record<string, string | number | boolean>>({});
  const drafts = ref<Record<string, JsonObject>>({});

  const context = computed<PluginUiContext>(() => ({
    locale: props.locale,
    pluginId: props.page.pluginId,
    pluginName: props.pluginName,
    settings: {
      state: props.settingsState,
      get: () => props.onGetSettings(props.page.pluginId),
      save: (values) => props.onSaveSettings(props.page.pluginId, values),
    },
    options: {
      values: props.actionOptions,
      errors: props.actionOptionErrors,
      selected: props.actionOptionSelected,
      get: (source, refresh) => props.onGetActionOptions(source, refresh),
    },
    actions: {
      execute: (actionType, config) => {
        if (props.onExecuteAction) props.onExecuteAction(actionType, config);
        else console.warn(`plugin-action ${actionType} has no executor in this host`);
      },
    },
    connection: {
      state: props.connection,
      test: () => props.onTestConnection(props.page.pluginId),
    },
    media: {
      pick: props.onOpenMediaPicker,
    },
    provisioning: {
      supported: props.supportsProvisioning ?? false,
      state: props.provisionState,
      provision: (username, password) =>
        props.onProvisionToken?.(props.page.pluginId, username, password),
    },
    local: {
      get: (name) => localState.value[name],
      set: (name, value) => {
        localState.value = { ...localState.value, [name]: value };
      },
    },
    customNodes: {
      'tts-settings': TtsSettingsNode,
    },
    formDrafts: {
      get: (key) => drafts.value[key],
      set: (key, value) => {
        drafts.value = { ...drafts.value, [key]: value };
      },
      keys: () => Object.keys(drafts.value),
      clear: () => {
        drafts.value = {};
      },
    },
  }));

  // TTS option sources live on the generated contributions (not on the
  // generic `tts-settings` node), so the wrapper fetches them alongside
  // the generic page fetch. Form dynamic-field sources are fetched by the
  // generic page itself.
  const requestTtsSources = (): void => {
    for (const contribution of adapted.value.tts) {
      props.onGetActionOptions(contribution.voicesFrom);
      if (contribution.outputsFrom) props.onGetActionOptions(contribution.outputsFrom);
    }
  };

  onMounted(requestTtsSources);
  watch(() => props.page, requestTtsSources);

  provide(TtsNodeStateKey, {
    settingsFor: (pluginId) => (pluginId === props.page.pluginId ? props.ttsSettings : undefined),
    speakingFor: (pluginId) =>
      pluginId === props.page.pluginId ? (props.ttsSpeaking ?? false) : false,
    logsFor: (pluginId) => (pluginId === props.page.pluginId ? (props.ttsLogs ?? []) : []),
    voicesFor: (contributionId) => {
      const contribution = adapted.value.tts.find((entry) => entry.id === contributionId);
      return {
        source: contribution?.voicesFrom ?? '',
        actionType: contribution?.actionType ?? '',
        outputsSource: contribution?.outputsFrom,
      };
    },
    onSettingsChange: (pluginId, next) => props.onTtsSettingsChange?.(pluginId, next),
    onSpeak: (pluginId, actionType, text, voice) =>
      props.onTtsSpeak?.(pluginId, actionType, text, voice),
    onOutputSelect: props.onTtsOutputSelect,
    outputPendingFor: (pluginId) =>
      pluginId === props.page.pluginId ? props.ttsOutputPending : undefined,
    outputErrorFor: (pluginId) =>
      pluginId === props.page.pluginId ? props.ttsOutputError : undefined,
  });

  return () => <PluginPage page={adapted.value.page} context={context.value} />;
  },
);

export default PluginPageView;
</script>
