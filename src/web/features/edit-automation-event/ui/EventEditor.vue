<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, useId, watch } from 'vue';
import {
  applyConditionTemplate,
  conditionTemplatesFor,
  type ConditionTemplate,
} from '../../../../automation/behavior/condition-templates.ts';
import { registerLiveGoalTemplates } from '../../../../automation/behavior/live-goal-templates.ts';
import { registerModerationTemplates } from '../../../../automation/behavior/moderation-templates.ts';
import type { BehaviorRun, LiveAction, LiveEvent, PluginEventType } from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import { cloneRecord, COOLDOWN_CHOICES, moveId, toggleId } from '@/entities/automation/lib/automation-records.ts';
import { rawHotkeyInputHint, requiresRawHotkeyInput } from '@/entities/automation/lib/hotkey-status.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import TestRunList from '@/entities/automation/ui/TestRunList.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import ConditionsEditor from './ConditionsEditor.vue';
import ConditionTemplateDialog from './ConditionTemplateDialog.vue';

const props = withDefaults(
  defineProps<{
    event: LiveEvent;
    isNew?: boolean;
    actions: readonly LiveAction[];
    eventTypes: readonly PluginEventType[];
    gifts: readonly SearchOption[];
    viewers: readonly SearchOption[];
    testRuns: readonly BehaviorRun[];
    hotkeyStatus?: HotkeyStatusData | null;
    error?: string;
  }>(),
  { isNew: false, hotkeyStatus: null, error: '' },
);

const emit = defineEmits<{
  save: [event: LiveEvent];
  delete: [id: string];
  test: [event: LiveEvent];
  fire: [event: LiveEvent];
}>();

const dirty = defineModel<boolean>('dirty', { default: false });

registerModerationTemplates();
registerLiveGoalTemplates();

const { t, triggers, sentence } = useAutomationText();
const id = useId();
const draft = ref<LiveEvent>(cloneRecord(props.event));
const deleteOpen = ref(false);
const templatesOpen = ref(false);
const templatesAnchor = shallowRef<HTMLElement | null>(null);
const configuring = shallowRef<ConditionTemplate | null>(null);
const configureOpen = ref(false);

const changed = computed(() => props.isNew || JSON.stringify(draft.value) !== JSON.stringify(props.event));
const triggerOptions = computed(() =>
  triggers(props.eventTypes).map((option) => ({
    value: option.value,
    label: option.plugin ? `${option.label} (${option.plugin})` : option.label,
  })),
);
const summary = computed(() => sentence(draft.value, props.actions, props.eventTypes));
const templates = computed(() => conditionTemplatesFor(draft.value.trigger));
const templateMenu = computed<MenuEntry[]>(() =>
  templates.value.map((template) => ({ id: template.id, label: localizedText(template.title) })),
);
const rawHotkey = computed(
  () =>
    draft.value.trigger === 'hotkey.pressed' &&
    rawHotkeyInputHint(props.hotkeyStatus, requiresRawHotkeyInput(draft.value.filters)) !== null,
);
const orderedActions = computed(() => {
  const chosen = draft.value.actionIds
    .map((actionId) => props.actions.find((action) => action.id === actionId))
    .filter((action): action is LiveAction => action !== undefined);
  return [...chosen, ...props.actions.filter((action) => !draft.value.actionIds.includes(action.id))];
});
const cooldownOptions = computed(() =>
  COOLDOWN_CHOICES.map((ms) => ({
    value: ms,
    label: ms === 0 ? t('automations.event.cooldownNone') : t('automations.event.cooldownSeconds', { count: ms / 1000 }),
  })),
);
const scopeOptions = computed(() => [
  { value: 'user' as const, label: t('automations.event.scopeUser') },
  { value: 'global' as const, label: t('automations.event.scopeGlobal') },
]);
const filters = computed({
  get: () => draft.value.filters,
  set: (next) => update({ filters: next }),
});

watch(
  () => props.event,
  (event) => {
    draft.value = cloneRecord(event);
  },
);
watch(changed, (value) => (dirty.value = value), { immediate: true });

function update(patch: Partial<LiveEvent>): void {
  draft.value = { ...draft.value, ...patch };
}

