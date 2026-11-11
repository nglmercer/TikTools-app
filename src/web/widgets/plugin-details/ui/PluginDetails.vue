<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { connectionStatusText } from '@/entities/plugin/lib/connection-status.ts';
import { pluginActionUsage } from '@/entities/plugin/lib/plugin-catalog.ts';
import { findTestConnectionAction } from '@/entities/plugin/lib/plugin-test-action.ts';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import PluginSettingsEditor from '@/features/configure-plugin-settings/ui/PluginSettingsEditor.vue';
import DiagnosticStatusAction from '@/shared/diagnostics/DiagnosticStatusAction.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconLock from '@/shared/ui/icon/icons/IconLock.vue';
import IconMinus from '@/shared/ui/icon/icons/IconMinus.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import type { PluginDetailsRequest, PluginDetailsTab } from '../model/details-tab.ts';
import PluginVersionsPane from './PluginVersionsPane.vue';

const props = defineProps<{
  plugin: PluginStatus;
  actions: readonly LiveAction[];
  actionTypes: readonly ActionTypeDefinition[];
}>();

const emit = defineEmits<{ back: [] }>();
const tab = defineModel<PluginDetailsTab>('tab', { required: true });
/** One-shot command from the plugin list's context menu; cleared once handled. */
const request = defineModel<PluginDetailsRequest | null>('request', { default: null });

const { t } = useI18n();
const plugins = usePluginControls();
const uninstallOpen = ref(false);
const purgeData = ref(false);

const descriptor = computed(() => props.plugin.descriptor);
const id = computed(() => descriptor.value.id);
const name = computed(() => localizedText(descriptor.value.name));
const description = computed(() => {
  const long = descriptor.value.longDescription ? localizedText(descriptor.value.longDescription) : '';
  return long || localizedText(descriptor.value.description);
});
const removable = computed(() => descriptor.value.source === 'user');
const hasSettings = computed(() => props.plugin.installed && descriptor.value.hasSettings === true);
const hasConnection = computed(
  () => props.plugin.installed && props.plugin.enabled && descriptor.value.hasConnectionProbe === true,
);
const usage = computed(() => pluginActionUsage(id.value, props.actions, props.actionTypes));
const actionNames = computed(() =>
  descriptor.value.actionTypeIds.map((typeId) => {
    const type = props.actionTypes.find((entry) => entry.id === typeId);
    return type ? localizedText(type.title) : typeId;
  }),
);

// A tab the plugin has no use for is left out rather than shown disabled.
const tabs = computed(() => [
  { value: 'general' as const, label: t('plugins.details.tabs.general'), icon: IconInfo },
  ...(hasSettings.value ? [{ value: 'settings' as const, label: t('plugins.details.tabs.settings'), icon: IconGear }] : []),
  ...(hasConnection.value ? [{ value: 'connection' as const, label: t('plugins.details.tabs.connection'), icon: IconLink }] : []),
  ...(removable.value ? [{ value: 'versions' as const, label: t('plugins.details.tabs.versions'), icon: IconHistory }] : []),
]);

const enabled = computed({
  get: () => props.plugin.enabled,
  set: (value: boolean) => plugins.setEnabled(id.value, value),
});

/** Server plugins check their connection from the Connection tab, saving pending edits first. */
const editor = useTemplateRef<{ flush?: () => void }>('editor');
const connection = computed(() => plugins.connections.value[id.value]);
const connecting = ref(false);
watch([() => connection.value?.at, id], () => { connecting.value = false; });

function testConnection(): void {
  connecting.value = true;
  editor.value?.flush?.();
  plugins.testConnection(id.value);
}

/** One-click self-check: the plugin's `*.test_connection` action run live, no automation needed. */
const testActionId = computed(() => findTestConnectionAction(props.actionTypes, id.value));
const testing = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);

