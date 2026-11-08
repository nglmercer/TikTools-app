<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { AutomationEdit } from '../model/automation-editor.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
defineProps<{ edit: AutomationEdit | null; hostChanged: boolean; memory: boolean }>();
defineEmits<{ compare: [] }>();
const { t } = useI18n();
</script>
<template>
  <div
    v-if="edit"
    class="automation-edit-notice"
  >
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
    <BaseButton
      size="sm"
      :disabled="edit.busy"
      @click="$emit('compare')"
    >
      {{ t('automations.conflict.compare') }}
    </BaseButton>
  </div>
</template>
<style scoped>
.automation-edit-notice { flex: none; padding: var(--space-3); overflow-wrap: anywhere; border-bottom: var(--border-width) solid var(--color-border); }
.automation-edit-notice p { margin: 0 0 var(--space-2); }
</style>
