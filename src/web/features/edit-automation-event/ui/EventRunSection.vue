<script setup lang="ts">
import { computed, ref } from 'vue';
import type { LiveEvent, LiveAction } from '../../../../automation/behavior/types.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import { moveId, toggleId } from '@/entities/automation/lib/automation-records.ts';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import IconDice from '@/shared/ui/icon/icons/IconDice.vue';
import IconGrip from '@/shared/ui/icon/icons/IconGrip.vue';
const props = defineProps<{ event: LiveEvent; actions: readonly LiveAction[] }>();
const emit = defineEmits<{ change: [patch: Partial<Pick<LiveEvent, 'actionIds' | 'runMode'>>] }>();
const { t } = useAutomationText();
const list = ref<HTMLElement | null>(null);
const dragging = ref<string | null>(null);
const over = ref<string | null>(null);
const orderedActions = computed(() => {
  const chosen = props.event.actionIds
    .map((actionId) => props.actions.find((action) => action.id === actionId))
    .filter((action): action is LiveAction => action !== undefined);
  return [...chosen, ...props.actions.filter((action) => !props.event.actionIds.includes(action.id))];
});

/** Pointer-based reordering works with a mouse and with touch; the grip keeps
 * ArrowUp/ArrowDown available for keyboard users. */
function onPointerDown(event: PointerEvent, id: string): void {
  if (event.button !== 0 || props.event.actionIds.length < 2) return;
  event.preventDefault();
  const handle = event.currentTarget as HTMLElement;
  handle.setPointerCapture?.(event.pointerId);
  dragging.value = id;
  const rows = Array.from(list.value?.querySelectorAll<HTMLElement>('[data-action-id]') ?? []);
  const onMove = (move: PointerEvent): void => {
    const row = rows.find((candidate) => {
      const rect = candidate.getBoundingClientRect();
      return move.clientY >= rect.top && move.clientY <= rect.bottom;
    });
    const target = row?.dataset.actionId;
    if (!target || target === id || !props.event.actionIds.includes(target)) return;
    over.value = target;
    // Use the target's position in the full list: after removing the dragged
    // id, that index inserts after it when moving down and before it when
    // moving up. Using the index in the filtered list reverted downward drags.
    const next = props.event.actionIds.filter((entry) => entry !== id);
    next.splice(props.event.actionIds.indexOf(target), 0, id);
    if (next.every((entry, index) => entry === props.event.actionIds[index])) return;
    emit('change', { actionIds: next });
  };
  const onUp = (): void => {
    dragging.value = null;
    over.value = null;
    handle.removeEventListener('pointermove', onMove);
    handle.removeEventListener('pointerup', onUp);
    handle.removeEventListener('pointercancel', onUp);
  };
  handle.addEventListener('pointermove', onMove);
  handle.addEventListener('pointerup', onUp);
  handle.addEventListener('pointercancel', onUp);
}

function onKeydown(event: KeyboardEvent, id: string): void {
  const direction = event.key === 'ArrowUp' ? -1 : event.key === 'ArrowDown' ? 1 : 0;
  if (!direction || props.event.actionIds.indexOf(id) < 0) return;
  event.preventDefault();
  emit('change', { actionIds: moveId(props.event.actionIds, id, direction) });
}
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
      ref="list"
      class="event-editor__actions"
    >
      <li
        v-for="action in orderedActions"
        :key="action.id"
        class="event-editor__action"
        :class="{
          'event-editor__action--dragging': dragging === action.id,
          'event-editor__action--over': over === action.id && dragging !== action.id,
        }"
        :data-action-id="action.id"
      >
        <div class="event-editor__action-info">
          <BaseCheckbox
            :model-value="event.actionIds.includes(action.id)"
            @update:model-value="emit('change', { actionIds: toggleId(event.actionIds, action.id) })"
          >
            {{ action.name }}
          </BaseCheckbox>
          <span
            class="event-editor__type"
            :title="action.typeId"
          >{{ action.typeId }}</span>
        </div>
        <BaseIconButton
          v-if="event.actionIds.includes(action.id)"
          class="event-editor__grip"
          :label="t('automations.event.reorder')"
          :hint="t('automations.event.reorderHint')"
          size="sm"
          :disabled="event.actionIds.length < 2"
          @pointerdown.prevent="onPointerDown($event, action.id)"
          @keydown="onKeydown($event, action.id)"
        >
          <IconGrip />
        </BaseIconButton>
      </li>
    </ul>
  </BaseSection>
</template>
