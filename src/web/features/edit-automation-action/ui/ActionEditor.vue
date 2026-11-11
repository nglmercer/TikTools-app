<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { deriveActionPermissions } from '../../../../automation/behavior/schema.ts';
import type { ActionTypeDefinition, BehaviorRun, LiveAction } from '../../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker } from '../../../../shared/messages.ts';
import { actionForm, variableScope } from '@/entities/automation/lib/action-form.ts';
import { actionTypeOf, cloneRecord } from '@/entities/automation/lib/automation-records.ts';
import { templateVariables } from '@/entities/automation/lib/template-variables.ts';
import TestRunList from '@/entities/automation/ui/TestRunList.vue';
import { fieldOptionsFor, optionSources } from '@/entities/plugin/lib/plugin-options.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconFlask from '@/shared/ui/icon/icons/IconFlask.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = withDefaults(
  defineProps<{
    action: LiveAction;
    baseline?: LiveAction;
    isNew?: boolean;
    busy?: boolean;
    saveBlocked?: boolean;
    actionTypes: readonly ActionTypeDefinition[];
    testRuns: readonly BehaviorRun[];
    globals: Readonly<Record<string, string>>;
    actionOptions: Readonly<Record<string, ActionOptionItem[]>>;
    actionOptionErrors: Readonly<Record<string, string>>;
    openMediaPicker?: OpenMediaPicker;
    error?: string;
  }>(),
  { baseline: undefined, isNew: false, busy: false, saveBlocked: false, openMediaPicker: undefined, error: '' },
);

const emit = defineEmits<{
  save: [action: LiveAction];
  change: [action: LiveAction];
  delete: [id: string];
  test: [action: LiveAction];
  loadOptions: [source: string, refresh: boolean];
}>();

const dirty = defineModel<boolean>('dirty', { default: false });

const { t, locale } = useI18n();
const id = useId();
const draft = ref<LiveAction>(cloneRecord(props.action));
const deleteOpen = ref(false);

const type = computed(() => actionTypeOf(draft.value, props.actionTypes));
const form = computed(() => (type.value ? actionForm(type.value) : null));
const sources = computed(() => optionSources(form.value?.uiHints));
const fieldOptions = computed(() => fieldOptionsFor(form.value?.uiHints, props.actionOptions));
const optionErrors = computed(() =>
  sources.value.map((source) => props.actionOptionErrors[source]).filter((message): message is string => Boolean(message)),
);
const permissions = computed(() => deriveActionPermissions(draft.value, type.value));
const runs = computed(() => props.testRuns.filter((run) => run.actionId === draft.value.id));
const changed = computed(() => props.isNew || JSON.stringify(draft.value) !== JSON.stringify(props.baseline ?? props.action));
const origin = computed(() => {
  const source = type.value?.source;
  if (!source) return draft.value.typeId;
  const from = source.kind === 'builtin' ? t('automations.action.builtin') : source.pluginId;
  return `${from} · ${type.value?.tag ?? ''}`;
});
const config = computed<JsonObject>({
  get: () => draft.value.config,
  set: (next) => {
    draft.value = { ...draft.value, config: next };
  },
});

function suggestionsFor(field: string) {
  return templateVariables({ scope: variableScope(draft.value.typeId, field), locale: locale.value, globals: props.globals });
}

function requestOptions(refresh: boolean): void {
  for (const source of sources.value) emit('loadOptions', source, refresh);
}

watch(
  () => props.action,
  (action) => {
    if (JSON.stringify(action) !== JSON.stringify(draft.value)) draft.value = cloneRecord(action);
  },
);
watch(draft, (action) => emit('change', cloneRecord(action)), { deep: true, flush: 'sync' });
watch(() => type.value?.id, () => requestOptions(false), { immediate: true });
watch(changed, (value) => (dirty.value = value), { immediate: true });

function confirmDelete(): void {
  deleteOpen.value = false;
  emit('delete', draft.value.id);
}
</script>

