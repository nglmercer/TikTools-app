<script lang="tsx">
import { computed, onMounted, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { JsonObject } from '../../automation/types.ts';
import {
  optionFields,
  type PluginConnectionState,
} from '../../automation/plugins/declarative.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { PluginSettingsState } from '../types.ts';
import { Button } from './ui/Button.vue';
import { Modal, ModalActions } from './ui/Modal.vue';
import { SchemaForm } from './ui/SchemaForm.vue';
import { t, type Locale } from '../i18n.ts';

type PluginConnectionModalProps = {
  locale: Locale;
  pluginId: string;
  pluginName: string;
  state?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Record<string, ActionOptionItem[]>;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string) => void;
  onTestConnection: (id: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onClose: () => void;
};

/**
 * Generic connection dialog for HTTP-integrated plugins (SonicBoom and any
 * future declarative server). Edits the plugin's own settings through the
 * host-owned form renderer and probes its declared health endpoint. Tokens
 * render masked and round-trip as placeholders, exactly like settings.
 */
export const PluginConnectionModal = defineVueComponent<PluginConnectionModalProps>(
  ['locale', 'pluginId', 'pluginName', 'state', 'connection', 'actionOptions', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onTestConnection', 'onOpenMediaPicker', 'onClose'],
  (props) => {
  const draft = ref<JsonObject | null>(null);
  const testing = ref(false);
  const lastSeenProbe = ref(0);
  const dynamicFields = computed(() => optionFields(props.state?.uiHints));
  const fieldOptions = computed(() => {
    const merged: Record<string, Array<{ value: string; label: string }>> = {};
    for (const field of dynamicFields.value) {
      const options = props.actionOptions[field.source];
      if (options && options.length > 0) merged[field.key] = options;
    }
    return merged;
  });

  onMounted(() => {
    if (!props.state) props.onGetSettings(props.pluginId);
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
    // First-run auto-detect: probe once when no result exists yet.
    if (!props.connection) props.onTestConnection(props.pluginId);
  });
  watch(() => props.state?.uiHints, () => {
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  });
  watch(() => props.connection, (connection) => {
    if (connection && connection.at !== lastSeenProbe.value) {
      lastSeenProbe.value = connection.at;
      testing.value = false;
    }
  });

  const toSettingValues = (value: JsonObject): PluginSettingValues => {
    const clean: PluginSettingValues = {};
    for (const [key, entry] of Object.entries(value)) {
      if (typeof entry === 'string' || typeof entry === 'number' || typeof entry === 'boolean') clean[key] = entry;
    }
    return clean;
  };

  return () => {
  const locale = props.locale;
  const state = props.state;
  const values = draft.value ?? state?.values ?? {};
  const connection = props.connection;
  return (
    <Modal
      title={t(locale, 'pluginConnectTitle', { name: props.pluginName })}
      description={t(locale, 'pluginConnectionHint')}
      size="md"
      onClose={props.onClose}
      footer={
        <ModalActions>
          <Button
            variant="soft"
            disabled={testing.value}
            onClick={() => {
              testing.value = true;
              props.onTestConnection(props.pluginId);
            }}
          >
            {testing.value ? t(locale, 'pluginTestingConnection') : t(locale, 'pluginTestConnection')}
          </Button>
          <Button
            variant="primary"
            disabled={!state}
            onClick={() => props.onSaveSettings(props.pluginId, toSettingValues(values))}
          >
            {t(locale, 'pluginSettingsSave')}
          </Button>
        </ModalActions>
      }
    >
      <div class="plg-form">
        {connection && (
          <div class={`plg-alert${connection.ok ? ' plg-alert--ok' : ''}`} role="status">
            {connection.ok
              ? t(locale, 'pluginConnectedIn', { ms: connection.latencyMs })
              : (connection.error || t(locale, 'pluginConnectionFailed'))}
          </div>
        )}
        {state ? (
          <SchemaForm
            locale={locale}
            schema={state.schema}
            uiHints={state.uiHints}
            value={values}
            fieldOptions={fieldOptions.value}
            onChange={(next) => { draft.value = next; }}
            onOpenMediaPicker={props.onOpenMediaPicker}
          />
        ) : (
          <span class="plg-group-note">{t(locale, 'pluginTestingConnection')}</span>
        )}
      </div>
    </Modal>
  );
  };
  },
);

export default PluginConnectionModal;
</script>
