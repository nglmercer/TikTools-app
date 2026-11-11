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
import { cloneRecord, moveId, toggleId } from '@/entities/automation/lib/automation-records.ts';
import { rawHotkeyInputHint, requiresRawHotkeyInput } from '@/entities/automation/lib/hotkey-status.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import TestRunList from '@/entities/automation/ui/TestRunList.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import IconClock from '@/shared/ui/icon/icons/IconClock.vue';
import IconDice from '@/shared/ui/icon/icons/IconDice.vue';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import IconFlask from '@/shared/ui/icon/icons/IconFlask.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import ConditionsEditor from './ConditionsEditor.vue';
import ConditionTemplateDialog from './ConditionTemplateDialog.vue';

const props = withDefaults(
  defineProps<{
    event: LiveEvent;
    baseline?: LiveEvent;
    isNew?: boolean;
    busy?: boolean;
    saveBlocked?: boolean;
    actions: readonly LiveAction[];
    eventTypes: readonly PluginEventType[];
    gifts: readonly SearchOption[];
    viewers: readonly SearchOption[];
    testRuns: readonly BehaviorRun[];
    hotkeyStatus?: HotkeyStatusData | null;
    error?: string;
  }>(),
  { baseline: undefined, isNew: false, busy: false, saveBlocked: false, hotkeyStatus: null, error: '' },
);

const emit = defineEmits<{
  save: [event: LiveEvent];
  change: [event: LiveEvent];
  delete: [id: string];
  test: [event: LiveEvent];
  fire: [event: LiveEvent];
}>();

const dirty = defineModel<boolean>('dirty', { default: false });

registerModerationTemplates();
registerLiveGoalTemplates();

const { t, triggers } = useAutomationText();
const id = useId();
const draft = ref<LiveEvent>(cloneRecord(props.event));
const deleteOpen = ref(false);
const templatesOpen = ref(false);
const templatesAnchor = shallowRef<HTMLElement | null>(null);
const configuring = shallowRef<ConditionTemplate | null>(null);
const configureOpen = ref(false);

const changed = computed(() => props.isNew || JSON.stringify(draft.value) !== JSON.stringify(props.baseline ?? props.event));
const triggerOptions = computed(() =>
  triggers(props.eventTypes).map((option) => ({
    value: option.value,
    label: option.plugin ? `${option.label} (${option.plugin})` : option.label,
  })),
);
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
// Cooldown is edited in whole seconds; the record keeps milliseconds.
const cooldownSeconds = computed({
  get: () => Math.round(draft.value.cooldownMs / 1000),
  set: (seconds: number) => update({ cooldownMs: Math.max(0, seconds) * 1000 }),
});
const filters = computed({
  get: () => draft.value.filters,
  set: (next) => update({ filters: next }),
});

watch(
  () => props.event,
  (event) => {
    if (JSON.stringify(event) !== JSON.stringify(draft.value)) draft.value = cloneRecord(event);
  },
);
watch(draft, (event) => emit('change', cloneRecord(event)), { deep: true, flush: 'sync' });
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
  draft.value = cloneRecord(props.baseline ?? props.event);
}
</script>

