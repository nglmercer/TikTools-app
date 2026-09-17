<script lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
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
import {
  AUTOSAVE_CONFIRM_TIMEOUT_MS,
  AUTOSAVE_DEBOUNCE_MS,
  connectionSummaryRows,
  echoConfirmsSave,
  findServerUrlKey,
  isHttpUrl,
  isLoopbackUrl,
  secretSettingKeys,
  settingsEqual,
  settingsMatch,
  stableSettingsJson,
  withSchemaDefaults,
} from '../components/plugin-connection-logic.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
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

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

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
 *
 * A `connection` section renders the centered connection card: it embeds the
 * full settings form (advanced fields collapsed by SchemaForm), validates
 * the `format: "uri"` field inline, autosaves edits, and collapses to a
 * compact summary once the probe succeeds.
 */
export const PluginPageView = defineVueComponent<PluginPageViewProps>(
  ['locale', 'page', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'actionOptionErrors', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker'],
  (props) => {
  const draft = ref<JsonObject | null>(null);
  const editing = ref(false);
  const testing = ref(false);
  const saveState = ref<SaveState>('idle');
  const lastSeenProbe = ref(0);
  const lastSent = ref<PluginSettingValues | null>(null);
  const lastSentJson = ref<string | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let confirmTimer: ReturnType<typeof setTimeout> | null = null;
  const hasConnection = computed(() => props.page.sections.some((section) => section.kind === 'connection'));
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
    // The connection card embeds the settings form, so its dynamic fields
    // resolve from the settings hints even without a form section.
    if (hasConnection.value) collect(props.settingsState?.uiHints);
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
  const urlKey = computed(() => findServerUrlKey(props.settingsState?.schema));
  // Masked secret fields round-trip as the host placeholder: the echo can
  // never carry the typed value back, so confirmation must exempt them.
  const secretKeys = computed(() => secretSettingKeys(
    props.settingsState?.schema,
    props.settingsState?.uiHints,
  ));
  const displayValues = computed(() => withSchemaDefaults(
    draft.value ?? props.settingsState?.values ?? {},
    props.settingsState?.schema,
  ));
  const urlValue = computed(() => {
    const key = urlKey.value;
    if (!key) return '';
    const raw = displayValues.value[key];
    return typeof raw === 'string' ? raw : '';
  });
  const urlInvalid = computed(() => urlKey.value !== undefined && !isHttpUrl(urlValue.value));
  const dirty = computed(() => {
    const state = props.settingsState;
    const next = draft.value;
    if (!state || !next) return false;
    // Secret-aware: a typed secret matches its redacted echo, so saving a
    // token goes clean while the typed value stays in the draft (masked) for
    // Show/Hide. Clearing a secret stays dirty until the host confirms it.
    return !settingsMatch(
      withSchemaDefaults(toSettingValues(next), state.schema),
      withSchemaDefaults(toSettingValues(state.values), state.schema),
      secretKeys.value,
    );
  });
  const showSummary = computed(() => (
    hasConnection.value
    && !editing.value
    && !dirty.value
    && props.connection?.ok === true
    && props.settingsState !== undefined
  ));
  const summaryRows = computed(() => connectionSummaryRows(
    displayValues.value,
    props.settingsState?.schema,
    props.settingsState?.uiHints,
    urlKey.value,
    props.locale,
  ));

  const clearSaveTimers = (): void => {
    if (saveTimer !== null) clearTimeout(saveTimer);
    if (confirmTimer !== null) clearTimeout(confirmTimer);
    saveTimer = null;
    confirmTimer = null;
  };

  const flushSave = (): void => {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = null;
    const state = props.settingsState;
    const next = draft.value;
    if (!state || !next) return;
    const payload = toSettingValues(next);
    const json = stableSettingsJson(payload);
    if (json === lastSentJson.value) return;
    lastSent.value = payload;
    lastSentJson.value = json;
    saveState.value = 'saving';
    props.onSaveSettings(props.page.pluginId, payload);
    if (confirmTimer !== null) clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => {
      confirmTimer = null;
      if (saveState.value === 'saving') saveState.value = 'error';
    }, AUTOSAVE_CONFIRM_TIMEOUT_MS);
  };

  const scheduleSave = (): void => {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = setTimeout(flushSave, AUTOSAVE_DEBOUNCE_MS);
  };

  const onConnectionFormChange = (next: JsonObject): void => {
    draft.value = next;
    editing.value = true;
    if (saveState.value === 'saved' || saveState.value === 'error') saveState.value = 'idle';
    scheduleSave();
  };

  const testConnection = (): void => {
    testing.value = true;
    props.onTestConnection(props.page.pluginId);
  };

  const requestListSources = (): void => {
    for (const source of listSources.value) props.onGetActionOptions(source);
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  };

  const resetPageState = (): void => {
    draft.value = null;
    editing.value = false;
    testing.value = false;
    saveState.value = 'idle';
    lastSent.value = null;
    lastSentJson.value = null;
    clearSaveTimers();
  };

  onMounted(() => {
    if (!props.settingsState) props.onGetSettings(props.page.pluginId);
    requestListSources();
    // First-run auto-detect: probe once when a connection section has no result yet.
    if (!props.connection && hasConnection.value) {
      testConnection();
    }
  });
  onUnmounted(clearSaveTimers);
  watch(() => props.page, () => {
    resetPageState();
    if (!props.settingsState) props.onGetSettings(props.page.pluginId);
    requestListSources();
  });
  watch(() => props.settingsState?.uiHints, () => {
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  });
  watch(() => props.settingsState?.values, (values) => {
    if (!values) return;
    const sent = lastSent.value;
    if ((saveState.value === 'saving' || saveState.value === 'error') && sent && echoConfirmsSave(values, sent, secretKeys.value)) {
      saveState.value = 'saved';
      if (confirmTimer !== null) clearTimeout(confirmTimer);
      confirmTimer = null;
      // Adopt confirmed clears: a secret the user emptied reads back as the
      // placeholder, so drop it from the draft and fall back to the echo.
      // Typed values stay in the draft (masked) so Show/Hide keeps revealing
      // what was typed. Only when nothing newer was typed after the send.
      const next = draft.value;
      if (next && settingsEqual(toSettingValues(next), sent)) {
        const pruned = { ...next };
        let changed = false;
        for (const key of secretKeys.value) {
          if (pruned[key] === '' && sent[key] === '') {
            delete pruned[key];
            changed = true;
          }
        }
        if (changed) draft.value = pruned;
      }
    }
    // A send raced with newer edits: converge instead of going stale.
    const state = props.settingsState;
    const next = draft.value;
    if (state && next && !settingsEqual(
      withSchemaDefaults(toSettingValues(next), state.schema),
      withSchemaDefaults(toSettingValues(values), state.schema),
    )) {
      scheduleSave();
    }
  });
  watch(() => props.connection, (connection) => {
    if (connection && connection.at !== lastSeenProbe.value) {
      lastSeenProbe.value = connection.at;
      testing.value = false;
      // A passing probe collapses the card to its summary; a failure opens
      // the form so the banner and fields are visible.
      editing.value = !connection.ok;
    }
  });

  const saveSettings = (): void => {
    const values = draft.value ?? props.settingsState?.values;
    if (values) props.onSaveSettings(props.page.pluginId, toSettingValues(values));
  };

  const renderSaveState = (locale: Locale) => {
    const state = saveState.value;
    if (state === 'idle') return null;
    const label = state === 'saving'
      ? t(locale, 'pluginSaving')
      : state === 'saved'
        ? `${t(locale, 'pluginSaved')} ✓`
        : t(locale, 'pluginSaveError');
    return (
      <span
        role="status"
        class={`plg-connect__save${state === 'saved' ? ' is-ok' : ''}${state === 'error' ? ' is-err' : ''}`}
      >
        {label}
      </span>
    );
  };

  const renderConnectionCard = (index: number) => {
    const locale = props.locale;
    const state = props.settingsState;
    const connection = props.connection;
    if (showSummary.value && connection?.ok) {
      return (
        <section class="plg-connect" key={index}>
          <div class="plg-connect__card">
            <div class="plg-connect__head">
              <span class="plg-connect__status">
                <span class="plg-dot is-ok" aria-hidden="true" />
                <span class="plg-connect__url">{t(locale, 'pluginConnectedTo', { url: urlValue.value })}</span>
              </span>
            </div>
            {summaryRows.value.length > 0 && (
              <dl class="plg-connect__rows">
                {summaryRows.value.map((row) => (
                  <div class="plg-connect__row" key={row.key}>
                    <dt>{row.label}</dt>
                    <dd>{row.value}</dd>
                  </div>
                ))}
              </dl>
            )}
            <div class="plg-connect__actions">
              <button
                type="button"
                class="plg-btn plg-btn--primary plg-btn--sm"
                disabled={testing.value}
                onClick={testConnection}
              >
                {testing.value ? t(locale, 'pluginTestingConnection') : t(locale, 'pluginTestAgain')}
              </button>
              <button
                type="button"
                class="plg-btn plg-btn--sm"
                onClick={() => { editing.value = true; }}
              >
                {t(locale, 'pluginEditSettings')}
              </button>
            </div>
          </div>
        </section>
      );
    }
    const failed = connection && !connection.ok && !dirty.value && !urlInvalid.value;
    const statusDot = dirty.value || !connection
      ? ''
      : testing.value
        ? ''
        : connection.ok
          ? ' is-ok'
          : ' is-err';
    const statusText = dirty.value
      ? t(locale, 'pluginTestToVerify')
      : testing.value || !connection
        ? testing.value ? t(locale, 'pluginTestingConnection') : t(locale, 'pluginStatusNotConfigured')
        : connection.ok
          ? t(locale, 'pluginConnectedIn', { ms: connection.latencyMs })
          : t(locale, 'pluginConnectionFailed');
    return (
      <section class="plg-connect" key={index}>
        <div class="plg-connect__card" onFocusout={flushSave}>
          <div class="plg-connect__head">
            <span class="plg-connect__status">
              <span class={`plg-dot${statusDot}`} aria-hidden="true" />
              <span>{statusText}</span>
            </span>
            {renderSaveState(locale)}
          </div>
          {failed && (
            <div class="plg-alert" role="status">
              {connection?.error || t(locale, 'pluginConnectionFailed')}
            </div>
          )}
          {state ? (
            <SchemaForm
              locale={locale}
              schema={state.schema}
              uiHints={state.uiHints}
              value={displayValues.value}
              fieldOptions={fieldOptions.value}
              fieldErrors={urlInvalid.value && urlKey.value
                ? { [urlKey.value]: t(locale, 'invalidUrl') }
                : undefined}
              onChange={onConnectionFormChange}
              onOpenMediaPicker={props.onOpenMediaPicker}
            />
          ) : (
            <span class="plg-group-note">{t(locale, 'pluginTestingConnection')}</span>
          )}
          <div class="plg-connect__actions">
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--block"
              disabled={!state || testing.value || urlInvalid.value}
              onClick={testConnection}
            >
              {testing.value ? t(locale, 'pluginTestingConnection') : t(locale, 'pluginTestConnection')}
            </button>
          </div>
          <p class="plg-note">
            {urlValue.value && isLoopbackUrl(urlValue.value)
              ? t(locale, 'pluginLocalTrustedNote')
              : t(locale, 'pluginConnectionHint')}
          </p>
        </div>
      </section>
    );
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
    </div>
  );
  };
  },
);

export default PluginPageView;
</script>
