<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../automation/behavior/types.ts';
import type { AutomationEvent, JsonObject } from '../../automation/types.ts';
import type { ActionOptionItem, HostMessage, OpenMediaPicker, PluginSettingValues, ProcessorStatusEntry } from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { optionFields, type PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { Icon } from '../components/icons/index.ts';
import { matchesPluginQuery, pluginIconTone, pluginTags, resolvePluginIcon } from '../components/plugin-cards.ts';
import { renderMarkdownLite } from '../components/plugin-markdown.ts';
import { PluginConnectionModal } from '../components/plugin-connection-modal.vue';
import { SchemaForm } from '../components/ui/SchemaForm.vue';
import { Switch } from '../components/ui/Checkbox.vue';
import { processorMetricRows, processorPreviewEvent, processorStatusTone } from '../components/processors.ts';
import { i18nText, t, type Locale } from '../i18n.ts';
import { useDialogs } from '../composables/useDialogs.ts';

export type ProcessorTestState = Extract<HostMessage, { type: 'processor-test-result' }>;

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

function pluginCopy(locale: Locale) {
  return {
    title: t(locale, 'pluginsTitle'),
    lead: t(locale, 'pluginsLead'),
    builtInLabel: t(locale, 'builtInActions'),
    builtInNote: t(locale, 'builtInActionsNote'),
    installed: t(locale, 'pluginsInstalled'),
    store: t(locale, 'pluginsAvailable'),
    actionsLabel: t(locale, 'pluginActionsLabel'),
    install: t(locale, 'pluginInstall'),
    activate: t(locale, 'pluginActivate'),
    uninstall: t(locale, 'pluginUninstall'),
    deactivate: t(locale, 'pluginDeactivate'),
    active: t(locale, 'pluginActive'),
    disabled: t(locale, 'pluginDisabled'),
    unavailable: t(locale, 'pluginUnavailable'),
    emptyInstalled: t(locale, 'pluginsEmpty'),
    emptyStore: t(locale, 'pluginsStoreEmpty'),
    searchPlaceholder: t(locale, 'pluginSearchPlaceholder'),
    searchEmpty: (query: string) => t(locale, 'pluginSearchEmpty', { query }),
    explore: t(locale, 'pluginsAvailable'),
    usedBy: (count: number) => t(locale, 'pluginUsedBy', { count }),
    confirm: t(locale, 'pluginUninstallConfirm'),
    details: t(locale, 'pluginDetails'),
    settings: t(locale, 'pluginSettings'),
    saveSettings: t(locale, 'pluginSettingsSave'),
    settingsHint: t(locale, 'pluginSettingsHint'),
    installPackage: t(locale, 'pluginInstallPackage'),
    installing: t(locale, 'pluginInstalling'),
    installSuccess: t(locale, 'pluginInstallSuccess'),
    installFailed: t(locale, 'pluginInstallFailed'),
    replaceConfirm: t(locale, 'pluginReplaceConfirm'),
    replaceAction: t(locale, 'pluginReplace'),
    cancel: t(locale, 'cancel'),
    processorsTab: t(locale, 'processorsTab'),
    processorsLead: t(locale, 'processorsLead'),
    processorsEmpty: t(locale, 'processorsEmpty'),
    processorsRefresh: t(locale, 'processorsRefresh'),
    processorEventsLabel: t(locale, 'processorEventsLabel'),
    processorAllEvents: t(locale, 'processorAllEvents'),
    processorTimeout: t(locale, 'processorTimeout'),
    processorPriority: t(locale, 'processorPriority'),
    processorMetricsLabel: t(locale, 'processorMetricsLabel'),
    processorTest: t(locale, 'processorTest'),
    processorTestOk: t(locale, 'processorTestOk'),
    processorTestFailed: t(locale, 'processorTestFailed'),
    processorTestDuration: t(locale, 'processorTestDuration'),
    metricLabels: {
      calls: t(locale, 'processorMetricCalls'),
      successes: t(locale, 'processorMetricSuccesses'),
      failures: t(locale, 'processorMetricFailures'),
      timeouts: t(locale, 'processorMetricTimeouts'),
      skippedCircuitOpen: t(locale, 'processorMetricSkippedCircuitOpen'),
      skippedOverloaded: t(locale, 'processorMetricSkippedOverloaded'),
      averageLatencyMs: t(locale, 'processorMetricAverageLatencyMs'),
      maxLatencyMs: t(locale, 'processorMetricMaxLatencyMs'),
    } as Record<string, string>,
    statusLabels: {
      ready: t(locale, 'processorStatusReady'),
      disabled: t(locale, 'processorStatusDisabled'),
      unavailable: t(locale, 'processorStatusUnavailable'),
      degraded: t(locale, 'processorStatusDegraded'),
      'circuit-open': t(locale, 'processorStatusCircuitOpen'),
    } as Record<ProcessorStatusEntry['status'], string>,
  };
}

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

/** Compact plugin card: name + status + primary action visible, actions/meta/settings behind Details. */
type PluginCardProps = {
  locale: Locale;
  plugin: PluginStatus;
  usedBy: number;
  actionTypes: ActionTypeDefinition[];
  settingsState?: PluginSettingsState;
  onSetEnabled: (id: string, enabled: boolean) => void;
  onSetInstalled: (id: string, installed: boolean) => void;
  onUninstall: (id: string) => void;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  actionOptions: Record<string, ActionOptionItem[]>;
  onGetActionOptions: (source: string) => void;
  onConnect: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

const PluginCard = defineVueComponent<PluginCardProps>(
  ['locale', 'plugin', 'usedBy', 'actionTypes', 'settingsState', 'onSetEnabled', 'onSetInstalled', 'onUninstall', 'onGetSettings', 'onSaveSettings', 'actionOptions', 'onGetActionOptions', 'onConnect', 'onOpenMediaPicker'],
  (props) => {
  const open = ref(false);
  const dialogs = useDialogs();

  return () => {
  const copy = pluginCopy(props.locale);
  const plugin = props.plugin;
  const canUninstall = plugin.descriptor.source === 'user';
  const icon = resolvePluginIcon(plugin.descriptor, props.actionTypes);
  const tone = pluginIconTone(plugin.descriptor, icon);
  const tags = pluginTags(plugin.descriptor, props.actionTypes);
  const description = i18nText(props.locale, plugin.descriptor.description);
  const longDescription = plugin.descriptor.longDescription
    ? i18nText(props.locale, plugin.descriptor.longDescription)
    : '';
  const showLong = open.value && longDescription.length > 0 && longDescription !== description;

  return (
    <div class={`plg-plugin${plugin.installed && !plugin.enabled ? ' is-off' : ''}`}>
      <div class="plg-plugin__head plg-plugin__head--card">
        <div class={`plg-plugin__icon is-${tone}`} aria-hidden="true">
          <Icon name={icon} size={22} />
        </div>
        <div class="plg-plugin__body">
          <div class="plg-plugin__title">
            <span class="plg-plugin__name">{i18nText(props.locale, plugin.descriptor.name)}</span>
            <span class="plg-pill plg-pill--mono">{plugin.descriptor.version}</span>
            {plugin.installed && (
              <span class={`plg-pill${plugin.enabled ? ' plg-pill--ok' : ''}`}>
                {plugin.enabled ? copy.active : copy.disabled}
              </span>
            )}
            {!plugin.available && <span class="plg-pill">{copy.unavailable}</span>}
          </div>
          <span class="plg-plugin__desc">{renderMarkdownLite(description, true)}</span>
          {tags.length > 0 && (
            <div class="plg-plugin__tags">
              {tags.map((tag) => <span class="plg-tag" key={tag}>{tag}</span>)}
            </div>
          )}
        </div>

        <div class="plg-plugin__controls">
          {plugin.installed && (
            <Switch
              checked={plugin.enabled}
              onCheckedChange={() => props.onSetEnabled(plugin.descriptor.id, !plugin.enabled)}
              ariaLabel={i18nText(props.locale, plugin.descriptor.name)}
            />
          )}
          <button
            type="button"
            class={`plg-btn plg-btn--sm${open.value ? ' is-active' : ''}`}
            aria-expanded={open.value ? 'true' : 'false'}
            onClick={() => { open.value = !open.value; }}
          >
            <Icon name="doc" size={14} />
            <span>{copy.details}</span>
          </button>
          {plugin.installed && plugin.enabled && plugin.descriptor.hasConnectionProbe && (
            <button
              type="button"
              class="plg-btn plg-btn--sm"
              onClick={() => props.onConnect(plugin.descriptor.id)}
            >
              <Icon name="settings" size={14} />
              <span>{t(props.locale, 'pluginConnect')}</span>
            </button>
          )}
          <button
            type="button"
            class={`plg-btn plg-btn--sm${plugin.installed ? ' plg-btn--danger' : ' plg-btn--primary'}`}
            onClick={async () => {
              if (!plugin.installed) {
                props.onSetInstalled(plugin.descriptor.id, true);
              } else if (canUninstall) {
                const confirmed = await dialogs.confirm(copy.confirm, {
                  title: copy.uninstall,
                  confirmLabel: copy.uninstall,
                  cancelLabel: copy.cancel,
                  danger: true,
                });
                if (confirmed) props.onUninstall(plugin.descriptor.id);
              } else {
                props.onSetInstalled(plugin.descriptor.id, false);
              }
            }}
          >
            {plugin.installed ? (canUninstall ? copy.uninstall : copy.deactivate) : (canUninstall ? copy.activate : copy.install)}
          </button>
        </div>
      </div>

      {plugin.installed && props.usedBy > 0 && (
        <div class="plg-warn">
          <strong>{canUninstall ? copy.uninstall : copy.deactivate}:</strong>
          {copy.usedBy(props.usedBy)}
        </div>
      )}

      {open.value && (
        <div class="plg-plugin__details">
          {showLong && (
            <div class="plg-plugin__long">{renderMarkdownLite(longDescription)}</div>
          )}
          <div class="plg-table__chips">
            <span class="plg-group-note">{copy.actionsLabel}</span>
            {plugin.descriptor.actionTypeIds.map((id) => (
              <span class="plg-pill" key={id}>
                {(() => {
                  const type = props.actionTypes.find((entry) => entry.id === id);
                  return type ? i18nText(props.locale, type.title) : id;
                })()}
              </span>
            ))}
          </div>
          <div class="plg-plugin__meta">
            <span>{i18nText(props.locale, plugin.descriptor.dependency)}</span>
            <span>·</span>
            <span>{plugin.descriptor.permissions.join(' · ')}</span>
          </div>
          {plugin.installed && plugin.descriptor.hasSettings && (
            <PluginSettingsForm
              locale={props.locale}
              pluginId={plugin.descriptor.id}
              plugin={plugin}
              state={props.settingsState}
              onGetSettings={props.onGetSettings}
              onSaveSettings={props.onSaveSettings}
              actionOptions={props.actionOptions}
              onGetActionOptions={props.onGetActionOptions}
              onOpenMediaPicker={props.onOpenMediaPicker}
            />
          )}
        </div>
      )}
    </div>
  );
  };
  },
);

/** Host-owned processor diagnostics: status, counters, and sample-event previews. */
type ProcessorsPanelProps = {
  locale: Locale;
  processors: ProcessorStatusEntry[];
  processorTest: ProcessorTestState | null;
  query: string;
  onGetProcessorStatus?: () => void;
  onTestProcessor?: (pluginId: string, processorId: string, event: AutomationEvent) => void;
};

const ProcessorsPanel = defineVueComponent<ProcessorsPanelProps>(
  ['locale', 'processors', 'processorTest', 'query', 'onGetProcessorStatus', 'onTestProcessor'],
  (props) => {
  return () => {
  const copy = pluginCopy(props.locale);
  const words = props.query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const entries = words.length === 0
    ? props.processors
    : props.processors.filter((entry) => {
        const haystack = `${entry.pluginId} ${entry.processorId} ${entry.status} ${entry.eventTypes.join(' ')}`.toLowerCase();
        return words.every((word) => haystack.includes(word));
      });
  return (
    <>
      <div class="plg-banner">
        <span class="plg-dot is-ok" />
        <span class="plg-banner__label">{copy.processorsTab}</span>
        <span class="plg-banner__note">{copy.processorsLead}</span>
        {props.onGetProcessorStatus && (
          <button type="button" class="plg-btn plg-btn--sm" onClick={() => props.onGetProcessorStatus?.()}>
            {copy.processorsRefresh}
          </button>
        )}
      </div>

      {entries.map((entry) => {
        const test = props.processorTest
          && props.processorTest.pluginId === entry.pluginId
          && props.processorTest.processorId === entry.processorId
          ? props.processorTest
          : null;
        return (
          <div class="plg-plugin" key={`${entry.pluginId}/${entry.processorId}`}>
            <div class="plg-plugin__head">
              <div class="plg-field">
                <div class="plg-plugin__title">
                  <span class={`plg-dot ${processorStatusTone(entry.status)}`} />
                  <span class="plg-plugin__name">{entry.processorId}</span>
                  <span class="plg-pill plg-pill--mono">{entry.pluginId}</span>
                  <span class="plg-pill">{copy.statusLabels[entry.status]}</span>
                </div>
                <div class="plg-table__chips">
                  <span class="plg-group-note">{copy.processorEventsLabel}</span>
                  {entry.eventTypes.length > 0
                    ? entry.eventTypes.map((type) => <span class="plg-pill plg-pill--mono" key={type}>{type}</span>)
                    : <span class="plg-group-note">{copy.processorAllEvents}</span>}
                </div>
                <div class="plg-table__chips">
                  <span class="plg-group-note">{copy.processorMetricsLabel}</span>
                  {processorMetricRows(entry.metrics).map((row) => (
                    <span class="plg-pill" key={row.key}>{copy.metricLabels[row.key] ?? row.key} · {row.value}</span>
                  ))}
                </div>
                <div class="plg-plugin__meta">
                  <span>{copy.processorTimeout} · {entry.timeoutMs} ms</span>
                  <span>·</span>
                  <span>{copy.processorPriority} · {entry.priority}</span>
                </div>
              </div>

              {props.onTestProcessor && (
                <div class="plg-plugin__controls">
                  <button
                    type="button"
                    class="plg-btn plg-btn--sm"
                    onClick={() => props.onTestProcessor?.(entry.pluginId, entry.processorId, processorPreviewEvent(entry))}
                  >
                    {copy.processorTest}
                  </button>
                </div>
              )}
            </div>

            {entry.metrics.lastError && (
              <div class="plg-warn"><span>{entry.metrics.lastError}</span></div>
            )}

            {test && (
              <div class="plg-console">
                <span>{test.ok ? copy.processorTestOk : copy.processorTestFailed} · {copy.processorTestDuration} {test.durationMs} ms</span>
                {test.error && <span>{test.error}</span>}
                <span style="white-space: pre-wrap;">{JSON.stringify(test.result, null, 2)}</span>
              </div>
            )}
          </div>
        );
      })}

      {entries.length === 0 && (
        <div class="plg-empty">
          <span class="plg-empty__desc">
            {words.length > 0 ? copy.searchEmpty(props.query.trim()) : copy.processorsEmpty}
          </span>
        </div>
      )}
    </>
  );
  };
  },
);

/** Host-rendered JSON settings for one plugin. Nothing here executes plugin code. */
type PluginSettingsFormProps = {
  locale: Locale;
  pluginId: string;
  plugin: PluginStatus;
  state?: PluginSettingsState;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  actionOptions: Record<string, ActionOptionItem[]>;
  onGetActionOptions: (source: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

const PluginSettingsForm = defineVueComponent<PluginSettingsFormProps>(
  ['locale', 'pluginId', 'plugin', 'state', 'onGetSettings', 'onSaveSettings', 'actionOptions', 'onGetActionOptions', 'onOpenMediaPicker'],
  (props) => {
  const open = ref(false);
  const draft = ref<JsonObject | null>(null);
  const dynamicFields = computed(() => optionFields(props.state?.uiHints));
  watch(() => props.state?.uiHints, () => {
    if (open.value) for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  });
  const fieldOptions = computed(() => {
    const merged: Record<string, Array<{ value: string; label: string }>> = {};
    for (const field of dynamicFields.value) {
      const options = props.actionOptions[field.source];
      if (options && options.length > 0) merged[field.key] = options;
    }
    return merged;
  });

  return () => {
  if (!props.plugin.descriptor.hasSettings) return null;
  const state = props.state;
  return (
    <div class="plg-plugin__settings">
      <button
        type="button"
        class="plg-btn plg-btn--sm"
        onClick={() => {
          if (!open.value && !state) props.onGetSettings(props.pluginId);
          if (!open.value) {
            draft.value = null;
            for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
          }
          open.value = !open.value;
        }}
      >
        {t(props.locale, 'pluginSettings')}
      </button>
      {open.value && state && (
        <div class="plg-form">
          <span class="plg-group-note">{t(props.locale, 'pluginSettingsHint')}</span>
          <SchemaForm
            locale={props.locale}
            schema={state.schema}
            uiHints={state.uiHints}
            value={draft.value ?? state.values}
            fieldOptions={fieldOptions.value}
            onChange={(value) => { draft.value = value; }}
            onOpenMediaPicker={props.onOpenMediaPicker}
          />
          <div class="plg-row">
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--sm"
              onClick={() => props.onSaveSettings(props.pluginId, toSettingValues(draft.value ?? state.values))}
            >
              {t(props.locale, 'pluginSettingsSave')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
  };
  },
);

function toSettingValues(value: JsonObject): PluginSettingValues {
  const clean: PluginSettingValues = {};
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string' || typeof entry === 'number' || typeof entry === 'boolean') clean[key] = entry;
  }
  return clean;
}

export default PluginsView;
</script>