function openTemplates(event: MouseEvent): void {
  templatesAnchor.value = markRaw(event.currentTarget as HTMLElement);
  templatesOpen.value = true;
}

function pickTemplate(action: MenuAction): void {
  const template = templates.value.find((entry) => entry.id === action.id);
  if (!template) return;
  if (template.params?.length) {
    configuring.value = template;
    configureOpen.value = true;
  } else {
    update({ filters: applyConditionTemplate(template, {}) });
  }
}

function applyConfigured(answers: Record<string, string>): void {
  if (configuring.value) update({ filters: applyConditionTemplate(configuring.value, answers) });
}

function confirmDelete(): void {
  deleteOpen.value = false;
  emit('delete', draft.value.id);
}

function discard(): void {
  draft.value = cloneRecord(props.event);
}
</script>

<template>
  <BasePanel
    class="event-editor"
    :title="draft.name || t('automations.editor.newEventName')"
    :subtitle="changed ? t('automations.editor.unsaved') : undefined"
    flush
  >
    <template #actions>
      <BaseButton
        size="sm"
        @click="emit('test', draft)"
      >
        {{ t('automations.editor.test') }}
      </BaseButton>
      <BaseButton
        size="sm"
        @click="emit('fire', draft)"
      >
        {{ t('automations.editor.fire') }}
      </BaseButton>
    </template>
    <div class="event-editor__body">
      <p
        v-if="error"
        class="event-editor__error"
        role="alert"
      >
        {{ error }}
      </p>
      <p class="event-editor__summary">
        {{ summary }}
      </p>
      <BaseGroupBox :title="t('automations.event.general')">
        <BaseFormGrid>
          <BaseFormRow
            :for="`${id}-name`"
            :label="t('automations.editor.name')"
          >
            <BaseTextField
              :id="`${id}-name`"
              :model-value="draft.name"
              @update:model-value="update({ name: $event })"
            />
          </BaseFormRow>
          <BaseFormRow
            :for="`${id}-trigger`"
            :label="t('automations.event.trigger')"
          >
            <BaseComboBox
              :id="`${id}-trigger`"
              :model-value="draft.trigger"
              :options="triggerOptions"
              @update:model-value="$event && update({ trigger: $event })"
            />
          </BaseFormRow>
          <BaseFormRow
            :for="`${id}-enabled`"
            :label="t('automations.editor.enabled')"
          >
            <BaseCheckbox
              :id="`${id}-enabled`"
              :model-value="draft.enabled"
              @update:model-value="update({ enabled: $event })"
            />
          </BaseFormRow>
        </BaseFormGrid>
      </BaseGroupBox>

      <BaseGroupBox
        :title="t('automations.event.conditions')"
        :description="t('automations.event.conditionsHint')"
      >
        <div
          v-if="templates.length"
          class="event-editor__tools"
        >
          <BaseButton
            size="sm"
            aria-haspopup="menu"
            @click="openTemplates"
          >
            {{ t('automations.conditions.templates') }}
          </BaseButton>
        </div>
        <p
          v-if="rawHotkey"
          class="event-editor__warning"
          role="note"
        >
          {{ t('automations.event.rawHotkey') }}
        </p>
        <ConditionsEditor
          v-model="filters"
          :trigger="draft.trigger"
          :gifts="gifts"
          :viewers="viewers"
        />
      </BaseGroupBox>

      <BaseGroupBox
        :title="t('automations.event.run')"
        :description="t('automations.event.runHint')"
      >
        <p
          v-if="!actions.length"
          class="event-editor__muted"
        >
          {{ t('automations.event.noActions') }}
        </p>
        <ul
          v-else
          class="event-editor__actions"
        >
          <li
            v-for="action in orderedActions"
            :key="action.id"
            class="event-editor__action"
          >
            <BaseCheckbox
              :model-value="draft.actionIds.includes(action.id)"
              @update:model-value="update({ actionIds: toggleId(draft.actionIds, action.id) })"
            >
              {{ action.name }}
            </BaseCheckbox>
            <span class="event-editor__type">{{ action.typeId }}</span>
            <template v-if="draft.actionIds.includes(action.id)">
              <BaseIconButton
                :label="t('automations.event.moveUp')"
                size="sm"
                :disabled="draft.actionIds.indexOf(action.id) === 0"
                @click="update({ actionIds: moveId(draft.actionIds, action.id, -1) })"
              >
                <IconChevronUp />
              </BaseIconButton>
              <BaseIconButton
                :label="t('automations.event.moveDown')"
                size="sm"
                :disabled="draft.actionIds.indexOf(action.id) === draft.actionIds.length - 1"
                @click="update({ actionIds: moveId(draft.actionIds, action.id, 1) })"
              >
                <IconChevronDown />
              </BaseIconButton>
            </template>
          </li>
        </ul>
        <BaseCheckbox
          v-if="draft.actionIds.length > 1"
          :model-value="draft.runMode === 'random'"
          @update:model-value="update({ runMode: $event ? 'random' : 'all' })"
        >
          {{ t('automations.event.random') }}
        </BaseCheckbox>
        <BaseFormGrid>
          <BaseFormRow
            :for="`${id}-cooldown`"
            :label="t('automations.event.cooldown')"
          >
            <BaseComboBox
              :id="`${id}-cooldown`"
              :model-value="draft.cooldownMs"
              :options="cooldownOptions"
              @update:model-value="$event !== undefined && update({ cooldownMs: $event })"
            />
          </BaseFormRow>
          <BaseFormRow
            v-if="draft.cooldownMs > 0"
            :for="`${id}-scope`"
            :label="t('automations.event.scope')"
            :hint="t('automations.event.scopeHint')"
          >
            <BaseSegmented
              :id="`${id}-scope`"
              :model-value="draft.cooldownScope"
              :options="scopeOptions"
              size="sm"
              @update:model-value="update({ cooldownScope: $event })"
            />
          </BaseFormRow>
        </BaseFormGrid>
      </BaseGroupBox>

      <TestRunList :runs="testRuns" />
    </div>
    <template #footer>
      <BaseButton
        v-if="!isNew"
        size="sm"
        variant="danger"
        @click="deleteOpen = true"
      >
        {{ t('automations.editor.delete') }}
      </BaseButton>
      <span class="event-editor__spacer" />
      <BaseButton
        size="sm"
        :disabled="!changed || isNew"
        @click="discard"
      >
        {{ t('automations.editor.discard') }}
      </BaseButton>
      <BaseButton
        size="sm"
        variant="primary"
        :disabled="!changed"
        @click="emit('save', draft)"
      >
        {{ t('automations.editor.save') }}
      </BaseButton>
    </template>
    <BaseMenu
      v-model:open="templatesOpen"
      :items="templateMenu"
      :anchor="templatesAnchor"
      @select="pickTemplate"
    />
    <ConditionTemplateDialog
      v-model:open="configureOpen"
      :template="configuring"
      @apply="applyConfigured"
    />
    <BaseConfirmDialog
      v-model:open="deleteOpen"
      :title="t('automations.editor.deleteEventTitle')"
      :message="t('automations.editor.deleteEventMessage', { name: draft.name })"
      :confirm-label="t('automations.editor.deleteConfirm')"
      danger
      @confirm="confirmDelete"
    />
  </BasePanel>
</template>

<style scoped>
.event-editor {
  height: 100%;
}

.event-editor__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  max-width: 820px;
  padding: var(--space-5);
}

.event-editor__error,
.event-editor__warning {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.event-editor__warning {
  border-color: var(--color-warning);
  color: var(--color-warning);
}

.event-editor__summary {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border-left: 3px solid var(--color-accent);
  background: var(--color-bg-field);
  color: var(--color-text);
}

.event-editor__tools {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--space-3);
}

.event-editor__muted {
  margin: 0;
  color: var(--color-text-subtle);
}

.event-editor__actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0 0 var(--space-4);
  padding: 0;
  list-style: none;
}

.event-editor__action {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--control-height-md);
}

.event-editor__action > :first-child {
  flex: 1;
  min-width: 0;
}

.event-editor__type {
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.event-editor__spacer {
  flex: 1;
}
</style>
