<script setup lang="ts">
import { computed } from 'vue';
import type { LiveEvent, LiveAction } from '../../../../automation/behavior/types.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import { moveId, toggleId } from '@/entities/automation/lib/automation-records.ts';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import IconDice from '@/shared/ui/icon/icons/IconDice.vue';
const props = defineProps<{ event: LiveEvent; actions: readonly LiveAction[] }>();
const emit = defineEmits<{ change: [patch: Partial<Pick<LiveEvent, 'actionIds' | 'runMode'>>] }>();
const { t } = useAutomationText();
const orderedActions = computed(() => {
  const chosen = props.event.actionIds
    .map((actionId) => props.actions.find((action) => action.id === actionId))
    .filter((action): action is LiveAction => action !== undefined);
  return [...chosen, ...props.actions.filter((action) => !props.event.actionIds.includes(action.id))];
});
</script>

<template>
  <BaseSection :title="t('automations.event.run')">
    <template
      v-if="event.actionIds.length > 1"
      #actions
    >
      <BaseIconButton
        :label="t('automations.event.random')"
        size="sm"
        :pressed="event.runMode === 'random'"
        @click="emit('change', { runMode: event.runMode === 'random' ? 'all' : 'random' })"
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
          :model-value="event.actionIds.includes(action.id)"
          @update:model-value="emit('change', { actionIds: toggleId(event.actionIds, action.id) })"
        >
          {{ action.name }}
        </BaseCheckbox>
        <span class="event-editor__type">{{ action.typeId }}</span>
        <template v-if="event.actionIds.includes(action.id)">
          <BaseIconButton
            :label="t('automations.event.moveUp')"
            size="sm"
            :disabled="event.actionIds.indexOf(action.id) === 0"
            @click="emit('change', { actionIds: moveId(event.actionIds, action.id, -1) })"
          >
            <IconChevronUp />
          </BaseIconButton>
          <BaseIconButton
            :label="t('automations.event.moveDown')"
            size="sm"
            :disabled="event.actionIds.indexOf(action.id) === event.actionIds.length - 1"
            @click="emit('change', { actionIds: moveId(event.actionIds, action.id, 1) })"
          >
            <IconChevronDown />
          </BaseIconButton>
        </template>
      </li>
    </ul>
  </BaseSection>
</template>
