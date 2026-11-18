<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import type { EditingRecord } from '../model/editing-record.ts';
defineProps<{
  editing: EditingRecord;
  automation: Pick<AutomationControls, 'persistent' | 'sourceHost'>;
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useAutomationText();
</script>

<template>
  <header class="automations-page__sheet-bar">
    <BaseIconButton
      :label="t('common.back')"
      variant="default"
      size="sm"
      @click="emit('close')"
    >
      <IconChevronLeft />
    </BaseIconButton>
    <span class="automations-page__sheet-title">{{ editing.record.name }}</span>
  </header>
</template>
