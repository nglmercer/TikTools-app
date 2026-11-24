<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationEdit } from '../model/automation-editor.ts';
import BaseNotification from '@/shared/ui/notification/BaseNotification.vue';

const props = defineProps<{ edit: AutomationEdit | null; hostChanged: boolean; memory: boolean }>();
const { t } = useI18n();

type Tone = 'error' | 'warning' | 'info' | 'ok';
/** All edit statuses are transient: one floating notification instead of a
 * bar that pushes the editor down. The compare action lives in the header. */
const notice = computed<{ key: string; message: string; tone: Tone } | null>(() => {
  const edit = props.edit;
  if (!edit) return null;
  if (edit.busy) return { key: 'waiting', message: t('automations.conflict.waiting'), tone: 'info' };
  if (edit.reviewRequired) {
    const base = t(edit.failure === 'unconfirmed' ? 'automations.conflict.unconfirmed' : 'automations.conflict.changed');
    return { key: `review:${edit.failure ?? 'changed'}`, message: edit.message ? `${base} ${edit.message}` : base, tone: 'error' };
  }
  if (edit.revision === undefined) return { key: 'revision', message: t('automations.conflict.missingRevision'), tone: 'error' };
  if (props.hostChanged) return { key: 'host', message: t('automations.conflict.hostChanged'), tone: 'warning' };
  if (props.memory) return { key: 'memory', message: t('automations.conflict.memoryLead'), tone: 'info' };
  if (edit.autoResolved) return { key: 'auto', message: t('automations.conflict.autoResolved'), tone: 'ok' };
  return null;
});
const dismissed = ref(false);
watch(() => notice.value?.key, () => { dismissed.value = false; });
</script>

<template>
  <BaseNotification
    v-if="notice && !dismissed"
    class="automation-edit-notice__toast"
    :tone="notice.tone"
    :message="notice.message"
    @dismiss="dismissed = true"
  />
</template>

<style scoped>
.automation-edit-notice__toast {
  position: fixed;
  z-index: 110;
  top: 42px;
  right: var(--space-4);
  width: 440px;
  max-width: calc(100vw - 32px);
}
</style>
