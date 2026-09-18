<script lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { JsonObject } from '../../automation/types.ts';
import {
  optionFields,
  type PluginConnectionState,
} from '../../automation/plugins/declarative.ts';
import {
  AUTOSAVE_CONFIRM_TIMEOUT_MS,
  AUTOSAVE_DEBOUNCE_MS,
  connectionSummaryRows,
  echoConfirmsSave,
  echoNeedsResave,
  findServerUrlKey,
  focusStayedInside,
  isSelectFocusSource,
  isHttpUrl,
  isLoopbackUrl,
  secretSettingKeys,
  settingsEqual,
  settingsMatch,
  shouldShowSummary,
  stableSettingsJson,
  withSchemaDefaults,
} from './plugin-connection-logic.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { SchemaForm } from './ui/SchemaForm.vue';
import { ProvisionTokenModal } from './ProvisionTokenModal.vue';
import { t, type Locale } from '../i18n.ts';

type PluginConnectionCardProps = {
  locale: Locale;
  pluginId: string;
  pluginName: string;
  settingsState?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Record<string, ActionOptionItem[]>;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string) => void;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  supportsProvisioning?: boolean;
  provisionState?: { working: boolean; ok: boolean; message: string };
  onProvisionToken?: (id: string, username: string, password: string) => void;
  /** Stretch full-width (Connections tab) instead of the centered 720px page column. */
  inline?: boolean;
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
 * Host-owned connection card shared by the plugin page (`connection`
 * section) and the Connections tab server list. One component, one
 * behavior: centered connection card embedding the full settings form
 * (advanced fields collapsed by SchemaForm), inline `format: "uri"`
 * validation, autosaved edits, compact summary once the probe succeeds,
 * plus the one-click API token provisioning entry point when the
 * manifest declares a supported strategy.
 *
 * Section kinds form a fixed widget set: every branch is host code
 * rendering manifest data as text and form controls. There is no markup,
 * script, or component indirection from the manifest.
 */
export const PluginConnectionCard = defineVueComponent<PluginConnectionCardProps>(
  ['locale', 'pluginId', 'pluginName', 'settingsState', 'connection', 'actionOptions', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker', 'supportsProvisioning', 'provisionState', 'onProvisionToken', 'inline'],
  (props) => {
  const draft = ref<JsonObject | null>(null);
  const editing = ref(false);
  const testing = ref(false);
  const showProvision = ref(false);
  const saveState = ref<SaveState>('idle');
  const lastSeenProbe = ref(0);
  const lastSent = ref<PluginSettingValues | null>(null);
  const lastSentJson = ref<string | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let confirmTimer: ReturnType<typeof setTimeout> | null = null;
  const listSources = computed(() => {
    const sources: string[] = [];
    for (const field of optionFields(props.settingsState?.uiHints)) {
      if (!sources.includes(field.source)) sources.push(field.source);
    }
    return sources;
  });
  const dynamicFields = computed(() => optionFields(props.settingsState?.uiHints));
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
  const showSummary = computed(() => shouldShowSummary({
    editing: editing.value,
    dirty: dirty.value,
    connectionOk: props.connection?.ok === true,
    hasSettings: props.settingsState !== undefined,
  }));
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
    props.onSaveSettings(props.pluginId, payload);
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

  // Internal focus moves (password input → Show button, input → select)
  // must not flush the debounced autosave. Only a real exit from the card —
  // or the normal debounce — saves. `relatedTarget` covers normal focus
  // transitions; the rAF fallback covers WebViews that report null.
  const onConnectionCardFocusOut = (event: FocusEvent): void => {
    const card = event.currentTarget as HTMLElement;
    if (focusStayedInside(card, event.relatedTarget, document.activeElement)) {
      return;
    }

    // Native select popups live outside the DOM focus tree: focusout around
    // a selection commit is unreliable and may fire before input/change
    // update the draft. Let the selection events commit first; the normal
    // debounce persists the draft.
    if (isSelectFocusSource(event.target)) {
      return;
    }

    requestAnimationFrame(() => {
      if (focusStayedInside(card, null, document.activeElement)) {
        return;
      }
      flushSave();
    });
  };

  const onConnectionFormChange = (next: JsonObject): void => {
    draft.value = next;
    editing.value = true;
    if (saveState.value === 'saved' || saveState.value === 'error') saveState.value = 'idle';
    scheduleSave();
  };

  const testConnection = (): void => {
    testing.value = true;
    props.onTestConnection(props.pluginId);
  };

  const requestListSources = (): void => {
    for (const source of listSources.value) props.onGetActionOptions(source);
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  };

  onMounted(() => {
    if (!props.settingsState) props.onGetSettings(props.pluginId);
    requestListSources();
    // First-run auto-detect: probe once when no result exists yet.
    if (!props.connection) {
      testConnection();
    }
  });
  onUnmounted(clearSaveTimers);
  watch(() => props.pluginId, () => {
    draft.value = null;
    editing.value = false;
    testing.value = false;
    showProvision.value = false;
    saveState.value = 'idle';
    lastSent.value = null;
    lastSentJson.value = null;
    clearSaveTimers();
    if (!props.settingsState) props.onGetSettings(props.pluginId);
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
    // Secret-aware: a typed secret vs its redacted echo converges (no
    // resave); only genuine differences schedule another save.
    const state = props.settingsState;
    const next = draft.value;
    if (state && next && echoNeedsResave(
      toSettingValues(next),
      toSettingValues(values),
      state.schema,
      secretKeys.value,
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

  /**
   * One-click API token provisioning entry point, rendered only when the
   * manifest declares a supported strategy. The admin login lives in its
   * own modal so operator credentials are never confused with the plugin
   * settings around them; progress and outcome come from controller state.
   */
  const renderProvision = () => {
    const provision = props.onProvisionToken;
    if (!props.supportsProvisioning || !provision) return null;
    const pluginId = props.pluginId;
    const state = props.provisionState;
    const result = state && !state.working && state.message
      ? { ok: state.ok, message: state.message }
      : undefined;
    return (
      <>
        <div class="plg-connect__actions">
          <button
            type="button"
            class="plg-btn plg-btn--sm"
            onClick={() => { showProvision.value = true; }}
          >
            {t(props.locale, 'pluginGetApiToken')}
          </button>
        </div>
        {showProvision.value && (
          <ProvisionTokenModal
            locale={props.locale}
            pluginName={props.pluginName}
            working={state?.working ?? false}
            result={result}
            onSubmit={(username, password) => provision(pluginId, username, password)}
            onClose={() => { showProvision.value = false; }}
          />
        )}
      </>
    );
  };

  return () => {
    const locale = props.locale;
    const state = props.settingsState;
    const connection = props.connection;
    if (showSummary.value && connection?.ok) {
      return (
        <section class={`plg-connect${props.inline ? ' plg-connect--inline' : ''}`}>
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
            {renderProvision()}
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
      <section class={`plg-connect${props.inline ? ' plg-connect--inline' : ''}`}>
        <div class="plg-connect__card" onFocusout={onConnectionCardFocusOut}>
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
          {renderProvision()}
        </div>
      </section>
    );
  };
  },
);

export default PluginConnectionCard;
</script>
