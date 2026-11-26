<script setup lang="ts">
import { computed } from 'vue';
import type { BehaviorSnapshot, LiveEvent, LiveAction } from '../../../../automation/behavior/types.ts';
import type { AutomationEdit } from '@/features/edit-automation-record/model/automation-editor.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { eventCaptures } from '@/entities/automation/lib/conditions.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import ActionEditor from '@/features/edit-automation-action/ui/ActionEditor.vue';
import EventEditor from '@/features/edit-automation-event/ui/EventEditor.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import type { EditingRecord } from '../model/editing-record.ts';

const props = defineProps<{
  editing: EditingRecord;
  state: AutomationEdit | null;
  saveBlocked: boolean;
  snapshot: BehaviorSnapshot;
  actions: readonly LiveAction[];
  eventTypes: NonNullable<BehaviorSnapshot['eventTypes']>;
  gifts: readonly SearchOption[];
  viewers: readonly SearchOption[];
  automation: Pick<AutomationControls, 'testRuns' | 'hotkeyStatus' | 'error' | 'testEvent' | 'fireEvent' | 'globals' | 'actionOptions' | 'actionOptionErrors' | 'openMediaPicker' | 'testAction' | 'loadOptions'>;
}>();
const emit = defineEmits<{ saveEvent: [event: LiveEvent]; saveAction: [action: LiveAction]; change: [record: LiveEvent | LiveAction]; remove: [kind: 'events' | 'actions', id: string]; compare: []; close: [] }>();
const { t } = useAutomationText();
const eventBaseline = computed(() => props.state?.kind === 'event' ? props.state.base as LiveEvent | null : null);
const actionBaseline = computed(() => props.state?.kind === 'action' ? props.state.base as LiveAction | null : null);
/** Captures exposed by the events that reference the action being edited. */
const actionCaptures = computed(() => {
  if (props.editing.kind !== 'action') return [];
  const id = props.editing.record.id;
  const paths = new Set<string>();
  for (const event of props.snapshot.events) {
    if (event.actionIds.includes(id)) {
      for (const path of eventCaptures(event)) paths.add(path);
    }
  }
  return [...paths];
});
</script>

<template>
  <EventEditor
    v-if="editing.kind === 'event'"
    data-guide="automation-editor"
    :key="state?.token"
    :baseline="eventBaseline ?? undefined"
    :busy="state?.busy"
    :save-blocked="saveBlocked"
    :event="editing.record"
    :is-new="editing.isNew"
    :actions="actions"
    :event-types="eventTypes"
    :gifts="gifts"
    :viewers="viewers"
    :test-runs="automation.testRuns.value"
    :hotkey-status="automation.hotkeyStatus.value"
    :error="automation.error.value"
    @save="emit('saveEvent', $event)"
    @compare="emit('compare')"
    @change="emit('change', $event)"
    @delete="emit('remove', 'events', $event)"
    @test="automation.testEvent"
    @fire="automation.fireEvent"
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        variant="default"
        size="sm"
        data-guide="automation-editor-close"
        @click="emit('close')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
  </EventEditor>
  <ActionEditor
    v-else
    data-guide="automation-editor"
    :key="state?.token"
    :baseline="actionBaseline ?? undefined"
    :busy="state?.busy"
    :save-blocked="saveBlocked"
    :action="editing.record"
    :is-new="editing.isNew"
    :action-types="snapshot.actionTypes"
    :test-runs="automation.testRuns.value"
    :globals="automation.globals.value"
    :action-options="automation.actionOptions.value"
    :action-option-errors="automation.actionOptionErrors.value"
    :open-media-picker="automation.openMediaPicker"
    :captures="actionCaptures"
    :error="automation.error.value"
    @save="emit('saveAction', $event)"
    @compare="emit('compare')"
    @change="emit('change', $event)"
    @delete="emit('remove', 'actions', $event)"
    @test="automation.testAction"
    @load-options="automation.loadOptions"
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        variant="default"
        size="sm"
        data-guide="automation-editor-close"
        @click="emit('close')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
  </ActionEditor>
</template>
