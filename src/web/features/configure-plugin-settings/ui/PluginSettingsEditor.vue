<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../../shared/messages.ts';
import { toSettingValues } from '../../../../shared/settings-values.ts';
import { stableSettingsJson } from '@/entities/plugin/lib/connection-settings.ts';
import { fieldOptionsFor, optionSources } from '@/entities/plugin/lib/plugin-options.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import { withSchemaDefaults } from '@/shared/ui/schema-form/lib/schema-fields.ts';
import type { PluginSettingsState } from '@/types.ts';

const props = withDefaults(
  defineProps<{
    pluginId: string;
    settings?: PluginSettingsState;
    actionOptions: Readonly<Record<string, ActionOptionItem[]>>;
    openMediaPicker?: OpenMediaPicker;
    /** Id of the plugin's `*.test_connection` action; null hides the check button. */
    testActionId?: string | null;
    testing?: boolean;
    testResult?: { ok: boolean; text: string } | null;
  }>(),
  { settings: undefined, openMediaPicker: undefined, testActionId: null, testing: false, testResult: null },
);

const emit = defineEmits<{
  loadSettings: [pluginId: string];
  loadOptions: [source: string];
  save: [pluginId: string, values: PluginSettingValues];
  test: [pluginId: string];
}>();

const { t } = useI18n();
const draft = ref<JsonObject | null>(null);
const savedAt = ref(0);

const stored = computed(() => withSchemaDefaults(props.settings?.values ?? {}, props.settings?.schema));
const values = computed<JsonObject>({
  get: () => draft.value ?? stored.value,
  set: (next) => {
    draft.value = next;
  },
});
const dirty = computed(
  () => draft.value !== null && stableSettingsJson(toSettingValues(draft.value)) !== stableSettingsJson(toSettingValues(stored.value)),
);
const fieldOptions = computed(() => fieldOptionsFor(props.settings?.uiHints, props.actionOptions));

function requestOptions(): void {
  for (const source of optionSources(props.settings?.uiHints)) emit('loadOptions', source);
}

function save(): void {
  if (!draft.value) return;
  emit('save', props.pluginId, toSettingValues(draft.value));
  savedAt.value = Date.now();
}

watch(
  () => props.settings?.values,
  () => {
    if (draft.value && !dirty.value) draft.value = null;
  },
);
watch(() => props.settings?.uiHints, requestOptions);
watch(
  () => props.pluginId,
  () => {
    draft.value = null;
    if (!props.settings) emit('loadSettings', props.pluginId);
  },
);

onMounted(() => {
  if (!props.settings) emit('loadSettings', props.pluginId);
  requestOptions();
});
</script>

<template>
  <div class="plugin-settings">
    <p
      v-if="!settings"
      class="plugin-settings__loading"
    >
      {{ t('plugins.settings.loading') }}
    </p>
    <template v-else>
      <BaseSchemaForm
        v-model="values"
        :schema="settings.schema"
        :ui-hints="settings.uiHints"
        :field-options="fieldOptions"
        :open-media-picker="openMediaPicker"
      />
      <div class="plugin-settings__actions">
        <BaseButton
          v-if="testActionId"
          :disabled="dirty || testing"
          :title="dirty ? t('plugins.settings.saveFirst') : ''"
          @click="emit('test', pluginId)"
        >
          {{ testing ? t('plugins.settings.testing') : t('plugins.settings.test') }}
        </BaseButton>
        <span
          v-if="testResult"
          class="plugin-settings__test-result"
          :class="testResult.ok ? 'plugin-settings__test-result--ok' : 'plugin-settings__test-result--error'"
          role="status"
        >{{ testResult.text }}</span>
        <span
          v-if="savedAt && !dirty"
          class="plugin-settings__saved"
          role="status"
        >{{ t('plugins.settings.saved') }}</span>
        <span class="plugin-settings__spacer" />
        <BaseButton
          :disabled="!dirty"
          @click="draft = null"
        >
          {{ t('plugins.settings.discard') }}
        </BaseButton>
        <BaseButton
          variant="primary"
          :disabled="!dirty"
          @click="save"
        >
          {{ t('plugins.settings.save') }}
        </BaseButton>
      </div>
    </template>
  </div>
</template>

<style scoped>
.plugin-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.plugin-settings__loading {
  margin: 0;
  color: var(--color-text-subtle);
}

.plugin-settings__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding-top: var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-settings__spacer {
  flex: 1;
}

.plugin-settings__saved {
  color: var(--color-success);
  font-size: var(--font-size-sm);
}

.plugin-settings__test-result {
  overflow: hidden;
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-settings__test-result--ok {
  color: var(--color-success);
}

.plugin-settings__test-result--error {
  color: var(--color-danger);
}
</style>
