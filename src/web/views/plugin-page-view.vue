<script lang="tsx">
import { computed, onMounted, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type {
  PluginPageDescriptor,
  PluginPageSection,
} from '../../automation/behavior/types.ts';
import type { JsonObject } from '../../automation/types.ts';
import {
  normalizeOptionsFrom,
  optionFields,
  type PluginConnectionState,
} from '../../automation/plugins/declarative.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { PluginConnectionModal } from '../components/plugin-connection-modal.vue';
import { SchemaForm } from '../components/ui/SchemaForm.vue';
import { i18nText, t, type Locale } from '../i18n.ts';

type PluginPageViewProps = {
  locale: Locale;
  page: PluginPageDescriptor;
  pluginName: string;
  settingsState?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Record<string, ActionOptionItem[]>;
  actionOptionErrors: Record<string, string>;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string) => void;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

function toSettingValues(value: JsonObject): PluginSettingValues {
  const clean: PluginSettingValues = {};
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string' || typeof entry === 'number' || typeof entry === 'boolean') clean[key] = entry;
  }
  return clean;
}

/**
 * Host-owned renderer for plugin configuration pages. Section kinds form a
 * fixed widget set (text, form, connection, list): every branch below is
 * host code rendering manifest data as text and form controls. There is no
 * markup, script, or component indirection from the manifest.
 */
