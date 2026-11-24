<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationEdit } from '../model/automation-editor.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseNotification from '@/shared/ui/notification/BaseNotification.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
const props = defineProps<{ edit: AutomationEdit | null; hostChanged: boolean; memory: boolean }>();
defineEmits<{ compare: [] }>();
const { t } = useI18n();
// An automatic rebase is informational and transient: show it as a toast
// instead of reserving a permanent row at the top of the editor.
const autoDismissed = ref(false);
watch(() => props.edit?.autoResolved, (resolved) => { if (resolved) autoDismissed.value = false; });
// The toast handles auto-resolve, so an otherwise quiet edit needs no bar at
// all. Collapsing it keeps the editor header directly under the sheet title.
const hasNotice = computed(() => {
  const edit = props.edit;
  return !!edit && (edit.busy || edit.reviewRequired || edit.revision === undefined || props.hostChanged || props.memory);
});
</script>
<template>
  <BaseNotification
    v-if="edit?.autoResolved && !autoDismissed"
    class="automation-edit-notice__toast"
    tone="info"
    :message="t('automations.conflict.autoResolved')"
    @dismiss="autoDismissed = true"
  />
  <div
    v-if="edit && hasNotice"
    class="automation-edit-notice"
  >
    <div class="automation-edit-notice__messages">
      <p
        v-if="edit.busy"
        role="status"
      >
        {{ t('automations.conflict.waiting') }}
      </p>
      <p
        v-else-if="edit.reviewRequired"
        role="alert"
      >
        {{ t(edit.failure === 'unconfirmed' ? 'automations.conflict.unconfirmed' : 'automations.conflict.changed') }}
        <span v-if="edit.message">{{ edit.message }}</span>
      </p>
      <p
        v-else-if="edit.revision === undefined"
        role="alert"
      >
        {{ t('automations.conflict.missingRevision') }}
      </p>
      <p
        v-else-if="hostChanged"
        role="status"
      >
        {{ t('automations.conflict.hostChanged') }}
      </p>
      <p
        v-if="memory"
        role="status"
      >
        {{ t('automations.conflict.memoryLead') }}
      </p>
    </div>
    <div class="automation-edit-notice__actions">
      <BaseIconButton
        class="automation-edit-notice__compare"
        :label="t('automations.conflict.compare')"
        :disabled="edit.busy"
        size="sm"
        @click="$emit('compare')"
      >
        <IconGitMerge />
      </BaseIconButton>
    </div>
  </div>
</template>
<style scoped>
.automation-edit-notice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--space-2);
  flex: none;
  padding: var(--space-3);
  overflow-wrap: anywhere;
  border-bottom: var(--border-width) solid var(--color-border);
}
.automation-edit-notice p { margin: 0 0 var(--space-2); }
.automation-edit-notice__toast {
  position: fixed;
  z-index: 110;
  top: 42px;
  right: var(--space-4);
  width: 440px;
  max-width: calc(100vw - 32px);
}
.automation-edit-notice__messages { min-width: 0; }
.automation-edit-notice__actions { display: flex; align-items: center; gap: var(--space-3); flex: none; }
.automation-edit-notice__compare { opacity: 0; transition: opacity var(--duration-fast); }
.automation-edit-notice:hover .automation-edit-notice__compare,
.automation-edit-notice:focus-within .automation-edit-notice__compare { opacity: 1; }
@media (prefers-reduced-motion: reduce) {
  .automation-edit-notice__compare { transition: none; }
}
@media (any-pointer: coarse), (width < 700px) {
  .automation-edit-notice__compare { opacity: 1; }
}
</style>
