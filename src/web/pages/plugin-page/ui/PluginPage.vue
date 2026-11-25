<script setup lang="ts">
import { computed, ref, watch, type Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute } from 'vue-router';
import type { PluginPageDescriptor, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import { adaptLegacyPage, collectOptionSources, type PluginUiDescriptor, type PluginUiPage } from '../../../../plugin-ui/index.ts';
import { optionFields } from '../../../../automation/plugins/declarative.ts';
import { storeToRefs } from 'pinia';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import {
  closePluginUi,
  controlBackend,
  isDesktopHost,
  openPluginUi,
  pluginUiUrl,
  reloadPluginUi,
} from '@/entities/plugin/api/plugin-webview-host.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { childNodes, formNodes } from '@/entities/plugin/lib/plugin-ui-runtime.ts';
import type { PluginUiContext } from '@/entities/plugin/model/plugin-ui-context.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconPopout from '@/shared/ui/icon/icons/IconPopout.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import PluginInlineFrame from '@/widgets/plugin-page/ui/PluginInlineFrame.vue';
import PluginUiNode from '@/widgets/plugin-page/ui/PluginUiNode.vue';

const { t } = useI18n();
const route = useRoute();
const control = useControlClient();
const plugins = usePluginControls();
const pluginsStore = usePluginsStore();
const { actionOptions, actionOptionErrors, actionOptionSelected } = storeToRefs(pluginsStore);
const { pluginPages, pluginUis } = storeToRefs(useAutomationStore());
const localeStore = useLocaleStore();

const local = ref<Record<string, string | number | boolean>>({});
const drafts = ref<Record<string, JsonObject>>({});
const windowOpen = ref(false);

const pages = pluginPages as unknown as Readonly<Ref<PluginPageDescriptor[]>>;
const uis = pluginUis as unknown as Readonly<Ref<PluginUiDescriptor[]>>;

const routePluginId = computed(() =>
  typeof route.params.pluginId === 'string' ? route.params.pluginId : '',
);
const routePageId = computed(() =>
  typeof route.params.pageId === 'string' ? route.params.pageId : '',
);
const legacyPage = computed(() =>
  pages.value.find((entry) => entry.pluginId === routePluginId.value && entry.id === routePageId.value),
);
const pluginId = routePluginId;
const ui = computed(() => uis.value.find((entry) => entry.pluginId === pluginId.value));
const typedPage = computed(() => ui.value?.pages.find((entry) => entry.id === routePageId.value));
const page = computed(() => {
  if (legacyPage.value) return legacyPage.value;
  if (!typedPage.value) return undefined;
  return {
    id: typedPage.value.id,
    pluginId: pluginId.value,
    title: typedPage.value.title,
    icon: typedPage.value.icon,
  };
});
const plugin = computed<PluginStatus | undefined>(() =>
  plugins.behavior.value.plugins.find((entry) => entry.descriptor.id === pluginId.value),
);
const pluginName = computed(() => (plugin.value ? localizedText(plugin.value.descriptor.name) : pluginId.value));
const declarative = computed<PluginUiPage | null>(() => {
  if (ui.value?.mode === 'declarative' && typedPage.value?.body) {
    return {
      id: typedPage.value.id,
      pluginId: pluginId.value,
      title: typedPage.value.title,
      icon: typedPage.value.icon,
      body: typedPage.value.body,
    };
  }
  return legacyPage.value ? adaptLegacyPage(legacyPage.value) : null;
});
const webviewPage = computed(() => {
  if (!ui.value || ui.value.mode !== 'webview') return undefined;
  return typedPage.value;
});
const frameSrc = computed(() =>
  ui.value?.entry && webviewPage.value ? pluginUiUrl(pluginId.value, ui.value.entry, webviewPage.value.id) : null,
);
const backend = computed(() => {
  const id = pluginId.value;
  if (!id) return undefined;
  return {
    backend: controlBackend(id, control, () => localeStore.locale, () => 'dark'),
    subscribeTopic: (topic: string, listener: (data: unknown) => void) =>
      control.onTopic(topic, (data: unknown) => {
        if (data !== null && typeof data === 'object' && (data as Record<string, unknown>)['pluginId'] === id) listener(data);
      }),
  };
});
const title = computed(() => {
  if (webviewPage.value) return localizedText(webviewPage.value.title);
  return declarative.value ? localizedText(declarative.value.title) : '';
});
const desktop = isDesktopHost();

const context = computed<PluginUiContext>(() => {
  const id = pluginId.value;
  const descriptor = plugin.value?.descriptor;
  return {
    pluginId: id,
    pluginName: pluginName.value,
    settings: {
      state: plugins.settings.value[id],
      get: () => plugins.loadSettings(id),
      save: (values) => plugins.saveSettings(id, values),
    },
    options: {
      values: actionOptions.value,
      errors: actionOptionErrors.value,
      selected: actionOptionSelected.value,
      get: (source, refresh, context) => pluginsStore.handleGetActionOptions(source, refresh, id, context),
    },
    actions: {
      execute: (actionType, config) => pluginsStore.executeAction(id, actionType, config, true),
    },
    connection: {
      state: plugins.connections.value[id],
      test: () => plugins.testConnection(id),
    },
    media: { pick: plugins.openMediaPicker },
    provisioning: {
      supportsImport: descriptor?.supportsTokenImport,
      supportsDiscovery: descriptor?.supportsTokenDiscovery,
      supported: descriptor?.supportsTokenProvisioning ?? false,
      state: plugins.provision.value[id],
      importToken: (options) => plugins.importToken(id, options),
      provision: (username, password) => plugins.provisionToken(id, username, password),
    },
    local: {
      get: (name) => local.value[name],
      set: (name, value) => {
        local.value = { ...local.value, [name]: value };
      },
    },
    customNodes: {},
    formDrafts: {
      get: (key) => drafts.value[key],
      set: (key, value) => {
        drafts.value = { ...drafts.value, [key]: value };
      },
      keys: () => Object.keys(drafts.value),
      clear: () => {
        drafts.value = {};
      },
    },
  };
});

function requestSources(): void {
  const body = declarative.value?.body;
  if (!body || webviewPage.value) return;
  const current = context.value;
  if (!current.settings.state) current.settings.get();
  for (const source of collectOptionSources(body)) current.options.get(source);
  for (const node of formNodes(body)) {
    if (node.type !== 'form') continue;
    for (const field of optionFields(node.uiHints ?? current.settings.state?.uiHints)) current.options.get(field.source);
  }
}

function openWindow(): void {
  if (page.value) windowOpen.value = openPluginUi(pluginId.value, page.value.id);
}

function closeWindow(): void {
  if (page.value) closePluginUi(pluginId.value, page.value.id);
  windowOpen.value = false;
}

function reloadWindow(): void {
  if (page.value) reloadPluginUi(pluginId.value, page.value.id);
}

watch(
  () => `${pluginId.value}:${page.value?.id ?? ''}`,
  () => {
    drafts.value = {};
    local.value = {};
    windowOpen.value = false;
    requestSources();
  },
  { immediate: true },
);
watch(() => context.value.settings.state?.uiHints, requestSources);
</script>

<template>
  <div class="plugin-page">
    <BasePanel
      v-if="page"
      class="plugin-page__panel"
      :title="title"
      :subtitle="pluginName"
      :flush="Boolean(frameSrc)"
    >
      <template
        v-if="webviewPage && (frameSrc || desktop)"
        #actions
      >
        <BaseButton
          v-if="desktop"
          size="sm"
          @click="windowOpen && !frameSrc ? closeWindow() : openWindow()"
        >
          <template #icon>
            <IconPopout />
          </template>
          {{ windowOpen && !frameSrc ? t('plugins.page.closeWindow') : t('plugins.page.openWindow') }}
        </BaseButton>
        <BaseButton
          v-if="desktop && windowOpen && !frameSrc"
          size="sm"
          @click="reloadWindow()"
        >
          <template #icon>
            <IconRefresh />
          </template>
          {{ t('plugins.page.reloadWindow') }}
        </BaseButton>
      </template>
      <PluginInlineFrame
        v-if="frameSrc && backend"
        :key="`${pluginId}:${frameSrc}`"
        :plugin-id="pluginId"
        :src="frameSrc"
        :backend="backend.backend"
        :subscribe-topic="backend.subscribeTopic"
      />
      <BaseEmptyState
        v-else-if="webviewPage"
        :title="desktop ? t('plugins.page.windowHint') : t('plugins.page.desktopOnly')"
      >
        <template #icon>
          <IconPuzzle :size="28" />
        </template>
      </BaseEmptyState>
      <div
        v-else-if="declarative"
        class="plugin-page__body"
      >
        <PluginUiNode
          v-for="(node, index) in childNodes(declarative.body)"
          :key="`${pluginId}:${page.id}:${index}`"
          :node="node"
          :context="context"
          :node-key="`s${index}`"
        />
      </div>
    </BasePanel>
    <BaseEmptyState
      v-else
      :title="t('plugins.page.missing')"
    >
      <template #icon>
        <IconPuzzle :size="28" />
      </template>
    </BaseEmptyState>
  </div>
</template>

<style scoped>
.plugin-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}

.plugin-page__panel {
  flex: 1;
}

.plugin-page__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  width: 100%;
  max-width: 920px;
}
</style>
