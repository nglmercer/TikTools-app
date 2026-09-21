<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import type { JsonObject } from '../../automation/types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { adaptLegacyPage } from '../../plugin-ui/index.ts';
import type { PluginUiDescriptor } from '../../plugin-ui/contracts.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import type { Locale } from '../i18n.ts';
import type { PluginUiContext } from '../plugin-ui/PluginUiContext.ts';
import { PluginPage } from '../plugin-ui/PluginPage.vue';
import { PluginWebviewPage } from '../plugin-ui/PluginWebviewPage.vue';

type PluginPageViewProps = {
  locale: Locale;
  page: PluginPageDescriptor;
  pluginName: string;
  settingsState?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Record<string, ActionOptionItem[]>;
  actionOptionErrors: Record<string, string>;
  actionOptionSelected: Record<string, string>;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string, refresh?: boolean, pluginId?: string) => void;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onExecuteAction?: (actionType: string, config: PluginSettingValues) => void;
  supportsProvisioning?: boolean;
  provisionState?: { working: boolean; ok: boolean; message: string };
  onProvisionToken?: (id: string, username: string, password: string) => void;
  /** Host-stamped `ui` descriptor for this plugin, when the snapshot carries one. */
  ui?: PluginUiDescriptor;
};

/**
 * Backward-compatible wrapper for legacy schema-v3 plugin pages.
 *
 * The v3 descriptor is adapted once to the generic declarative contract
 * and rendered through the domain-free `<PluginPage>` + `<PluginNode>`
 * registry. No domain state flows through this wrapper: plugin-specific
 * panels (audio, speech, …) live in the plugin's isolated view, and the
 * legacy `tts` section kind degrades to a neutral status note in the
 * adapter.
 */
export const PluginPageView = defineVueComponent<PluginPageViewProps>(
  ['locale', 'page', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'actionOptionErrors', 'actionOptionSelected', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker', 'onExecuteAction', 'supportsProvisioning', 'provisionState', 'onProvisionToken', 'ui'],
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
      get: (source, refresh) => props.onGetActionOptions(source, refresh, props.page.pluginId),
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
    customNodes: {},
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

  // Webview-mode pages never render through the declarative renderer:
  // the plugin's compiled UI loads isolated (native window on desktop).
  const webviewPage = computed(() => {
    const ui = props.ui;
    if (!ui || ui.mode !== 'webview' || ui.pluginId !== props.page.pluginId) return undefined;
    return ui.pages.find((entry) => entry.id === props.page.id);
  });

  return () => {
    const webview = webviewPage.value;
    if (webview) {
      return (
        <PluginWebviewPage
          locale={props.locale}
          pluginId={props.page.pluginId}
          pluginName={props.pluginName}
          pageId={webview.id}
          title={webview.title}
        />
      );
    }
    return <PluginPage page={adapted.value} context={context.value} />;
  };
  },
);

export default PluginPageView;
</script>
