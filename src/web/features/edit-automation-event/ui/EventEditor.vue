<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, useId, watch } from 'vue';
import type { BehaviorRun, LiveAction, LiveEvent, PluginEventType } from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import { cloneRecord } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import TestRunList from '@/entities/automation/ui/TestRunList.vue';
import { blankEventForType } from '../../../../automation/event-registry.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCode from '@/shared/ui/icon/icons/IconCode.vue';
import IconFlask from '@/shared/ui/icon/icons/IconFlask.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseJsonEditor from '@/shared/ui/json/BaseJsonEditor.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

import EventConditionsSection from './EventConditionsSection.vue';
import EventRunSection from './EventRunSection.vue';
import EventCooldownSection from './EventCooldownSection.vue';

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
  test: [event: LiveEvent, customEvent?: unknown];
  fire: [event: LiveEvent, customEvent?: unknown];
}>();

const dirty = defineModel<boolean>('dirty', { default: false });

const { t, triggers } = useAutomationText();
const id = useId();
const draft = ref<LiveEvent>(cloneRecord(props.event));
const deleteOpen = ref(false);

// One grouped menu for the three ways to exercise a record: dry-run, fire
// with the host-resolved event, or fire with an edited custom envelope.
const runMenuOpen = ref(false);
const runMenuAnchor = shallowRef<HTMLElement | null>(null);
const runMenu = computed<MenuEntry[]>(() => [
  { id: 'test', label: t('automations.runMenu.test'), icon: IconFlask },
  { id: 'fire', label: t('automations.runMenu.fire') },
  { type: 'separator', id: 'separator' },
  { id: 'custom', label: t('automations.runMenu.custom'), icon: IconCode },
]);
const customOpen = ref(false);
const customText = ref('');
const customValue = shallowRef<Record<string, unknown> | undefined>(undefined);
const customError = ref('');

function onCustomValidation(result: { value?: Record<string, unknown>; error: string }): void {
  customValue.value = result.value;
  customError.value = result.error;
}
const changed = computed(() => JSON.stringify(draft.value) !== JSON.stringify(props.baseline ?? props.event));
// A new record may be saved with its defaults, but it is not "unsaved" until edited.
const canSave = computed(() => props.isNew || changed.value);
const triggerOptions = computed(() =>
  triggers(props.eventTypes).map((option) => ({
    value: option.value,
    label: option.plugin ? `${option.label} (${option.plugin})` : option.label,
  })),
);

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

function confirmDelete(): void {
  deleteOpen.value = false;
  emit('delete', draft.value.id);
}

function discard(): void {
  draft.value = cloneRecord(props.baseline ?? props.event);
}

function openRunMenu(event: MouseEvent): void {
  runMenuAnchor.value = markRaw(event.currentTarget as HTMLElement);
  runMenuOpen.value = true;
}

function pickRun(action: MenuAction): void {
  if (action.id === 'test') emit('test', cloneRecord(draft.value));
  else if (action.id === 'fire') emit('fire', cloneRecord(draft.value));
  else if (action.id === 'custom') {
    // Start from the contract shape without the sample-only sentinels.
    customText.value = JSON.stringify(blankEventForType(draft.value.trigger), null, 2);
    customValue.value = undefined;
    customError.value = '';
    customOpen.value = true;
  }
}

function useLastTestData(): void {
  const lastData = [...props.testRuns].reverse().find((run) => run.eventData !== undefined)?.eventData;
  if (lastData === undefined) return;
  // Run snapshots are the `data` object; the RPC expects a full envelope.
  customText.value = JSON.stringify({ type: draft.value.trigger, data: lastData }, null, 2);
}

function runCustom(kind: 'test' | 'fire'): void {
  if (!customValue.value || customError.value) return;
  customOpen.value = false;
  if (kind === 'test') emit('test', cloneRecord(draft.value), customValue.value);
  else emit('fire', cloneRecord(draft.value), customValue.value);
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
        :label="t('automations.runMenu.label')"
        :hint="t('automations.runMenu.hint')"
        variant="default"
        size="sm"
        aria-haspopup="menu"
        :aria-expanded="runMenuOpen"
        @click="openRunMenu"
      >
        <IconFlask />
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
        :disabled="!canSave || busy || saveBlocked"
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

          <EventConditionsSection
            :event="draft"
            :gifts="gifts"
            :viewers="viewers"
            :hotkey-status="hotkeyStatus"
            @change="update({ filters: $event })"
          />
        </div>

        <div class="event-editor__column">
          <EventRunSection
            :event="draft"
            :actions="actions"
            @change="update"
          />

          <EventCooldownSection
            :event="draft"
            :field-id="id"
            @change="update"
          />
        </div>
      </div>

      <TestRunList :runs="testRuns" />
    </div>
    <BaseMenu
      v-model:open="runMenuOpen"
      :items="runMenu"
      :anchor="runMenuAnchor"
      @select="pickRun"
    />
    <BaseDialog
      v-model:open="customOpen"
      :title="t('automations.runMenu.customTitle')"
      :width="720"
      mobile-fullscreen
    >
      <BaseJsonEditor
        v-model="customText"
        :label="t('automations.runMenu.customTitle')"
        :rows="14"
        @validation="onCustomValidation"
      />
      <BaseButton
        v-if="testRuns.some((run) => run.eventData !== undefined)"
        variant="ghost"
        size="sm"
        class="event-editor__custom-reuse"
        @click="useLastTestData"
      >
        {{ t('automations.runMenu.useLast') }}
      </BaseButton>
      <template #footer>
        <BaseButton
          variant="primary"
          :disabled="!customValue || Boolean(customError)"
          @click="runCustom('test')"
        >
          {{ t('automations.runMenu.test') }}
        </BaseButton>
        <BaseButton
          :disabled="!customValue || Boolean(customError)"
          @click="runCustom('fire')"
        >
          {{ t('automations.runMenu.fire') }}
        </BaseButton>
      </template>
    </BaseDialog>
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

<style src="./event-editor.css"></style>