async function runTestConnection(): Promise<void> {
  if (!testActionId.value || testing.value) return;
  testing.value = true;
  testResult.value = null;
  try {
    const outcome = await plugins.executeAction(id.value, testActionId.value, {}, true);
    testResult.value = {
      ok: outcome.ok,
      text: outcome.ok ? outcome.summary : (outcome.error ?? outcome.summary),
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    testResult.value = { ok: false, text: message };
  } finally {
    testing.value = false;
  }
}

// A completed check belongs to the selected plugin and the saved settings.
// Never display a previous plugin's connection result after switching.
watch(id, () => { testResult.value = null; });
watch(() => plugins.settings.value[id.value]?.values, () => { testResult.value = null; });

watch(
  [hasSettings, hasConnection],
  () => {
    if (
      (tab.value === 'settings' && !hasSettings.value) ||
      (tab.value === 'connection' && !hasConnection.value) ||
      (tab.value === 'versions' && !removable.value)
    ) {
      tab.value = 'general';
    }
  },
  { immediate: true },
);

watch(
  request,
  (next) => {
    if (!next) return;
    request.value = null;
    if (next === 'uninstall' && props.plugin.installed && removable.value) uninstallOpen.value = true;
    if (next === 'test' && hasSettings.value && testActionId.value) {
      tab.value = 'settings';
      void runTestConnection();
    }
  },
  { immediate: true },
);

function uninstall(): void {
  uninstallOpen.value = false;
  plugins.uninstall(id.value, purgeData.value);
  purgeData.value = false;
}
</script>

<template>
  <BasePanel
    class="plugin-details"
    :title="name"
    flush
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        size="sm"
        @click="emit('back')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
    <template #status>
      <span
        v-tooltip="t('plugins.details.version')"
        class="plugin-details__chip plugin-details__mono"
      >{{ descriptor.version }}</span>
    </template>
    <template #actions>
      <BaseButton
        v-if="!plugin.installed"
        variant="primary"
        :disabled="!plugin.available"
        @click="plugins.setInstalled(id, true)"
      >
        {{ removable ? t('plugins.details.activate') : t('plugins.details.install') }}
      </BaseButton>
      <template v-else>
        <BaseSwitch
          v-model="enabled"
          :aria-label="t('plugins.details.enabled')"
        />
        <span
          class="plugin-details__divider"
          aria-hidden="true"
        />
        <BaseIconButton
          v-if="removable"
          :label="t('plugins.details.uninstall')"
          variant="danger"
          size="sm"
          @click="uninstallOpen = true"
        >
          <IconTrash />
        </BaseIconButton>
        <!-- Built-in and development plugins own no package files: they can only
             leave the installed list (reversible from Explore). -->
        <BaseIconButton
          v-else
          :label="t('plugins.menu.remove')"
          :hint="t('plugins.menu.removeHint')"
          size="sm"
          @click="plugins.setInstalled(id, false)"
        >
          <IconMinus />
        </BaseIconButton>
      </template>
    </template>

    <div class="plugin-details__content">
      <p
        v-if="!plugin.available && plugin.unavailableReason"
        class="plugin-details__warning"
        role="alert"
      >
        <IconWarning />
        {{ t('plugins.details.unavailableReason', { reason: plugin.unavailableReason }) }}
      </p>

      <BaseTabs
        v-model="tab"
        class="plugin-details__tabs"
        :tabs="tabs"
      >
        <template #default="{ value }">
          <div
            v-if="value === 'general'"
            class="plugin-details__general"
          >
            <BaseMarkdown
              v-if="description"
              class="plugin-details__description"
              :source="description"
            />
            <div class="plugin-details__columns">
              <div class="plugin-details__column">
                <BaseSection :title="t('plugins.details.source')">
                  <div
                    v-tooltip="t('plugins.details.source')"
                    class="plugin-details__field"
                  >
                    <IconLayers />
                    <span class="plugin-details__value">{{ t(`plugins.details.sources.${descriptor.source ?? 'builtin'}`) }}</span>
                  </div>
                  <div
                    v-tooltip="t('plugins.details.dependency')"
                    class="plugin-details__field"
                  >
                    <IconPuzzle />
                    <span class="plugin-details__value">{{ localizedText(descriptor.dependency) || t('plugins.details.none') }}</span>
                  </div>
                </BaseSection>
                <BaseSection :title="t('plugins.details.permissions')">
                  <div class="plugin-details__chips">
                    <IconLock class="plugin-details__lead" />
                    <span
                      v-if="!descriptor.permissions.length"
                      class="plugin-details__none"
                    >{{ t('plugins.details.none') }}</span>
                    <span
                      v-for="permission in descriptor.permissions"
                      :key="permission"
                      class="plugin-details__chip plugin-details__mono"
                    >{{ permission }}</span>
                  </div>
                </BaseSection>
              </div>
              <div class="plugin-details__column">
                <BaseSection :title="t('plugins.details.actions')">
                  <span
                    v-if="!actionNames.length"
                    class="plugin-details__none"
                  >{{ t('plugins.details.none') }}</span>
                  <div
                    v-for="action in actionNames"
                    :key="action"
                    class="plugin-details__field"
                  >
                    <span class="plugin-details__value">{{ action }}</span>
                  </div>
                </BaseSection>
                <BaseSection :title="t('plugins.details.events')">
                  <span
                    v-if="!descriptor.eventTypeIds.length"
                    class="plugin-details__none"
                  >{{ t('plugins.details.none') }}</span>
                  <div
                    v-for="event in descriptor.eventTypeIds"
                    :key="event"
                    class="plugin-details__field"
                  >
                    <span class="plugin-details__value plugin-details__mono">{{ event }}</span>
                  </div>
                </BaseSection>
              </div>
            </div>
          </div>
          <div
            v-else-if="value === 'settings'"
            class="plugin-details__pane"
          >
            <PluginSettingsEditor
              :plugin-id="id"
              :settings="plugins.settings.value[id]"
              :action-options="plugins.actionOptions.value"
              :open-media-picker="plugins.openMediaPicker"
              :test-action-id="testActionId"
              :testing="testing"
              :test-result="testResult"
              @load-settings="plugins.loadSettings"
              @load-options="plugins.loadOptions"
              @save="plugins.saveSettings"
              @test="runTestConnection"
            />
          </div>
          <div
            v-else-if="value === 'versions'"
            class="plugin-details__pane"
          >
            <PluginVersionsPane :plugin-id="id" />
          </div>
          <template v-else>
            <div class="plugin-details__strip">
              <DiagnosticStatusAction
                v-if="connection && !connection.ok"
                :label="connectionStatusText(connection, t)"
                source="plugins"
                :entity-id="id"
              />
              <BaseStatus
                v-else
                :label="connectionStatusText(connection, t)"
                :tone="connection?.ok ? 'success' : 'neutral'"
              />
              <BaseButton
                v-if="!descriptor.guidedConnection"
                class="plugin-details__test"
                variant="primary"
                :disabled="connecting"
                @click="testConnection"
              >
                <template #icon>
                  <IconZap />
                </template>
                {{ connecting ? t('plugins.connection.testing') : t('connections.servers.test') }}
              </BaseButton>
            </div>
            <div class="plugin-details__pane">
              <PluginConnectionEditor
                ref="editor"
                :key="id"
                layout="sections"
                :plugin-id="id"
                :plugin-name="name"
                :settings="plugins.settings.value[id]"
                :connection="connection"
                :action-options="plugins.actionOptions.value"
                :open-media-picker="plugins.openMediaPicker"
                :supports-token-import="descriptor.supportsTokenImport"
                :supports-token-discovery="descriptor.supportsTokenDiscovery"
                :supports-provisioning="descriptor.supportsTokenProvisioning"
                :guided="descriptor.guidedConnection"
                :provision-state="plugins.provision.value[id]"
                @load-settings="plugins.loadSettings"
                @load-options="plugins.loadOptions"
                @save="plugins.saveSettings"
                @test="plugins.testConnection"
                @import-token="plugins.importToken"
                @provision="plugins.provisionToken"
              />
            </div>
          </template>
        </template>
      </BaseTabs>
    </div>

    <BaseConfirmDialog
      v-model:open="uninstallOpen"
      :title="t('plugins.details.uninstallTitle', { name })"
      :message="t('plugins.details.uninstallMessage', usage)"
      :confirm-label="t('plugins.details.uninstallConfirm')"
      danger
      @confirm="uninstall"
    >
      <BaseCheckbox v-model="purgeData">
        {{ t('plugins.details.purgeData') }}
      </BaseCheckbox>
    </BaseConfirmDialog>
  </BasePanel>
</template>

<style scoped>
.plugin-details {
  flex: 1;
}

.plugin-details__content {
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

.plugin-details__divider {
  width: var(--border-width);
  height: var(--control-height-md);
  background: var(--color-border);
}

.plugin-details__tabs {
  flex: 1;
}

.plugin-details__tabs :deep(.base-tabs__bar) {
  padding: var(--space-2) var(--space-3) 0;
}

.plugin-details__warning {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border-bottom: var(--border-width) solid var(--color-warning);
  color: var(--color-warning);
}

.plugin-details__pane {
  box-sizing: border-box;
  padding: var(--space-4);
}

.plugin-details__pane > * {
  max-width: 960px;
}

.plugin-details__strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.plugin-details__general {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
}

.plugin-details__description {
  margin: 0;
}

.plugin-details__columns {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-4);
  align-items: start;
}

.plugin-details__column {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}

.plugin-details__field {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--control-height-md);
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  background: var(--color-bg-field);
  color: var(--color-text-muted);
}

.plugin-details__value {
  min-width: 0;
  overflow: hidden;
  color: var(--color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-details__chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.plugin-details__lead {
  color: var(--color-text-muted);
}

.plugin-details__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-details__chip {
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-text);
  line-height: 20px;
}

.plugin-details__none {
  color: var(--color-text-subtle);
}
</style>
