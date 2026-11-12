<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, toRef, watch } from 'vue';
import type { PluginSettingValues } from '../../../../shared/messages.ts';
import type { TokenSetupOptions } from '../../../../shared/token-import.ts';
import { useConnectionAutosave } from '../model/use-connection-autosave.ts';
import { optionSources } from '@/entities/plugin/lib/plugin-options.ts';
import type { PluginConnectionEditorProps } from '../model/connection-editor-types.ts';
import { useConnectionPresentation } from '../model/use-connection-presentation.ts';
import ConnectionStackedPanel from './ConnectionStackedPanel.vue';
import ConnectionSectionsPanel from './ConnectionSectionsPanel.vue';
import PluginCredentialsBlock from './PluginCredentialsBlock.vue';
import ProvisionTokenDialog from './ProvisionTokenDialog.vue';

const props = withDefaults(defineProps<PluginConnectionEditorProps>(), {
  settings: undefined,
  connection: undefined,
  guided: undefined,
  provisionState: undefined,
  openMediaPicker: undefined,
  layout: 'stacked',
});

const emit = defineEmits<{
  loadSettings: [pluginId: string];
  loadOptions: [source: string];
  save: [pluginId: string, values: PluginSettingValues];
  test: [pluginId: string];
  importToken: [pluginId: string, options?: TokenSetupOptions];
  provision: [pluginId: string, username: string, password: string];
}>();

const provisionOpen = ref(false);
let modelRefresh: ReturnType<typeof setTimeout> | undefined;
onBeforeUnmount(() => clearTimeout(modelRefresh));
const advanced = ref(false);
const connectRequested = ref(false);
const guided = computed(() => props.guided ?? props.connection?.serverInfo !== undefined);
const serverInfo = computed(() => props.connection?.serverInfo);
const needsAuthorization = computed(() => serverInfo.value?.authRequired && !serverInfo.value.authorized);
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
const { testing, saveState, dirty } = autosave;
const presentation = useConnectionPresentation(props, autosave, { guided, advanced, needsAuthorization });

const working = computed(() => props.provisionState?.working === true);
const importDisabled = computed(
  () => working.value || testing.value || dirty.value || saveState.value === 'saving' || !props.settings,
);
const provisionResult = computed(() =>
  props.provisionState && !props.provisionState.working && props.provisionState.message
    ? { ok: props.provisionState.ok, message: props.provisionState.message }
    : undefined,
);

/** Authorization is offered when the service needs a token, or on request in the advanced view. */
const authorizable = computed(() => props.supportsProvisioning === true && (!guided.value || needsAuthorization.value === true || advanced.value));
const showCredentials = computed(() => props.supportsTokenImport === true || authorizable.value);
/** The credentials block only renders these; every action it reports is handled here. */
const credentialsProps = computed(() => ({
  supportsTokenImport: props.supportsTokenImport,
  supportsTokenDiscovery: props.supportsTokenDiscovery,
  authorizable: authorizable.value,
  guided: guided.value,
  working: working.value,
  disabled: importDisabled.value,
  state: props.provisionState,
}));

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

function authorize(): void {
  if (guided.value) connect();
  else provisionOpen.value = true;
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
    <component
      :is="layout === 'stacked' ? ConnectionStackedPanel : ConnectionSectionsPanel"
      :source="props"
      :view="presentation"
      :state="autosave"
      :guided="guided"
      :advanced="advanced"
      v-bind="layout === 'stacked' ? { needsAuthorization, working } : {}"
      @edit="autosave.edit"
      @test="test"
      @connect="connect"
      @edit-settings="autosave.editing.value = true"
      @toggle-advanced="advanced = !advanced"
    >
      <template #credentials>
        <PluginCredentialsBlock
          v-if="showCredentials"
          v-bind="credentialsProps"
          :variant="layout"
          @import="startImport"
          @authorize="authorize"
        />
      </template>
    </component>
    <ProvisionTokenDialog
      v-if="supportsProvisioning"
      v-model:open="provisionOpen"
      :plugin-name="pluginName"
      :simple="guided"
      :working="working"
      :result="provisionResult"
      @submit="(username, password) => emit('provision', pluginId, username, password)"
    />
  </div>
</template>

<style src="./connection-editor.css"></style>
