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
import { outputsTarget } from '../components/tts/tts-outputs.ts';
import { withSchemaDefaults } from '../components/plugin-connection-logic.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { SchemaForm } from '../components/ui/SchemaForm.vue';
import { PluginConnectionCard } from '../components/plugin-connection-card.vue';
import { TtsSettingsPanel } from '../components/tts/TtsSettingsPanel.vue';
import { i18nText, t, type Locale } from '../i18n.ts';
import type { TtsLogEntry, TtsSettings } from '../tts/tts-policy.ts';

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
  ttsSettings?: TtsSettings;
  ttsSpeaking?: boolean;
  ttsLogs?: TtsLogEntry[];
  onTtsSettingsChange?: (pluginId: string, next: TtsSettings) => void;
  onTtsSpeak?: (pluginId: string, actionType: string, text: string, voice: string) => void;
  supportsProvisioning?: boolean;
  provisionState?: { working: boolean; ok: boolean; message: string };
  onProvisionToken?: (id: string, username: string, password: string) => void;
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
 * fixed widget set (text, form, connection, list, tts): every branch below
 * is host code rendering manifest data as text and form controls. There is
 * no markup, script, or component indirection from the manifest.
 *
 * A `connection` section renders the centered connection card: it embeds the
 * full settings form (advanced fields collapsed by SchemaForm), validates
 * the `format: "uri"` field inline, autosaves edits, and collapses to a
 * compact summary once the probe succeeds.
 *
 * A `tts` section renders the host-owned TTS settings panel bound to the
 * manifest-declared speech action and voice source.
 */
export const PluginPageView = defineVueComponent<PluginPageViewProps>(
  ['locale', 'page', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'actionOptionErrors', 'actionOptionSelected', 'ttsOutputPending', 'ttsOutputError', 'onTtsOutputSelect', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker', 'ttsSettings', 'ttsSpeaking', 'ttsLogs', 'onTtsSettingsChange', 'onTtsSpeak', 'supportsProvisioning', 'provisionState', 'onProvisionToken'],
  (props) => {
  // Draft for `form` sections only. `connection` sections render through
  // the shared PluginConnectionCard, which owns its own draft + autosave.
  const draft = ref<JsonObject | null>(null);
  const listSources = computed(() => {
    const sources: string[] = [];
    for (const section of props.page.sections) {
      const markers = section.kind === 'list'
        ? [section.optionsFrom]
        : section.kind === 'tts'
          ? [section.voicesFrom, section.outputsFrom]
          : [];
      for (const marker of markers) {
        if (!marker) continue;
        const source = normalizeOptionsFrom(marker);
        if (source && !sources.includes(source)) sources.push(source);
      }
    }
    return sources;
  });
  const dynamicFields = computed(() => {
    const fields: Array<{ key: string; source: string }> = [];
    const collect = (uiHints: JsonObject | undefined): void => {
      for (const field of optionFields(uiHints)) {
        if (!fields.some((entry) => entry.key === field.key && entry.source === field.source)) {
          fields.push(field);
        }
      }
    };
    for (const section of props.page.sections) {
      if (section.kind !== 'form') continue;
      collect(section.uiHints ?? props.settingsState?.uiHints);
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

  const resetPageState = (): void => {
    draft.value = null;
  };

  onMounted(() => {
    if (!props.settingsState) props.onGetSettings(props.page.pluginId);
    requestListSources();
  });
  watch(() => props.page, () => {
    resetPageState();
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

  // `connection` sections share one component with the Connections tab
  // server list — same card, same behavior, no duplicate implementation.
  const renderConnectionCard = (index: number) => (
    <PluginConnectionCard
      key={index}
      locale={props.locale}
      pluginId={props.page.pluginId}
      pluginName={props.pluginName}
      settingsState={props.settingsState}
      connection={props.connection}
      actionOptions={props.actionOptions}
      onGetSettings={props.onGetSettings}
      onSaveSettings={props.onSaveSettings}
      onGetActionOptions={props.onGetActionOptions}
      onTestConnection={props.onTestConnection}
      onOpenMediaPicker={props.onOpenMediaPicker}
      supportsProvisioning={props.supportsProvisioning}
      provisionState={props.provisionState}
      onProvisionToken={props.onProvisionToken}
    />
  );

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
        // Same display defaults as the connection card so enum selects with
        // schema defaults never render as blank boxes before the first edit.
        const formValues = withSchemaDefaults(draft.value ?? state?.values ?? {}, schema);
        return (
          <section class="plg-stack" key={index}>
            {title && <h3 class="plg-topbar__title">{title}</h3>}
            <div class="plg-form">
              <SchemaForm
                locale={locale}
                schema={schema}
                uiHints={section.uiHints ?? state?.uiHints}
                value={formValues}
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
      case 'connection':
        return renderConnectionCard(index);
      case 'tts': {
        const source = normalizeOptionsFrom(section.voicesFrom) ?? '';
        const options = source ? (props.actionOptions[source] ?? []) : [];
        const voicesError = source ? props.actionOptionErrors[source] : undefined;
        const actionType = section.actionType ?? '';
        if (!actionType || !props.ttsSettings || !props.onTtsSettingsChange || !props.onTtsSpeak) {
          return (
            <section class="plg-stack" key={index}>
              <span class="plg-group-note">{t(locale, 'pluginListEmpty')}</span>
            </section>
          );
        }
        const onSettingsChange = props.onTtsSettingsChange;
        const onSpeak = props.onTtsSpeak;
        const onOutputSelect = props.onTtsOutputSelect;
        const pluginId = props.page.pluginId;
        const outputsSource = section.outputsFrom ? normalizeOptionsFrom(section.outputsFrom) : undefined;
        const outputTarget = outputsSource ? outputsTarget(outputsSource) : undefined;
        return (
          <section key={index}>
            {title && <h3 class="plg-topbar__title" style="padding: 0 16px;">{title}</h3>}
            <TtsSettingsPanel
              locale={locale}
              settings={props.ttsSettings}
              voices={options}
              voicesError={voicesError}
              speaking={props.ttsSpeaking ?? false}
              logs={props.ttsLogs ?? []}
              onSettingsChange={(next) => onSettingsChange(pluginId, next)}
              onRefreshVoices={() => { if (source) props.onGetActionOptions(source, true); }}
              onSpeak={(text, voice) => onSpeak(pluginId, actionType, text, voice)}
              outputsSupported={!!outputsSource}
              outputs={outputsSource ? props.actionOptions[outputsSource] : undefined}
              outputsSelected={outputsSource ? props.actionOptionSelected[outputsSource] : undefined}
              outputsError={outputsSource ? props.actionOptionErrors[outputsSource] : undefined}
              outputsPending={props.ttsOutputPending}
              outputError={props.ttsOutputError}
              onSelectOutput={outputTarget && onOutputSelect && outputsSource
                ? (device) => onOutputSelect(pluginId, outputTarget.actionType, outputTarget.field, device, outputsSource)
                : undefined}
              onRefreshOutputs={outputsSource ? () => props.onGetActionOptions(outputsSource, true) : undefined}
            />
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
                onClick={() => { if (source) props.onGetActionOptions(source, true); }}
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
    </div>
  );
  };
  },
);

export default PluginPageView;
</script>