<template>
  <BasePanel
    class="action-editor"
    :title="draft.name || (type ? localizedText(type.title) : draft.typeId)"
    :subtitle="changed ? t('automations.editor.unsaved') : origin"
    flush
  >
    <template
      v-if="$slots.leading"
      #leading
    >
      <slot name="leading" />
    </template>
    <template #actions>
      <BaseIconButton
        :label="t('automations.editor.test')"
        variant="default"
        size="sm"
        @click="emit('test', draft)"
      >
        <IconFlask />
      </BaseIconButton>
      <span
        class="action-editor__divider"
        aria-hidden="true"
      />
      <BaseIconButton
        :label="t('automations.editor.discard')"
        variant="default"
        size="sm"
        :disabled="!changed || isNew || busy"
        @click="draft = cloneRecord(baseline ?? action)"
      >
        <IconUndo />
      </BaseIconButton>
      <BaseIconButton
        :label="t('automations.editor.save')"
        variant="primary"
        size="sm"
        :disabled="!changed || busy || saveBlocked"
        @click="emit('save', draft)"
      >
        <IconCheck />
      </BaseIconButton>
      <BaseIconButton
        v-if="!isNew"
        :label="t('automations.editor.delete')"
        variant="danger"
        size="sm"
        :disabled="busy || saveBlocked"
        @click="deleteOpen = true"
      >
        <IconTrash />
      </BaseIconButton>
    </template>
    <div class="action-editor__body">
      <BaseAlert
        v-if="error"
        :title="error"
      />
      <div class="action-editor__columns">
        <div class="action-editor__column">
          <BaseSection
            :title="t('automations.event.general')"
            :description="type ? localizedText(type.description) : undefined"
          >
            <BaseTextField
              :id="`${id}-name`"
              :model-value="draft.name"
              :placeholder="type ? localizedText(type.title) : t('automations.editor.name')"
              :aria-label="t('automations.editor.name')"
              @update:model-value="draft = { ...draft, name: $event }"
            />
            <BaseSwitch
              :id="`${id}-enabled`"
              :model-value="draft.enabled"
              @update:model-value="draft = { ...draft, enabled: $event }"
            >
              {{ t('automations.editor.enabled') }}
            </BaseSwitch>
          </BaseSection>

          <BaseSection :title="t('automations.action.permissions')">
            <dl class="action-editor__permissions">
              <dt :title="t('automations.action.networkHint')">
                {{ t('automations.action.network') }}
              </dt>
              <dd>
                <span
                  v-if="!permissions.network.length"
                  class="action-editor__muted"
                >{{ t('automations.action.none') }}</span>
                <span
                  v-for="host in permissions.network"
                  :key="host"
                  class="action-editor__chip"
                >{{ host }}</span>
              </dd>
              <dt :title="t('automations.action.capabilitiesHint')">
                {{ t('automations.action.capabilities') }}
              </dt>
              <dd>
                <span
                  v-if="!permissions.capabilities.length"
                  class="action-editor__muted"
                >{{ t('automations.action.none') }}</span>
                <span
                  v-for="capability in permissions.capabilities"
                  :key="capability"
                  class="action-editor__chip"
                >{{ capability }}</span>
              </dd>
            </dl>
          </BaseSection>
        </div>

        <div class="action-editor__column">
          <BaseSection :title="t('automations.action.settings')">
            <template
              v-if="form && sources.length"
              #actions
            >
              <BaseIconButton
                :label="t('automations.action.refreshOptions')"
                size="sm"
                @click="requestOptions(true)"
              >
                <IconRefresh />
              </BaseIconButton>
            </template>
            <BaseAlert
              v-if="!form"
              :title="t('automations.action.unknownType', { type: draft.typeId })"
            />
            <template v-else>
              <BaseAlert
                v-if="optionErrors.length"
                v-tooltip="optionErrors.join('\n')"
                :title="t('automations.action.optionsFailed', optionErrors.length)"
                compact
              />
              <BaseSchemaForm
                v-model="config"
                :schema="form.schema"
                :ui-hints="form.uiHints"
                :field-options="fieldOptions"
                :suggestions-for="suggestionsFor"
                :open-media-picker="openMediaPicker"
              />
            </template>
          </BaseSection>
        </div>
      </div>

      <TestRunList :runs="runs" />
    </div>
    <BaseConfirmDialog
      v-model:open="deleteOpen"
      :title="t('automations.editor.deleteActionTitle')"
      :message="t('automations.editor.deleteActionMessage', { name: draft.name })"
      :confirm-label="t('automations.editor.deleteConfirm')"
      :busy="busy || saveBlocked"
      danger
      @confirm="confirmDelete"
    />
  </BasePanel>
</template>

<style scoped>
.action-editor {
  height: 100%;
}

.action-editor__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
}

/* Identity and permissions on one side, the type's settings on the other; wraps on narrow widths. */
.action-editor__columns {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: var(--space-4);
  align-items: start;
}

.action-editor__column {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}

.action-editor__divider {
  align-self: stretch;
  width: var(--border-width);
  margin: var(--space-1) var(--space-2);
  background: var(--color-border);
}

.action-editor__permissions {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-3) var(--space-6);
  margin: 0;
}

.action-editor__permissions dt {
  color: var(--color-text-muted);
}

.action-editor__permissions dd {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
}

.action-editor__chip {
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 20px;
}

.action-editor__muted {
  color: var(--color-text-subtle);
}

.action-editor__spacer {
  flex: 1;
}
</style>
