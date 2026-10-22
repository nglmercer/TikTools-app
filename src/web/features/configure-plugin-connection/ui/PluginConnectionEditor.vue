<script setup lang="ts">
import { computed, onMounted, ref, toRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../../shared/messages.ts';
import type { PluginCredentialState, TokenSetupOptions } from '../../../../shared/token-import.ts';
import { connectionSummaryRows, isLoopbackUrl } from '@/entities/plugin/lib/connection-settings.ts';
import { fieldOptionsFor, optionSources } from '@/entities/plugin/lib/plugin-options.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import type { PluginSettingsState } from '@/types.ts';
import { useConnectionAutosave } from '../model/use-connection-autosave.ts';
import ProvisionTokenDialog from './ProvisionTokenDialog.vue';

const props = withDefaults(
  defineProps<{
    pluginId: string;
    pluginName: string;
    settings?: PluginSettingsState;
    connection?: PluginConnectionState;
    actionOptions: Readonly<Record<string, ActionOptionItem[]>>;
    supportsProvisioning?: boolean;
    supportsTokenImport?: boolean;
    supportsTokenDiscovery?: boolean;
    provisionState?: PluginCredentialState;
    openMediaPicker?: OpenMediaPicker;
  }>(),
  {
    settings: undefined,
    connection: undefined,
    provisionState: undefined,
    openMediaPicker: undefined,
  },
);

const emit = defineEmits<{
  loadSettings: [pluginId: string];
  loadOptions: [source: string];
  save: [pluginId: string, values: PluginSettingValues];
  test: [pluginId: string];
  importToken: [pluginId: string, options?: TokenSetupOptions];
  provision: [pluginId: string, username: string, password: string];
}>();

const { t } = useI18n();
const provisionOpen = ref(false);

const autosave = useConnectionAutosave({
  pluginId: toRef(props, 'pluginId'),
  settings: toRef(props, 'settings'),
  connection: toRef(props, 'connection'),
  save: (values) => emit('save', props.pluginId, values),
});
const { testing, saveState, values, url, urlKey, urlInvalid, dirty, summary } = autosave;

const fieldOptions = computed(() => fieldOptionsFor(props.settings?.uiHints, props.actionOptions));
const formValues = computed<JsonObject>({ get: () => values.value, set: (next) => autosave.edit(next) });
const fieldErrors = computed(() => (urlInvalid.value && urlKey.value ? { [urlKey.value]: t('plugins.connection.invalidUrl') } : {}));
const summaryRows = computed(() =>
  connectionSummaryRows(values.value, props.settings?.schema, props.settings?.uiHints, urlKey.value, localizedText),
);
const working = computed(() => props.provisionState?.working === true);
const importDisabled = computed(
  () => working.value || testing.value || dirty.value || saveState.value === 'saving' || !props.settings,
);
const provisionResult = computed(() =>
  props.provisionState && !props.provisionState.working && props.provisionState.message
    ? { ok: props.provisionState.ok, message: props.provisionState.message }
    : undefined,
);

const tone = computed(() => {
  if (dirty.value || testing.value || !props.connection) return 'idle';
  return props.connection.ok ? 'ok' : 'error';
});

const statusText = computed(() => {
  const connection = props.connection;
  if (dirty.value) return t('plugins.connection.testToVerify');
  if (testing.value) return t('plugins.connection.testing');
  if (!connection) return t('connections.servers.notConfigured');
  if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
  return connection.ok ? t('connections.servers.connectedIn', { ms: connection.latencyMs }) : t('connections.servers.failed');
});

function test(): void {
  testing.value = true;
  emit('test', props.pluginId);
}

function requestOptions(): void {
  for (const source of optionSources(props.settings?.uiHints)) emit('loadOptions', source);
}

function startImport(options?: TokenSetupOptions): void {
  autosave.reset();
  emit('importToken', props.pluginId, options);
}

function onFocusout(event: FocusEvent): void {
  const root = event.currentTarget as HTMLElement;
  const next = event.relatedTarget as Node | null;
  if (next && root.contains(next)) return;
  requestAnimationFrame(() => {
    if (!root.contains(document.activeElement)) autosave.flush();
  });
}

watch(
  () => props.pluginId,
  () => {
    provisionOpen.value = false;
    if (!props.settings) emit('loadSettings', props.pluginId);
    requestOptions();
  },
);
watch(() => props.settings?.uiHints, requestOptions);

onMounted(() => {
  if (!props.settings) emit('loadSettings', props.pluginId);
  requestOptions();
  if (!props.connection) test();
});
</script>

<template>
  <div
    class="connection-editor"
    @focusout="onFocusout"
  >
    <header class="connection-editor__status">
      <span
        class="connection-editor__dot"
        :class="`connection-editor__dot--${tone}`"
      />
      <span
        v-if="summary && connection?.ok"
        class="connection-editor__url"
      >{{ t('plugins.connection.connectedTo', { url }) }}</span>
      <span v-else>{{ statusText }}</span>
      <span
        v-if="saveState !== 'idle'"
        class="connection-editor__save"
        :class="`connection-editor__save--${saveState}`"
        role="status"
      >{{ t(`plugins.connection.save.${saveState}`) }}</span>
    </header>

    <p
      v-if="connection?.gameplayReady !== undefined && !dirty"
      class="connection-editor__note"
    >
      {{ connection.gameplayReady ? t('plugins.connection.gameplayReady') : t('connections.servers.apiNotReady') }}
    </p>
    <p
      v-if="connection?.details && !dirty"
      class="connection-editor__details"
      :class="{ 'connection-editor__details--ok': connection.ok }"
      role="status"
    >
      {{ connection.details }}
    </p>

    <template v-if="summary && connection?.ok">
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
          @click="test"
        >
          {{ testing ? t('plugins.connection.testing') : t('plugins.connection.testAgain') }}
        </BaseButton>
        <BaseButton @click="autosave.editing.value = true">
          {{ t('plugins.connection.edit') }}
        </BaseButton>
      </div>
    </template>

    <template v-else>
      <p
        v-if="connection && !connection.ok && !dirty && !urlInvalid && connection.error"
        class="connection-editor__details"
        role="status"
      >
        {{ connection.error }}
      </p>
      <BaseSchemaForm
        v-if="settings && !(supportsTokenImport && working)"
        v-model="formValues"
        :schema="settings.schema"
        :ui-hints="settings.uiHints"
        :field-options="fieldOptions"
        :field-errors="fieldErrors"
        :open-media-picker="openMediaPicker"
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
          :disabled="!settings || testing || urlInvalid"
          @click="test"
        >
          {{ testing ? t('plugins.connection.testing') : t('plugins.connection.test') }}
        </BaseButton>
      </div>
      <p class="connection-editor__note">
        {{ url && isLoopbackUrl(url) ? t('plugins.connection.localNote') : t('plugins.connection.hint') }}
      </p>
    </template>

    <section
      v-if="supportsTokenImport"
      class="connection-editor__token"
    >
      <div class="connection-editor__actions">
        <template v-if="supportsTokenDiscovery">
          <BaseButton
            variant="primary"
            :disabled="importDisabled"
            @click="startImport({ mode: 'auto' })"
          >
            {{ working ? t('plugins.connection.importing') : t('plugins.connection.autoConnect') }}
          </BaseButton>
          <BaseButton
            :disabled="importDisabled"
            @click="startImport({ mode: 'folder' })"
          >
            {{ t('plugins.connection.selectFolder') }}
          </BaseButton>
        </template>
        <BaseButton
          :disabled="importDisabled"
          @click="startImport()"
        >
          {{ working ? t('plugins.connection.importing') : t('plugins.connection.importToken') }}
        </BaseButton>
      </div>
      <p class="connection-editor__note">
        {{ supportsTokenDiscovery ? t('plugins.connection.autoConnectHint') : t('plugins.connection.importTokenHint') }}
      </p>
      <div
        v-if="provisionState?.candidates?.length"
        class="connection-editor__candidates"
      >
        <p class="connection-editor__note">
          {{ t('plugins.connection.chooseConfiguration') }}
        </p>
        <ul class="connection-editor__candidate-list">
          <li
            v-for="candidate in provisionState.candidates"
            :key="candidate.id"
          >
            <span class="connection-editor__candidate">{{ candidate.label }}</span>
            <BaseButton
              size="sm"
              :disabled="importDisabled"
              @click="startImport({ mode: 'candidate', candidateId: candidate.id })"
            >
              {{ t('plugins.connection.useConfiguration') }}
            </BaseButton>
          </li>
        </ul>
      </div>
      <p
        v-if="provisionState?.message"
        class="connection-editor__details"
        :class="{ 'connection-editor__details--ok': provisionState.ok }"
        role="status"
      >
        {{ provisionState.message }}
      </p>
    </section>

    <div
      v-if="supportsProvisioning"
      class="connection-editor__actions"
    >
      <BaseButton :disabled="importDisabled" @click="provisionOpen = true">
        {{ t('plugins.connection.getToken') }}
      </BaseButton>
      <ProvisionTokenDialog
        v-model:open="provisionOpen"
        :plugin-name="pluginName"
        :working="working"
        :result="provisionResult"
        @submit="(username, password) => emit('provision', pluginId, username, password)"
      />
    </div>
  </div>
</template>

<style scoped>
.connection-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.connection-editor__status {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-weight: var(--font-weight-semibold);
}

.connection-editor__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-subtle);
}

.connection-editor__dot--ok {
  background: var(--color-success);
}

.connection-editor__dot--error {
  background: var(--color-danger);
}

.connection-editor__url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.connection-editor__save {
  margin-left: auto;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-regular);
}

.connection-editor__save--saved {
  color: var(--color-success);
}

.connection-editor__save--error {
  color: var(--color-danger-hover);
}

.connection-editor__note {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}

.connection-editor__details {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
  overflow-wrap: anywhere;
}

.connection-editor__details--ok {
  border-color: var(--color-success);
  color: var(--color-success);
}

.connection-editor__summary {
  margin: 0;
}

.connection-editor__row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-5);
  padding: var(--space-3) 0;
}

.connection-editor__row + .connection-editor__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.connection-editor__row dt {
  color: var(--color-text-muted);
}

.connection-editor__row dd {
  margin: 0;
}

.connection-editor__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.connection-editor__token {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding-top: var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.connection-editor__candidate-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 20rem;
  margin: var(--space-2) 0 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.connection-editor__candidate-list li {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.connection-editor__candidate {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
