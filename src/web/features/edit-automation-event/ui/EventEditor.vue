<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import type { BehaviorRun, LiveAction, LiveEvent, PluginEventType } from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import { cloneRecord } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import TestRunList from '@/entities/automation/ui/TestRunList.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconFlask from '@/shared/ui/icon/icons/IconFlask.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
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
  test: [event: LiveEvent];
  fire: [event: LiveEvent];
}>();

const dirty = defineModel<boolean>('dirty', { default: false });

const { t, triggers } = useAutomationText();
const id = useId();
const draft = ref<LiveEvent>(cloneRecord(props.event));
const deleteOpen = ref(false);
const changed = computed(() => props.isNew || JSON.stringify(draft.value) !== JSON.stringify(props.baseline ?? props.event));
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
