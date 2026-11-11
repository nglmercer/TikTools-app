<script setup lang="ts">
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, markRaw, onBeforeUnmount, onMounted, ref, toRef, watch, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../../shared/messages.ts';
import type { PluginCredentialState, TokenSetupOptions } from '../../../../shared/token-import.ts';
import { connectionSummaryRows, isLoopbackUrl, secretSettingKeys } from '@/entities/plugin/lib/connection-settings.ts';
import { fieldOptionsFor, optionSources } from '@/entities/plugin/lib/plugin-options.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconClock from '@/shared/ui/icon/icons/IconClock.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import IconFolder from '@/shared/ui/icon/icons/IconFolder.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconHash from '@/shared/ui/icon/icons/IconHash.vue';
import IconKey from '@/shared/ui/icon/icons/IconKey.vue';
import IconMapPin from '@/shared/ui/icon/icons/IconMapPin.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
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
    /** Guided `service-info` connection; inferred from a probe result when omitted. */
    guided?: boolean;
    supportsTokenImport?: boolean;
    supportsTokenDiscovery?: boolean;
    provisionState?: PluginCredentialState;
    openMediaPicker?: OpenMediaPicker;
    /** `sections` puts connection and default settings side by side, as on the server connections page. */
    layout?: 'stacked' | 'sections';
  }>(),
  {
    settings: undefined,
    connection: undefined,
    guided: undefined,
    provisionState: undefined,
    openMediaPicker: undefined,
    layout: 'stacked',
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
let modelRefresh: ReturnType<typeof setTimeout> | undefined;
onBeforeUnmount(() => clearTimeout(modelRefresh));
const advanced = ref(false);
const connectRequested = ref(false);
const guided = computed(() => props.guided ?? props.connection?.serverInfo !== undefined);
const serverInfo = computed(() => props.connection?.serverInfo);
const needsAuthorization = computed(() => serverInfo.value?.authRequired && !serverInfo.value.authorized);
const visibleSchema = computed(() => {
  const schema = props.settings?.schema;
  if (!schema || !guided.value || advanced.value) return schema ?? {};
  const properties = schema.properties as JsonObject | undefined;
  return { ...schema, properties: Object.fromEntries(Object.entries(properties ?? {}).filter(([key]) => key !== 'apiToken')) };
});
function connect(): void {
  connectRequested.value = true;
  test();
}
watch(() => props.connection?.at, () => {
  if (connectRequested.value && needsAuthorization.value) provisionOpen.value = true;
  connectRequested.value = false;
  clearTimeout(modelRefresh);
  if (guided.value && props.connection?.ok && ['idle', 'downloading', 'loading'].includes(serverInfo.value?.modelStatus ?? '')) {
    modelRefresh = setTimeout(test, 3000);
  }
});

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
const localHint = computed(() => (url.value && isLoopbackUrl(url.value) ? t('plugins.connection.localNote') : t('plugins.connection.hint')));
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
  if (guided.value && needsAuthorization.value) return t('plugins.connection.authorize');
  if (guided.value && connection.ok) return t('plugins.connection.connected');
  if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
  return connection.ok ? t('connections.servers.connectedIn', { ms: connection.latencyMs }) : t('connections.servers.failed');
});

/** Slim failure line for the sectioned layout; the panel header already shows the status. */
const failureText = computed(() => {
  const connection = props.connection;
  if (dirty.value || testing.value || !connection || connection.ok) return '';
  if (connection.details) return connection.details;
  if (connection.error) return connection.error;
  if (guided.value && !connection.reachable) return t('plugins.connection.openGuided', { name: props.pluginName });
  return statusText.value;
});

