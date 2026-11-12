import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { PluginSettingValues } from '../../../../shared/messages.ts';
import { toSettingValues } from '../../../../shared/settings-values.ts';
import {
  AUTOSAVE_CONFIRM_TIMEOUT_MS,
  AUTOSAVE_DEBOUNCE_MS,
  echoConfirmsSave,
  echoNeedsResave,
  findServerUrlKey,
  isHttpUrl,
  secretSettingKeys,
  settingsEqual,
  settingsMatch,
  shouldShowSummary,
  stableSettingsJson,
} from '@/entities/plugin/lib/connection-settings.ts';
import { withSchemaDefaults } from '@/shared/ui/schema-form/lib/schema-fields.ts';
import type { PluginSettingsState } from '@/types.ts';

export type SaveState = 'idle' | 'saving' | 'saved' | 'error';

interface AutosaveSource {
  pluginId: Readonly<Ref<string>>;
  settings: Readonly<Ref<PluginSettingsState | undefined>>;
  connection: Readonly<Ref<PluginConnectionState | undefined>>;
  save: (values: PluginSettingValues) => void;
}

export function useConnectionAutosave(source: AutosaveSource) {
  const draft = ref<JsonObject | null>(null);
  const editing = ref(false);
  const testing = ref(false);
  const saveState = ref<SaveState>('idle');
  const lastSent = ref<PluginSettingValues | null>(null);
  let lastSentJson: string | null = null;
  let lastSeenProbe = 0;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let confirmTimer: ReturnType<typeof setTimeout> | null = null;

  const schema = computed(() => source.settings.value?.schema);
  const urlKey = computed(() => findServerUrlKey(schema.value));
  const secretKeys = computed(() => secretSettingKeys(schema.value, source.settings.value?.uiHints));
  const values = computed(() => withSchemaDefaults(draft.value ?? source.settings.value?.values ?? {}, schema.value));
  const url = computed(() => {
    const raw = urlKey.value ? values.value[urlKey.value] : undefined;
    return typeof raw === 'string' ? raw : '';
  });
  const urlInvalid = computed(() => urlKey.value !== undefined && !isHttpUrl(url.value));
  const dirty = computed(() => {
    const state = source.settings.value;
    if (!state || !draft.value) return false;
    return !settingsMatch(
      withSchemaDefaults(toSettingValues(draft.value), state.schema),
      withSchemaDefaults(toSettingValues(state.values), state.schema),
      secretKeys.value,
    );
  });
  const summary = computed(() =>
    shouldShowSummary({
      editing: editing.value,
      dirty: dirty.value,
      connectionOk: source.connection.value?.ok === true,
      hasSettings: source.settings.value !== undefined,
    }),
  );

  function clearTimers(): void {
    if (saveTimer !== null) clearTimeout(saveTimer);
    if (confirmTimer !== null) clearTimeout(confirmTimer);
    saveTimer = confirmTimer = null;
  }

  function flush(): void {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = null;
    if (!source.settings.value || !draft.value) return;
    const payload = toSettingValues(draft.value);
    const json = stableSettingsJson(payload);
    if (json === lastSentJson) return;
    lastSent.value = payload;
    lastSentJson = json;
    saveState.value = 'saving';
    source.save(payload);
    if (confirmTimer !== null) clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => {
      confirmTimer = null;
      if (saveState.value === 'saving') saveState.value = 'error';
    }, AUTOSAVE_CONFIRM_TIMEOUT_MS);
  }

  function schedule(): void {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = setTimeout(flush, AUTOSAVE_DEBOUNCE_MS);
  }

  function edit(next: JsonObject): void {
    draft.value = next;
    editing.value = true;
    if (saveState.value === 'saved' || saveState.value === 'error') saveState.value = 'idle';
    schedule();
  }

  function reset(): void {
    clearTimers();
    draft.value = null;
    editing.value = false;
    testing.value = false;
    saveState.value = 'idle';
    lastSent.value = null;
    lastSentJson = null;
  }

  watch(source.pluginId, reset);

  watch(
    () => source.settings.value?.values,
    (echo) => {
      if (!echo) return;
      const sent = lastSent.value;
      if ((saveState.value === 'saving' || saveState.value === 'error') && sent && echoConfirmsSave(echo, sent, secretKeys.value)) {
        saveState.value = 'saved';
        if (confirmTimer !== null) clearTimeout(confirmTimer);
        confirmTimer = null;
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
      const state = source.settings.value;
      if (state && draft.value && echoNeedsResave(toSettingValues(draft.value), toSettingValues(echo), state.schema, secretKeys.value)) {
        schedule();
      }
    },
  );

  watch(source.connection, (connection) => {
    if (!connection || connection.at === lastSeenProbe) return;
    lastSeenProbe = connection.at;
    testing.value = false;
    editing.value = !connection.ok;
  });

  onBeforeUnmount(clearTimers);

  return { draft, editing, testing, saveState, values, url, urlKey, urlInvalid, dirty, summary, edit, flush, reset, clearTimers };
}

export type ConnectionAutosave = ReturnType<typeof useConnectionAutosave>;
