<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../automation/behavior/types.ts';
import type { AutomationEvent } from '../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues, ProcessorStatusEntry } from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { Icon } from '../components/icons/index.ts';
import { matchesPluginQuery } from '../components/plugin-cards.ts';
import { PluginConnectionModal } from '../components/plugin-connection-modal.vue';
import { i18nText, type Locale } from '../i18n.ts';
import { pluginCopy } from './plugins/plugin-copy.ts';
import { PluginCard } from './plugins/PluginCard.vue';
import { ProcessorsPanel, type ProcessorTestState } from './plugins/ProcessorsPanel.vue';

export type { ProcessorTestState };

export type PluginInstallViewState = {
  installing: boolean;
  error: string;
  success: string;
  pendingPath: string;
  needsReplace: boolean;
};

type PluginsViewProps = {
  locale: Locale;
  plugins: PluginStatus[];
  actions: LiveAction[];
  actionTypes: ActionTypeDefinition[];
  error?: string;
  onUninstall: (id: string) => void;
  onSetInstalled: (id: string, installed: boolean) => void;
  onSetEnabled: (id: string, enabled: boolean) => void;
  settings: Record<string, PluginSettingsState>;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  actionOptions: Record<string, ActionOptionItem[]>;
  onGetActionOptions: (source: string) => void;
  connections: Record<string, PluginConnectionState>;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onInstallPlugin?: () => void;
  pluginInstallState?: PluginInstallViewState;
  onConfirmReplace?: () => void;
  onCancelReplace?: () => void;
  processors?: ProcessorStatusEntry[];
  processorTest?: ProcessorTestState | null;
  onGetProcessorStatus?: () => void;
  onTestProcessor?: (pluginId: string, processorId: string, event: AutomationEvent) => void;
};

