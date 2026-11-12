<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionEditorProps } from '../model/connection-editor-types.ts';
import type { ConnectionPresentation } from '../model/use-connection-presentation.ts';
import type { ConnectionAutosave } from '../model/use-connection-autosave.ts';
import ConnectionServiceNotes from './ConnectionServiceNotes.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import DiagnosticStatusAction from '@/shared/diagnostics/DiagnosticStatusAction.vue';
const props = defineProps<{
  source: Pick<PluginConnectionEditorProps, 'pluginId' | 'pluginName' | 'settings' | 'connection' | 'supportsTokenImport' | 'openMediaPicker'>;
  view: Pick<ConnectionPresentation, 'visibleSchema' | 'fieldOptions' | 'fieldErrors' | 'summaryRows' | 'localHint' | 'tone' | 'statusText'>;
  state: Pick<ConnectionAutosave, 'testing' | 'saveState' | 'values' | 'url' | 'urlInvalid' | 'dirty' | 'summary'>;
  guided: boolean;
  advanced: boolean;
  needsAuthorization?: boolean;
  working: boolean;
}>();
const emit = defineEmits<{
  edit: [values: JsonObject];
  test: [];
  connect: [];
  editSettings: [];
  toggleAdvanced: [];
}>();
const { t } = useI18n();
const { visibleSchema, fieldOptions, fieldErrors, summaryRows, localHint, tone, statusText } = props.view;
const { testing, saveState, values, url, urlInvalid, dirty, summary } = props.state;
</script>

<template>
  <header class="connection-editor__status">
    <BaseStatus
      v-if="summary && source.connection?.ok"
      :label="t('plugins.connection.connectedTo', { url })"
      :tone="tone === 'ok' ? 'success' : 'neutral'"
    />
    <DiagnosticStatusAction
      v-else-if="tone === 'error'"
      :label="statusText"
      source="plugins"
      :entity-id="source.pluginId"
    />
    <BaseStatus
      v-else
      :label="statusText"
      :tone="tone === 'ok' ? 'success' : 'neutral'"
    />
    <span
      v-if="saveState !== 'idle'"
      class="connection-editor__save"
      :class="`connection-editor__save--${saveState}`"
      role="status"
    >{{ t(`plugins.connection.save.${saveState}`) }}</span>
  </header>

  <ConnectionServiceNotes
    :connection="source.connection"
    :guided="guided"
    :dirty="dirty"
  />
  <p
    v-if="source.connection?.ok && source.connection.details && !dirty"
    class="connection-editor__details"
    :class="{ 'connection-editor__details--ok': source.connection.ok }"
    role="status"
  >
    {{ source.connection.details }}
  </p>

  <template v-if="summary && source.connection?.ok">
    <dl
      v-if="summaryRows.length"
      class="connection-editor__summary"
    >
      <div
        v-for="row in summaryRows"
        :key="row.key"
        class="connection-editor__row"
      >
        <dt>{{ row.label }}</dt>
        <dd>{{ row.value }}</dd>
      </div>
    </dl>
    <div class="connection-editor__actions">
      <BaseButton
        variant="primary"
        :disabled="testing"
        @click="emit('test')"
      >
        {{ testing ? t('plugins.connection.testing') : t('plugins.connection.testAgain') }}
      </BaseButton>
      <BaseButton @click="emit('editSettings')">
        {{ t('plugins.connection.edit') }}
      </BaseButton>
    </div>
  </template>

  <template v-else>
    <p
      v-if="source.connection && !source.connection.ok && !dirty && !urlInvalid && source.connection.error && !needsAuthorization && guided && !source.connection.reachable && !advanced"
      class="connection-editor__details"
      role="status"
    >
      {{ t('plugins.connection.openGuided', { name: source.pluginName }) }}
    </p>
    <BaseSchemaForm
      v-if="source.settings && !(source.supportsTokenImport && working)"
      :model-value="values"
      switches
      :schema="visibleSchema"
      :ui-hints="source.settings.uiHints"
      :field-options="fieldOptions"
      :field-errors="fieldErrors"
      :open-media-picker="source.openMediaPicker"
      @update:model-value="emit('edit', $event)"
    />
    <p
      v-else
      class="connection-editor__note"
    >
      {{ t('plugins.connection.testing') }}
    </p>
    <div class="connection-editor__actions">
      <BaseButton
        variant="primary"
        :disabled="!source.settings || testing || urlInvalid || (guided && (dirty || saveState === 'saving'))"
        :title="guided ? undefined : localHint"
        @click="guided ? emit('connect') : emit('test')"
      >
        {{ testing ? t('plugins.connection.testing') : t(guided ? 'plugins.connection.connectGuided' : 'plugins.connection.test', { name: source.pluginName }) }}
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
  </template>

  <slot name="credentials" />
</template>
