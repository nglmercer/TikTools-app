<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { PluginStatus } from '../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../shared/messages.ts';
import { toSettingValues } from '../../../shared/settings-values.ts';
import type { PluginSettingsState } from '../../types.ts';
import { optionFields } from '../../../automation/plugins/declarative.ts';
import { SchemaForm } from '../../components/ui/SchemaForm.vue';
import { t, type Locale } from '../../i18n.ts';

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

export const PluginSettingsForm = defineVueComponent<PluginSettingsFormProps>(
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

export default PluginSettingsForm;
</script>
