<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';

/**
 * The header buttons every automation record editor (event, action) shares: compare with the host copy, then
 * whatever the editor adds in the default slot (test / run), a divider, and discard, save and delete.
 */
defineProps<{
  busy: boolean;
  changed: boolean;
  isNew: boolean;
  canSave: boolean;
  saveBlocked: boolean;
}>();
const emit = defineEmits<{ compare: []; discard: []; save: []; delete: [] }>();
const { t } = useAutomationText();
</script>

<template>
  <BaseIconButton
    :label="t('automations.conflict.compare')"
    variant="default"
    size="sm"
    :disabled="busy"
    @click="emit('compare')"
  >
    <IconGitMerge />
  </BaseIconButton>
  <slot />
  <span
    class="record-editor-actions__divider"
    aria-hidden="true"
  />
  <BaseIconButton
    :label="t('automations.editor.discard')"
    variant="default"
    size="sm"
    :disabled="!changed || isNew || busy"
    @click="emit('discard')"
  >
    <IconUndo />
  </BaseIconButton>
  <BaseIconButton
    :label="t('automations.editor.save')"
    variant="primary"
    size="sm"
    :disabled="!canSave || busy || saveBlocked"
    @click="emit('save')"
  >
    <IconCheck />
  </BaseIconButton>
  <BaseIconButton
    v-if="!isNew"
    :label="t('automations.editor.delete')"
    variant="danger"
    size="sm"
    :disabled="busy || saveBlocked"
    @click="emit('delete')"
  >
    <IconTrash />
  </BaseIconButton>
</template>

<style scoped>
.record-editor-actions__divider {
  align-self: stretch;
  width: var(--border-width);
  margin: var(--space-1) var(--space-2);
  background: var(--color-border);
}
</style>
