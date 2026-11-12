<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionEditorProps } from '../model/connection-editor-types.ts';
import type { ConnectionPresentation } from '../model/use-connection-presentation.ts';
import type { ConnectionAutosave } from '../model/use-connection-autosave.ts';
import ConnectionServiceNotes from './ConnectionServiceNotes.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
const props = defineProps<{
  source: Pick<PluginConnectionEditorProps, 'pluginName' | 'settings' | 'connection' | 'openMediaPicker'>;
  view: Pick<ConnectionPresentation, 'fieldOptions' | 'fieldErrors' | 'failureText' | 'connectionSchema' | 'defaultsSchema' | 'hasDefaults' | 'fieldIcons'>;
  state: Pick<ConnectionAutosave, 'testing' | 'saveState' | 'values' | 'urlInvalid' | 'dirty'>;
  guided: boolean;
  advanced: boolean;
}>();
const emit = defineEmits<{
  edit: [values: JsonObject];
  test: [];
  connect: [];
  editSettings: [];
  toggleAdvanced: [];
}>();
const { t } = useI18n();
const { fieldOptions, fieldErrors, failureText, connectionSchema, defaultsSchema, hasDefaults, fieldIcons } = props.view;
const { testing, saveState, values, urlInvalid, dirty } = props.state;
</script>

<template>
  <div class="connection-editor__grid">
    <BaseSection :title="t('plugins.connection.sections.connection')">
      <ConnectionServiceNotes
        :connection="source.connection"
        :guided="guided"
        :dirty="dirty"
      />
      <BaseSchemaForm
        v-if="source.settings"
        :model-value="values"
        compact
        switches
        :schema="connectionSchema"
        :ui-hints="source.settings.uiHints"
        :field-options="fieldOptions"
        :field-errors="fieldErrors"
        :field-icons="fieldIcons"
        :open-media-picker="source.openMediaPicker"
        @update:model-value="emit('edit', $event)"
      />
      <p
        v-else
        class="connection-editor__note"
      >
        {{ t('plugins.connection.testing') }}
      </p>

      <div
        v-if="guided && !advanced"
        class="connection-editor__actions"
      >
        <BaseButton
          variant="primary"
          :disabled="!source.settings || testing || urlInvalid || dirty || saveState === 'saving'"
          @click="emit('connect')"
        >
          {{ testing ? t('plugins.connection.testing') : t('plugins.connection.connectGuided', { name: source.pluginName }) }}
        </BaseButton>
      </div>
      <BaseButton
        v-if="guided"
        variant="ghost"
        :aria-expanded="advanced"
        @click="emit('toggleAdvanced')"
      >
        {{ t('plugins.connection.advanced') }}
      </BaseButton>

      <slot name="credentials" />
    </BaseSection>

    <BaseSection
      v-if="hasDefaults"
      :title="t('plugins.connection.sections.defaults')"
    >
      <BaseSchemaForm
        v-if="source.settings"
        :model-value="values"
        compact
        switches
        :schema="defaultsSchema"
        :ui-hints="source.settings.uiHints"
        :field-options="fieldOptions"
        :field-errors="fieldErrors"
        :field-icons="fieldIcons"
        :open-media-picker="source.openMediaPicker"
        @update:model-value="emit('edit', $event)"
      />
    </BaseSection>
  </div>

  <p
    v-if="guided && failureText"
    class="connection-editor__failure"
    role="status"
  >
    {{ failureText }}
  </p>
  <span
    v-if="saveState !== 'idle'"
    class="connection-editor__save"
    :class="`connection-editor__save--${saveState}`"
    role="status"
  >{{ t(`plugins.connection.save.${saveState}`) }}</span>
</template>