export const PluginsView = defineVueComponent<PluginsViewProps>(
  ['locale', 'plugins', 'actions', 'actionTypes', 'error', 'onSetInstalled', 'onUninstall', 'onSetEnabled', 'settings', 'onGetSettings', 'onSaveSettings', 'actionOptions', 'onGetActionOptions', 'connections', 'onTestConnection', 'onOpenMediaPicker', 'onInstallPlugin', 'pluginInstallState', 'onConfirmReplace', 'onCancelReplace', 'processors', 'processorTest', 'onGetProcessorStatus', 'onTestProcessor'],
  (props) => {
  const tab = ref<'installed' | 'store' | 'processors'>('installed');
  const connectTarget = ref<string | null>(null);
  const query = ref('');

  return () => {
  const copy = pluginCopy(props.locale);
  const installed = props.plugins.filter((plugin) => plugin.installed);
  const available = props.plugins.filter((plugin) => !plugin.installed);
  const base = tab.value === 'installed' ? installed : available;
  const visible = base.filter((plugin) => matchesPluginQuery(props.locale, plugin, props.actionTypes, query.value));
  const filtering = query.value.trim().length > 0;
  // Annotated local keeps the generic-inference chain shallow for vue-tsc.
  const installedPlugins: PluginStatus[] = props.plugins;
  const connectPlugin = connectTarget.value ? installedPlugins.find((plugin) => plugin.descriptor.id === connectTarget.value) : undefined;

  const installState = props.pluginInstallState;
  const installing = installState?.installing ?? false;

  return (
    <div class="plg">
      <div class="plg-topbar">
        <div class="plg-topbar__icon" aria-hidden="true">
          <Icon name="plugins" size={20} />
        </div>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{copy.title}</h2>
          <span class="plg-topbar__subtitle">{copy.lead}</span>
        </div>
        {props.onInstallPlugin && (
          <button
            type="button"
            class="plg-btn plg-btn--primary"
            disabled={installing}
            onClick={() => props.onInstallPlugin?.()}
          >
            <Icon name="plus" size={15} />
            <span>{installing ? copy.installing : copy.installPackage}</span>
          </button>
        )}
      </div>

      <div class="plg-browsebar">
        <div class="plg-tabs">
          <button
            type="button"
            class={`plg-tab${tab.value === 'installed' ? ' is-active' : ''}`}
            onClick={() => { tab.value = 'installed'; }}
          >
            <Icon name="plugin" size={14} />
            <span>{copy.installed}</span>
            <span class="plg-tab__count">{installed.length}</span>
          </button>
          <button
            type="button"
            class={`plg-tab${tab.value === 'store' ? ' is-active' : ''}`}
            onClick={() => { tab.value = 'store'; }}
          >
            <Icon name="plus" size={14} />
            <span>{copy.store}</span>
            <span class="plg-tab__count">{available.length}</span>
          </button>
          <button
            type="button"
            class={`plg-tab${tab.value === 'processors' ? ' is-active' : ''}`}
            onClick={() => { tab.value = 'processors'; props.onGetProcessorStatus?.(); }}
          >
            <Icon name="bolt" size={14} />
            <span>{copy.processorsTab}</span>
            <span class="plg-tab__count">{(props.processors ?? []).length}</span>
          </button>
        </div>
        <div class="plg-search">
          <Icon name="search" size={15} />
          <input
            type="search"
            class="plg-search__input"
            placeholder={copy.searchPlaceholder}
            aria-label={copy.searchPlaceholder}
            value={query.value}
            onInput={(event) => { query.value = (event.currentTarget as HTMLInputElement).value; }}
          />
          {filtering && (
            <button
              type="button"
              class="plg-search__clear"
              aria-label={copy.cancel}
              onClick={() => { query.value = ''; }}
            >
              <Icon name="close" size={13} />
            </button>
          )}
        </div>
      </div>

      {props.error && <div class="plg-stack"><div class="plg-alert">{props.error}</div></div>}
      {installState?.success && <div class="plg-stack"><div class="plg-banner"><span class="plg-dot is-ok" /><span>{installState.success}</span></div></div>}
      {installState?.error && !installState.needsReplace && <div class="plg-stack"><div class="plg-alert">{installState.error}</div></div>}
      {installState?.needsReplace && (
        <div class="plg-stack"><div class="plg-alert">
          <span>{copy.replaceConfirm}</span>
          <div class="plg-row">
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--sm"
              disabled={installing}
              onClick={() => props.onConfirmReplace?.()}
            >
              {copy.replaceAction}
            </button>
            <button
              type="button"
              class="plg-btn plg-btn--sm"
              disabled={installing}
              onClick={() => props.onCancelReplace?.()}
            >
              {copy.cancel}
            </button>
          </div>
        </div></div>
      )}

      <div class="plg-scroll">
        <div class="plg-stack">
          {tab.value === 'processors' ? (
            <ProcessorsPanel
              locale={props.locale}
              processors={props.processors ?? []}
              processorTest={props.processorTest ?? null}
              query={query.value}
              onGetProcessorStatus={props.onGetProcessorStatus}
              onTestProcessor={props.onTestProcessor}
            />
          ) : (
          <>

          {visible.length > 0 && (
            <div class="plg-plugin-list">
              {visible.map((plugin) => {
                const usedBy = props.actions.filter((action) => {
                  const type = props.actionTypes.find((entry) => entry.id === action.typeId);
                  return type?.source.kind === 'plugin' && type.source.pluginId === plugin.descriptor.id;
                }).length;
                return (
                  <PluginCard
                    key={plugin.descriptor.id}
                    locale={props.locale}
                    plugin={plugin}
                    usedBy={usedBy}
                    actionTypes={props.actionTypes}
                    settingsState={props.settings[plugin.descriptor.id]}
                    onSetEnabled={props.onSetEnabled}
                    onSetInstalled={props.onSetInstalled}
                    onUninstall={props.onUninstall}
                    onGetSettings={props.onGetSettings}
                    onSaveSettings={props.onSaveSettings}
                    actionOptions={props.actionOptions}
                    onGetActionOptions={props.onGetActionOptions}
                    onConnect={(id) => { connectTarget.value = id; }}
                    onOpenMediaPicker={props.onOpenMediaPicker}
                  />
                );
              })}
            </div>
          )}

          {visible.length === 0 && (
            <div class="plg-empty">
              <span class="plg-empty__desc">
                {filtering
                  ? copy.searchEmpty(query.value.trim())
                  : tab.value === 'store' ? copy.emptyStore : copy.emptyInstalled}
              </span>
              {!filtering && tab.value === 'installed' && available.length > 0 && (
                <button type="button" class="plg-btn plg-btn--primary" onClick={() => { tab.value = 'store'; }}>
                  {copy.explore}
                </button>
              )}
              {!filtering && tab.value === 'store' && props.onInstallPlugin && (
                <button
                  type="button"
                  class="plg-btn plg-btn--primary"
                  disabled={installing}
                  onClick={() => props.onInstallPlugin?.()}
                >
                  {installing ? copy.installing : copy.installPackage}
                </button>
              )}
            </div>
          )}
          </>
          )}
        </div>
      </div>
      {connectPlugin && (
        <PluginConnectionModal
          locale={props.locale}
          pluginId={connectPlugin.descriptor.id}
          pluginName={i18nText(props.locale, connectPlugin.descriptor.name)}
          state={props.settings[connectPlugin.descriptor.id]}
          connection={props.connections[connectPlugin.descriptor.id]}
          actionOptions={props.actionOptions}
          onGetSettings={props.onGetSettings}
          onSaveSettings={props.onSaveSettings}
          onGetActionOptions={props.onGetActionOptions}
          onTestConnection={props.onTestConnection}
          onOpenMediaPicker={props.onOpenMediaPicker}
          onClose={() => { connectTarget.value = null; }}
        />
      )}
    </div>
  );
  };
  },
);

export default PluginsView;
</script>