/** Keys that describe how to reach the service; everything else is a gameplay default. */
const connectionKeys = computed(() => new Set([urlKey.value, ...secretSettingKeys(props.settings?.schema, props.settings?.uiHints)]));
function schemaSubset(schema: JsonObject, keep: (key: string) => boolean): JsonObject {
  const properties = schema.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return schema;
  return { ...schema, properties: Object.fromEntries(Object.entries(properties as JsonObject).filter(([key]) => keep(key))) };
}
const connectionSchema = computed(() => schemaSubset(visibleSchema.value, (key) => connectionKeys.value.has(key)));
const defaultsSchema = computed(() => schemaSubset(visibleSchema.value, (key) => !connectionKeys.value.has(key)));
const hasDefaults = computed(() => {
  const properties = defaultsSchema.value.properties;
  return Boolean(properties && typeof properties === 'object' && !Array.isArray(properties) && Object.keys(properties).length > 0);
});
const fieldIcons = markRaw<Readonly<Record<string, Component>>>({
  apiToken: IconKey,
  serverUrl: IconGlobe,
  playerId: IconUser,
  positionMode: IconMapPin,
  quantity: IconHash,
  lifetimeSeconds: IconClock,
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
    advanced.value = false;
    connectRequested.value = false;
    clearTimeout(modelRefresh);
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

defineExpose({ flush: autosave.flush });
</script>

<template>
  <div
    class="connection-editor"
    :class="{ 'connection-editor--sections': layout === 'sections' }"
    @focusout="onFocusout"
  >
    <template v-if="layout === 'stacked'">
      <header class="connection-editor__status">
        <BaseStatus
          v-if="summary && connection?.ok"
          :label="t('plugins.connection.connectedTo', { url })"
          :tone="tone === 'ok' ? 'success' : 'neutral'"
        />
        <DiagnosticStatusAction
          v-else-if="tone === 'error'"
          :label="statusText"
          source="plugins"
          :entity-id="pluginId"
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

      <p
        v-if="guided && serverInfo && !dirty"
        class="connection-editor__note"
        role="status"
      >
        {{ t(`plugins.connection.model.${serverInfo.modelStatus}`) }}
        <span v-if="!serverInfo.authRequired"> · {{ t('plugins.connection.noTokenNeeded') }}</span>
      </p>
      <p
        v-if="guided && serverInfo && !serverInfo.playback && !dirty"
        class="connection-editor__note"
      >
        {{ t('plugins.connection.noPlayback') }}
      </p>
      <p
        v-if="connection?.gameplayReady !== undefined && !dirty"
        class="connection-editor__note"
      >
        {{ connection.gameplayReady ? t('plugins.connection.gameplayReady') : t('connections.servers.apiNotReady') }}
      </p>
      <p
        v-if="connection?.ok && connection.details && !dirty"
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
          v-if="connection && !connection.ok && !dirty && !urlInvalid && connection.error && !needsAuthorization && guided && !connection.reachable && !advanced"
          class="connection-editor__details"
          role="status"
        >
          {{ t('plugins.connection.openGuided', { name: pluginName }) }}
        </p>
        <BaseSchemaForm
          v-if="settings && !(supportsTokenImport && working)"
          v-model="formValues"
          switches
          :schema="visibleSchema"
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
            :disabled="!settings || testing || urlInvalid || (guided && (dirty || saveState === 'saving'))"
            :title="guided ? undefined : localHint"
            @click="guided ? connect() : test()"
          >
            {{ testing ? t('plugins.connection.testing') : t(guided ? 'plugins.connection.connectGuided' : 'plugins.connection.test', { name: pluginName }) }}
          </BaseButton>
        </div>
        <BaseButton
          v-if="guided"
          variant="ghost"
          :aria-expanded="advanced"
          @click="advanced = !advanced"
        >
          {{ t('plugins.connection.advanced') }}
        </BaseButton>
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
              :title="t('plugins.connection.autoConnectHint')"
              @click="startImport({ mode: 'auto' })"
            >
              <template #icon>
                <IconSearch />
              </template>
              {{ working ? t('plugins.connection.importing') : t('plugins.connection.autoConnect') }}
            </BaseButton>
            <BaseButton
              :disabled="importDisabled"
              :title="t('plugins.connection.autoConnectHint')"
              @click="startImport({ mode: 'folder' })"
            >
              <template #icon>
                <IconFolder />
              </template>
              {{ t('plugins.connection.selectFolder') }}
            </BaseButton>
          </template>
          <BaseButton
            :disabled="importDisabled"
            :title="t('plugins.connection.importTokenHint')"
            @click="startImport()"
          >
            <template #icon>
              <IconDownload />
            </template>
            {{ working ? t('plugins.connection.importing') : t('plugins.connection.importToken') }}
          </BaseButton>
        </div>
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
        v-if="supportsProvisioning && (!guided || needsAuthorization || advanced)"
        class="connection-editor__actions"
      >
        <BaseButton
          :disabled="importDisabled"
          @click="guided ? connect() : provisionOpen = true"
        >
          {{ t(guided ? 'plugins.connection.authorize' : 'plugins.connection.getToken') }}
        </BaseButton>
        <ProvisionTokenDialog
          v-model:open="provisionOpen"
          :plugin-name="pluginName"
          :simple="guided"
          :working="working"
          :result="provisionResult"
          @submit="(username, password) => emit('provision', pluginId, username, password)"
        />
      </div>
    </template>

    <template v-else>
      <div class="connection-editor__grid">
        <BaseSection :title="t('plugins.connection.sections.connection')">
          <p
            v-if="guided && serverInfo && !dirty"
            class="connection-editor__note"
            role="status"
          >
            {{ t(`plugins.connection.model.${serverInfo.modelStatus}`) }}
            <span v-if="!serverInfo.authRequired"> · {{ t('plugins.connection.noTokenNeeded') }}</span>
          </p>
          <p
            v-if="guided && serverInfo && !serverInfo.playback && !dirty"
            class="connection-editor__note"
          >
            {{ t('plugins.connection.noPlayback') }}
          </p>
          <p
            v-if="connection?.gameplayReady !== undefined && !dirty"
            class="connection-editor__note"
          >
            {{ connection.gameplayReady ? t('plugins.connection.gameplayReady') : t('connections.servers.apiNotReady') }}
          </p>
          <BaseSchemaForm
            v-if="settings"
            v-model="formValues"
            compact
            switches
            :schema="connectionSchema"
            :ui-hints="settings.uiHints"
            :field-options="fieldOptions"
            :field-errors="fieldErrors"
            :field-icons="fieldIcons"
            :open-media-picker="openMediaPicker"
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
              :disabled="!settings || testing || urlInvalid || dirty || saveState === 'saving'"
              @click="connect()"
            >
              {{ testing ? t('plugins.connection.testing') : t('plugins.connection.connectGuided', { name: pluginName }) }}
            </BaseButton>
          </div>
          <BaseButton
            v-if="guided"
            variant="ghost"
            :aria-expanded="advanced"
            @click="advanced = !advanced"
          >
            {{ t('plugins.connection.advanced') }}
          </BaseButton>

          <div
            v-if="supportsTokenImport"
            class="connection-editor__import"
          >
            <template v-if="supportsTokenDiscovery">
              <BaseButton
                variant="primary"
                :disabled="importDisabled"
                :title="t('plugins.connection.autoConnectHint')"
                @click="startImport({ mode: 'auto' })"
              >
                <template #icon>
                  <IconGear />
                </template>
                {{ working ? t('plugins.connection.importing') : t('plugins.connection.actions.auto') }}
              </BaseButton>
              <BaseButton
                :disabled="importDisabled"
                :title="t('plugins.connection.autoConnectHint')"
                @click="startImport({ mode: 'folder' })"
              >
                <template #icon>
                  <IconFolder />
                </template>
                {{ t('plugins.connection.actions.folder') }}
              </BaseButton>
            </template>
            <BaseButton
              :disabled="importDisabled"
              :title="t('plugins.connection.importTokenHint')"
              @click="startImport()"
            >
              <template #icon>
                <IconFile />
              </template>
              {{ working ? t('plugins.connection.importing') : t('plugins.connection.actions.import') }}
            </BaseButton>
          </div>

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

          <div
            v-if="supportsProvisioning && (!guided || needsAuthorization || advanced)"
            class="connection-editor__actions"
          >
            <BaseButton
              :disabled="importDisabled"
              @click="guided ? connect() : provisionOpen = true"
            >
              {{ t(guided ? 'plugins.connection.authorize' : 'plugins.connection.getToken') }}
            </BaseButton>
            <ProvisionTokenDialog
              v-model:open="provisionOpen"
              :plugin-name="pluginName"
              :simple="guided"
              :working="working"
              :result="provisionResult"
              @submit="(username, password) => emit('provision', pluginId, username, password)"
            />
          </div>
        </BaseSection>

        <BaseSection
          v-if="hasDefaults"
          :title="t('plugins.connection.sections.defaults')"
        >
          <BaseSchemaForm
            v-if="settings"
            v-model="formValues"
            compact
            switches
            :schema="defaultsSchema"
            :ui-hints="settings.uiHints"
            :field-options="fieldOptions"
            :field-errors="fieldErrors"
            :field-icons="fieldIcons"
            :open-media-picker="openMediaPicker"
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
  </div>
</template>

<style scoped>
.connection-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.connection-editor--sections {
  gap: var(--space-3);
}

.connection-editor__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 340px), 1fr));
  gap: var(--space-4);
  align-items: start;
}

.connection-editor__status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  font-weight: var(--font-weight-semibold);
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

.connection-editor__failure {
  margin: 0;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
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

.connection-editor__import {
  display: flex;
  gap: var(--space-2);
}

.connection-editor__import :deep(.base-button) {
  flex: 1 1 0;
  min-width: 0;
  padding: 0 var(--space-3);
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