export const PluginPageView = defineVueComponent<PluginPageViewProps>(
  ['locale', 'page', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'actionOptionErrors', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker'],
  (props) => {
  const draft = ref<JsonObject | null>(null);
  const configureOpen = ref(false);
  const listSources = computed(() => {
    const sources: string[] = [];
    for (const section of props.page.sections) {
      if (section.kind !== 'list') continue;
      const source = normalizeOptionsFrom(section.optionsFrom);
      if (source && !sources.includes(source)) sources.push(source);
    }
    return sources;
  });
  const dynamicFields = computed(() => {
    const fields: Array<{ key: string; source: string }> = [];
    for (const section of props.page.sections) {
      if (section.kind !== 'form') continue;
      for (const field of optionFields(section.uiHints ?? props.settingsState?.uiHints)) {
        if (!fields.some((entry) => entry.key === field.key && entry.source === field.source)) {
          fields.push(field);
        }
      }
    }
    return fields;
  });
  const fieldOptions = computed(() => {
    const merged: Record<string, Array<{ value: string; label: string }>> = {};
    for (const field of dynamicFields.value) {
      const options = props.actionOptions[field.source];
      if (options && options.length > 0) merged[field.key] = options;
    }
    return merged;
  });

  const requestListSources = (): void => {
    for (const source of listSources.value) props.onGetActionOptions(source);
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  };

  onMounted(() => {
    if (!props.settingsState) props.onGetSettings(props.page.pluginId);
    requestListSources();
    // First-run auto-detect: probe once when a connection section has no result yet.
    if (!props.connection && props.page.sections.some((section) => section.kind === 'connection')) {
      props.onTestConnection(props.page.pluginId);
    }
  });
  watch(() => props.page, () => {
    draft.value = null;
    if (!props.settingsState) props.onGetSettings(props.page.pluginId);
    requestListSources();
  });
  watch(() => props.settingsState?.uiHints, () => {
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  });

  const saveSettings = (): void => {
    const values = draft.value ?? props.settingsState?.values;
    if (values) props.onSaveSettings(props.page.pluginId, toSettingValues(values));
  };

  const renderSection = (
    section: PluginPageSection,
    index: number,
  ) => {
    const locale = props.locale;
    const title = section.title ? i18nText(locale, section.title) : '';
    switch (section.kind) {
      case 'text':
        return (
          <section class="plg-stack" key={index}>
            {title && <h3 class="plg-topbar__title">{title}</h3>}
            <p class="plg-plugin__desc">{section.text ? i18nText(locale, section.text) : ''}</p>
          </section>
        );
      case 'form': {
        const state = props.settingsState;
        const schema = section.schema ?? state?.schema;
        if (!schema) return null;
        return (
          <section class="plg-stack" key={index}>
            {title && <h3 class="plg-topbar__title">{title}</h3>}
            <div class="plg-form">
              <SchemaForm
                locale={locale}
                schema={schema}
                uiHints={section.uiHints ?? state?.uiHints}
                value={draft.value ?? state?.values ?? {}}
                fieldOptions={fieldOptions.value}
                onChange={(next) => { draft.value = next; }}
                onOpenMediaPicker={props.onOpenMediaPicker}
              />
              <div class="plg-row">
                <button
                  type="button"
                  class="plg-btn plg-btn--primary plg-btn--sm"
                  disabled={!state}
                  onClick={saveSettings}
                >
                  {t(locale, 'pluginSettingsSave')}
                </button>
              </div>
            </div>
          </section>
        );
      }
      case 'connection': {
        const connection = props.connection;
        return (
          <section class="plg-stack" key={index}>
            {title && <h3 class="plg-topbar__title">{title}</h3>}
            {connection ? (
              <div class={`plg-alert${connection.ok ? ' plg-alert--ok' : ''}`} role="status">
                {connection.ok
                  ? t(locale, 'pluginConnectedIn', { ms: connection.latencyMs })
                  : (connection.error || t(locale, 'pluginConnectionFailed'))}
              </div>
            ) : (
              <span class="plg-group-note">{t(locale, 'pluginConnectionHint')}</span>
            )}
            <div class="plg-row">
              <button
                type="button"
                class="plg-btn plg-btn--sm"
                onClick={() => props.onTestConnection(props.page.pluginId)}
              >
                {t(locale, 'pluginTestConnection')}
              </button>
              <button
                type="button"
                class="plg-btn plg-btn--sm"
                onClick={() => { configureOpen.value = true; }}
              >
                {t(locale, 'pluginConfigure')}
              </button>
            </div>
          </section>
        );
      }
      case 'list': {
        const source = normalizeOptionsFrom(section.optionsFrom) ?? '';
        const options = source ? (props.actionOptions[source] ?? []) : [];
        const listError = source ? props.actionOptionErrors[source] : undefined;
        return (
          <section class="plg-stack" key={index}>
            {title && <h3 class="plg-topbar__title">{title}</h3>}
            {listError && <div class="plg-alert" role="status">{listError}</div>}
            {options.length > 0 ? (
              <ul class="plg-list">
                {options.map((option) => (
                  <li key={option.value} class="plg-list__row">
                    <span class="plg-list__label">{option.label}</span>
                    <span class="plg-pill plg-pill--mono">{option.value}</span>
                  </li>
                ))}
              </ul>
            ) : (
              !listError && <span class="plg-group-note">{t(locale, 'pluginListEmpty')}</span>
            )}
            <div class="plg-row">
              <button
                type="button"
                class="plg-btn plg-btn--sm"
                onClick={() => { if (source) props.onGetActionOptions(source); }}
              >
                {t(locale, 'pluginListRefresh')}
              </button>
            </div>
          </section>
        );
      }
    }
  };

  return () => {
  const locale = props.locale;
  return (
    <div class="plg">
      <div class="plg-topbar">
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{i18nText(locale, props.page.title)}</h2>
          <span class="plg-topbar__subtitle">{props.pluginName}</span>
        </div>
      </div>
      <div class="plg-scroll">
        <div class="plg-stack">
          {props.page.sections.map((section, index) => renderSection(section, index))}
        </div>
      </div>
      {configureOpen.value && (
        <PluginConnectionModal
          locale={locale}
          pluginId={props.page.pluginId}
          pluginName={props.pluginName}
          state={props.settingsState}
          connection={props.connection}
          actionOptions={props.actionOptions}
          onGetSettings={props.onGetSettings}
          onSaveSettings={props.onSaveSettings}
          onGetActionOptions={props.onGetActionOptions}
          onTestConnection={props.onTestConnection}
          onOpenMediaPicker={props.onOpenMediaPicker}
          onClose={() => { configureOpen.value = false; }}
        />
      )}
    </div>
  );
  };
  },
);

export default PluginPageView;
</script>
