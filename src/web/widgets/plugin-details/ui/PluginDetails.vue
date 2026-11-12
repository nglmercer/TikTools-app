<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, LiveAction, PluginStatus } from '../../../../automation/behavior/types.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { pluginActionUsage } from '@/entities/plugin/lib/plugin-catalog.ts';
import { findTestConnectionAction } from '@/entities/plugin/lib/plugin-test-action.ts';
import PluginSettingsEditor from '@/features/configure-plugin-settings/ui/PluginSettingsEditor.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconMinus from '@/shared/ui/icon/icons/IconMinus.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import type { PluginDetailsRequest, PluginDetailsTab } from '../model/details-tab.ts';
import PluginGeneralPane from './PluginGeneralPane.vue';
import PluginConnectionPane from './PluginConnectionPane.vue';
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
const removable = computed(() => descriptor.value.source === 'user');
const hasSettings = computed(() => props.plugin.installed && descriptor.value.hasSettings === true);
const hasConnection = computed(
  () => props.plugin.installed && props.plugin.enabled && descriptor.value.hasConnectionProbe === true,
);
const usage = computed(() => pluginActionUsage(id.value, props.actions, props.actionTypes));
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
const connection = computed(() => plugins.connections.value[id.value]);
const connecting = ref(false);
watch([() => connection.value?.at, id], () => { connecting.value = false; });

function testConnection(): void {
  connecting.value = true;
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
          <PluginGeneralPane
            v-if="value === 'general'"
            :descriptor="descriptor"
            :action-types="actionTypes"
          />
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
          <PluginConnectionPane
            v-else
            :descriptor="descriptor"
            :name="name"
            :connection="connection"
            :connecting="connecting"
            :plugins="plugins"
            @test="testConnection"
          />
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

<style src="./plugin-details.css"></style>
