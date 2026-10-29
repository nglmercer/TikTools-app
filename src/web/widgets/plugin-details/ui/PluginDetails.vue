<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { pluginActionUsage, pluginState } from '@/entities/plugin/lib/plugin-catalog.ts';
import { findTestConnectionAction } from '@/entities/plugin/lib/plugin-test-action.ts';
import PluginStateBadge from '@/entities/plugin/ui/PluginStateBadge.vue';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import PluginSettingsEditor from '@/features/configure-plugin-settings/ui/PluginSettingsEditor.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import type { PluginDetailsTab } from '../model/details-tab.ts';

const props = defineProps<{
  plugin: PluginStatus;
  actions: readonly LiveAction[];
  actionTypes: readonly ActionTypeDefinition[];
}>();

const tab = defineModel<PluginDetailsTab>('tab', { required: true });

const { t } = useI18n();
const plugins = usePluginControls();
const uninstallOpen = ref(false);

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

const tabs = computed(() => [
  { value: 'general' as const, label: t('plugins.details.tabs.general') },
  { value: 'settings' as const, label: t('plugins.details.tabs.settings'), disabled: !hasSettings.value },
  { value: 'connection' as const, label: t('plugins.details.tabs.connection'), disabled: !hasConnection.value },
]);

const enabled = computed({
  get: () => props.plugin.enabled,
  set: (value: boolean) => plugins.setEnabled(id.value, value),
});

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
  } finally {
    testing.value = false;
  }
}

watch(
  [hasSettings, hasConnection],
  () => {
    if ((tab.value === 'settings' && !hasSettings.value) || (tab.value === 'connection' && !hasConnection.value)) {
      tab.value = 'general';
    }
  },
  { immediate: true },
);

function uninstall(): void {
  uninstallOpen.value = false;
  plugins.uninstall(id.value);
}
</script>

