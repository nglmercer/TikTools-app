<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseDisclosure from '@/shared/ui/disclosure/BaseDisclosure.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';

defineProps<{ edit: AutomationEdit | null; review: AutomationReview | null; busy: boolean; error: string; canAdopt: boolean }>();
const open = defineModel<boolean>('open', { default: false });
defineEmits<{ adopt: []; refresh: [] }>();
const { t } = useI18n();
const changesOpen = ref(false);
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="automation-review-dialog"
    :title="t('automations.conflict.compareTitle')"
    :width="1080"
  >
    <p>{{ t('automations.conflict.compareLead') }}</p>
    <p
      v-if="busy"
      role="status"
    >
      {{ t('automations.conflict.reading') }}
    </p>
    <p
      v-if="error"
      role="alert"
    >
      {{ error }}
    </p>
    <div
      v-if="edit && review"
      class="automation-review"
    >
      <section>
        <h3>{{ t('automations.conflict.base') }} · {{ edit.revision ?? t('automations.conflict.memory') }}</h3>
        <BaseJsonView
          v-if="edit.base"
          :value="edit.base"
          max-height="48vh"
        />
        <p v-else>
          {{ t('automations.conflict.absent') }}
        </p>
      </section>
      <section>
        <h3>{{ t('automations.conflict.candidate') }}</h3>
        <BaseJsonView
          :value="edit.candidate"
          max-height="48vh"
        />
      </section>
      <section>
        <h3>{{ t('automations.conflict.current') }} · {{ review.current.revision ?? t('automations.conflict.memory') }}</h3>
        <BaseJsonView
          v-if="review.current.record"
          :value="review.current.record"
          max-height="48vh"
        />
        <p v-else>
          {{ t('automations.conflict.deleted') }}
        </p>
      </section>
    </div>
    <BaseDisclosure
      v-if="edit && review"
      v-model:open="changesOpen"
      :title="t('jsonView.showChanges')"
    >
      <div
        v-if="changesOpen"
        class="automation-review automation-review--diffs"
      >
        <section>
          <h3>{{ t('jsonView.draftChanges') }}</h3>
          <BaseJsonDiff
            :before="edit.base ?? undefined"
            :after="edit.candidate"
            max-height="40vh"
          />
        </section>
        <section>
          <h3>{{ t('jsonView.hostChanges') }}</h3>
          <BaseJsonDiff
            :before="edit.base ?? undefined"
            :after="review.current.record ?? undefined"
            max-height="40vh"
          />
        </section>
      </div>
    </BaseDisclosure>
    <p v-if="review && !review.current.record">
      {{ t('automations.conflict.recreateLead') }}
    </p>
    <p v-else-if="review">
      {{ t('automations.conflict.mergeLead') }}
    </p>
    <template #footer>
      <BaseButton
        :disabled="busy"
        @click="$emit('refresh')"
      >
        {{ t('automations.conflict.readCurrent') }}
      </BaseButton>
      <BaseButton
        :disabled="!canAdopt || busy"
        @click="$emit('adopt')"
      >
        {{ t(review && !review.current.record ? 'automations.conflict.prepareRecreate' : 'automations.conflict.useBase') }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.automation-review {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-4);
}
.automation-review section { min-width: 0; }
.automation-review h3 { font-size: var(--font-size-md); overflow-wrap: anywhere; }
.automation-review--diffs { grid-template-columns: repeat(2, minmax(0, 1fr)); }
@media (width < 800px) {
  .automation-review { grid-template-columns: 1fr; }
  .automation-review--diffs { grid-template-columns: 1fr; }
  .automation-review :deep(.base-json__scroll) { max-height: 28vh; }
  .automation-review-dialog :deep(.base-dialog__footer) { flex-direction: column; align-items: stretch; }
  .automation-review-dialog :deep(.base-dialog__spacer) { display: none; }
  .automation-review-dialog :deep(.base-dialog__footer .base-button) {
    height: auto;
    min-height: var(--control-height-md);
    padding: var(--space-2) var(--space-3);
    white-space: normal;
  }
}
</style>