<template>
  <BasePanel
    class="event-editor"
    :title="draft.name || t('automations.editor.newEventName')"
    :subtitle="changed ? t('automations.editor.unsaved') : undefined"
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
        :hint="t('automations.editor.testHint')"
        variant="default"
        size="sm"
        @click="emit('test', draft)"
      >
        <IconFlask />
      </BaseIconButton>
      <BaseIconButton
        :label="t('automations.editor.fire')"
        :hint="t('automations.editor.fireHint')"
        variant="default"
        size="sm"
        @click="emit('fire', draft)"
      >
        <IconZap />
      </BaseIconButton>
      <span
        class="event-editor__divider"
        aria-hidden="true"
      />
      <BaseIconButton
        :label="t('automations.editor.discard')"
        variant="default"
        size="sm"
        :disabled="!changed || isNew || busy"
        @click="discard"
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
    <div class="event-editor__body">
      <p
        v-if="error"
        class="event-editor__error"
        role="alert"
      >
        {{ error }}
      </p>
      <div class="event-editor__columns">
        <div class="event-editor__column">
          <BaseSection :title="t('automations.event.general')">
            <BaseTextField
              :id="`${id}-name`"
              :model-value="draft.name"
              :placeholder="t('automations.editor.name')"
              :aria-label="t('automations.editor.name')"
              @update:model-value="update({ name: $event })"
            />
            <BaseComboBox
              :id="`${id}-trigger`"
              :model-value="draft.trigger"
              :options="triggerOptions"
              :aria-label="t('automations.event.trigger')"
              @update:model-value="$event && update({ trigger: $event })"
            />
            <BaseSwitch
              :id="`${id}-enabled`"
              :model-value="draft.enabled"
              @update:model-value="update({ enabled: $event })"
            >
              {{ t('automations.editor.enabled') }}
            </BaseSwitch>
          </BaseSection>

          <BaseSection
            :title="t('automations.event.conditions')"
            :description="rawHotkey ? t('automations.event.rawHotkey') : undefined"
          >
            <template
              v-if="templates.length"
              #actions
            >
              <BaseIconButton
                :label="t('automations.conditions.templates')"
                size="sm"
                aria-haspopup="menu"
                @click="openTemplates"
              >
                <IconFile />
              </BaseIconButton>
            </template>
            <ConditionsEditor
              v-model="filters"
              :trigger="draft.trigger"
              :gifts="gifts"
              :viewers="viewers"
            />
          </BaseSection>
        </div>

        <div class="event-editor__column">
          <BaseSection :title="t('automations.event.run')">
            <template
              v-if="draft.actionIds.length > 1"
              #actions
            >
              <BaseIconButton
                :label="t('automations.event.random')"
                size="sm"
                :pressed="draft.runMode === 'random'"
                @click="update({ runMode: draft.runMode === 'random' ? 'all' : 'random' })"
              >
                <IconDice />
              </BaseIconButton>
            </template>
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
          </BaseSection>

          <BaseSection :title="t('automations.event.cooldown')">
            <BaseSpinBox
              :id="`${id}-cooldown`"
              v-model="cooldownSeconds"
              orientation="horizontal"
              :min="0"
              :max="86400"
              suffix="s"
              :aria-label="t('automations.event.cooldown')"
            >
              <template #prefix>
                <IconClock />
              </template>
            </BaseSpinBox>
            <div
              v-if="draft.cooldownMs > 0"
              class="event-editor__scope"
              role="group"
              :aria-label="t('automations.event.scope')"
            >
              <BaseIconButton
                :label="t('automations.event.scopeUser')"
                variant="default"
                size="sm"
                :pressed="draft.cooldownScope === 'user'"
                @click="update({ cooldownScope: 'user' })"
              >
                <IconUser />
              </BaseIconButton>
              <BaseIconButton
                :label="t('automations.event.scopeGlobal')"
                variant="default"
                size="sm"
                :pressed="draft.cooldownScope === 'global'"
                @click="update({ cooldownScope: 'global' })"
              >
                <IconGlobe />
              </BaseIconButton>
            </div>
          </BaseSection>
        </div>
      </div>

      <TestRunList :runs="testRuns" />
    </div>
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
      :busy="busy || saveBlocked"
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
  gap: var(--space-4);
  padding: var(--space-4);
}

/* Two columns that wrap on narrow widths: what triggers it | what it runs. */
.event-editor__columns {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: var(--space-4);
  align-items: start;
}

.event-editor__column {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}

.event-editor__divider {
  align-self: stretch;
  width: var(--border-width);
  margin: var(--space-1) var(--space-2);
  background: var(--color-border);
}

.event-editor__error {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.event-editor__muted {
  margin: 0;
  color: var(--color-text-subtle);
}

.event-editor__actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
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

.event-editor__scope {
  display: flex;
  gap: var(--space-2);
}
</style>