<template>
  <section
    class="plugin-details"
    :aria-label="name"
  >
    <header class="plugin-details__header">
      <IconPuzzle
        class="plugin-details__icon"
        :size="28"
      />
      <div class="plugin-details__heading">
        <h2 class="plugin-details__name">
          {{ name }}
        </h2>
        <span class="plugin-details__meta">
          <span class="plugin-details__version">{{ descriptor.version }}</span>
          <PluginStateBadge :state="pluginState(plugin)" />
        </span>
      </div>
      <div class="plugin-details__controls">
        <BaseButton
          v-if="!plugin.installed"
          variant="primary"
          :disabled="!plugin.available"
          @click="plugins.setInstalled(id, true)"
        >
          {{ removable ? t('plugins.details.activate') : t('plugins.details.install') }}
        </BaseButton>
        <BaseCheckbox
          v-else
          v-model="enabled"
        >
          {{ t('plugins.details.enabled') }}
        </BaseCheckbox>
        <BaseButton
          v-if="plugin.installed && removable"
          variant="danger"
          @click="uninstallOpen = true"
        >
          <template #icon>
            <IconTrash />
          </template>
          {{ t('plugins.details.uninstall') }}
        </BaseButton>
      </div>
    </header>

    <p
      v-if="!plugin.available && plugin.unavailableReason"
      class="plugin-details__warning"
      role="alert"
    >
      {{ t('plugins.details.unavailableReason', { reason: plugin.unavailableReason }) }}
    </p>

    <BaseTabs
      v-model="tab"
      class="plugin-details__tabs"
      :tabs="tabs"
    >
      <template #default="{ value }">
        <div class="plugin-details__pane">
          <div
            v-if="value === 'general'"
            class="plugin-details__general"
          >
            <BaseMarkdown
              v-if="description"
              :source="description"
            />
            <dl class="plugin-details__facts">
              <dt>{{ t('plugins.details.version') }}</dt>
              <dd class="plugin-details__mono">
                {{ descriptor.version }}
              </dd>
              <dt>{{ t('plugins.details.source') }}</dt>
              <dd>{{ t(`plugins.details.sources.${descriptor.source ?? 'builtin'}`) }}</dd>
              <dt>{{ t('plugins.details.dependency') }}</dt>
              <dd>{{ localizedText(descriptor.dependency) || t('plugins.details.none') }}</dd>
              <dt>{{ t('plugins.details.permissions') }}</dt>
              <dd>
                <span
                  v-if="!descriptor.permissions.length"
                  class="plugin-details__none"
                >{{ t('plugins.details.none') }}</span>
                <span
                  v-for="permission in descriptor.permissions"
                  :key="permission"
                  class="plugin-details__chip plugin-details__mono"
                >{{ permission }}</span>
              </dd>
              <dt>{{ t('plugins.details.actions') }}</dt>
              <dd>
                <span
                  v-if="!actionNames.length"
                  class="plugin-details__none"
                >{{ t('plugins.details.none') }}</span>
                <span
                  v-for="action in actionNames"
                  :key="action"
                  class="plugin-details__chip"
                >{{ action }}</span>
              </dd>
              <dt>{{ t('plugins.details.events') }}</dt>
              <dd>
                <span
                  v-if="!descriptor.eventTypeIds.length"
                  class="plugin-details__none"
                >{{ t('plugins.details.none') }}</span>
                <span
                  v-for="event in descriptor.eventTypeIds"
                  :key="event"
                  class="plugin-details__chip plugin-details__mono"
                >{{ event }}</span>
              </dd>
            </dl>
          </div>
          <PluginSettingsEditor
            v-else-if="value === 'settings'"
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
          <PluginConnectionEditor
            v-else
            :plugin-id="id"
            :plugin-name="name"
            :settings="plugins.settings.value[id]"
            :connection="plugins.connections.value[id]"
            :action-options="plugins.actionOptions.value"
            :open-media-picker="plugins.openMediaPicker"
            :supports-token-import="descriptor.supportsTokenImport"
            :supports-token-discovery="descriptor.supportsTokenDiscovery"
            :supports-provisioning="descriptor.supportsTokenProvisioning"
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
    </BaseTabs>

    <BaseConfirmDialog
      v-model:open="uninstallOpen"
      :title="t('plugins.details.uninstallTitle', { name })"
      :message="t('plugins.details.uninstallMessage', usage)"
      :confirm-label="t('plugins.details.uninstallConfirm')"
      danger
      @confirm="uninstall"
    />
  </section>
</template>

<style scoped>
.plugin-details {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  color: var(--color-text);
}

.plugin-details__header {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
}

.plugin-details__icon {
  flex: none;
  color: var(--color-accent);
}

.plugin-details__heading {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.plugin-details__name {
  margin: 0;
  overflow: hidden;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-details__meta {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.plugin-details__version,
.plugin-details__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-details__version {
  color: var(--color-text-subtle);
}

.plugin-details__controls {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
}

.plugin-details__warning {
  margin: var(--space-4) var(--space-5) 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-warning);
  border-radius: var(--radius-sm);
  color: var(--color-warning);
}

.plugin-details__tabs {
  flex: 1;
  min-height: 0;
}

.plugin-details__pane {
  box-sizing: border-box;
  height: 100%;
  padding: var(--space-5);
  overflow: auto;
}

.plugin-details__pane > * {
  max-width: 760px;
}

.plugin-details__general {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.plugin-details__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-3) var(--space-6);
  margin: 0;
}

.plugin-details__facts dt {
  color: var(--color-text-muted);
}

.plugin-details__facts dd {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
}

.plugin-details__chip {
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  line-height: 20px;
}

.plugin-details__none {
  color: var(--color-text-subtle);
}

@media (width < 900px) {
  .plugin-details__header {
    flex-wrap: wrap;
    row-gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
  }

  .plugin-details__pane {
    padding: var(--space-4);
  }

  .plugin-details__facts {
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-1);
  }
}
</style>
