<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { ActionTypeDefinition, PluginStatus } from '../../../automation/behavior/types.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../shared/messages.ts';
import type { PluginSettingsState } from '../../types.ts';
import { Icon } from '../../components/icons/index.ts';
import { pluginIconTone, pluginTags, resolvePluginIcon } from '../../components/plugin-cards.ts';
import { renderMarkdownLite } from '../../components/plugin-markdown.ts';
import { Switch } from '../../components/ui/Checkbox.vue';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { useDialogs } from '../../composables/useDialogs.ts';
import { pluginCopy } from './plugin-copy.ts';
import { PluginSettingsForm } from './PluginSettingsForm.vue';

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

export const PluginCard = defineVueComponent<PluginCardProps>(
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
          {!plugin.installed && (
            <button
              type="button"
              class="plg-btn plg-btn--sm plg-btn--primary"
              onClick={() => { props.onSetInstalled(plugin.descriptor.id, true); }}
            >
              {canUninstall ? copy.activate : copy.install}
            </button>
          )}
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
            class={`plg-btn plg-btn--sm${open.value ? ' is-active' : ''}`}
            aria-expanded={open.value ? 'true' : 'false'}
            onClick={() => { open.value = !open.value; }}
          >
            <Icon name="doc" size={14} />
            <span>{copy.details}</span>
          </button>
          {plugin.installed && (
            <Switch
              checked={plugin.enabled}
              onCheckedChange={() => props.onSetEnabled(plugin.descriptor.id, !plugin.enabled)}
              ariaLabel={i18nText(props.locale, plugin.descriptor.name)}
            />
          )}
        </div>
      </div>

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
          {plugin.installed && canUninstall && (
            <div class="plg-plugin__danger">
              {props.usedBy > 0 && (
                <div class="plg-warn">
                  <strong>{copy.uninstall}:</strong>
                  {copy.usedBy(props.usedBy)}
                </div>
              )}
              <div>
                <button
                  type="button"
                  class="plg-btn plg-btn--sm plg-btn--danger"
                  onClick={async () => {
                    const confirmed = await dialogs.confirm(copy.confirm, {
                      title: copy.uninstall,
                      confirmLabel: copy.uninstall,
                      cancelLabel: copy.cancel,
                      danger: true,
                    });
                    if (confirmed) props.onUninstall(plugin.descriptor.id);
                  }}
                >
                  {copy.uninstall}
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
  };
  },
);

export default PluginCard;
</script>
