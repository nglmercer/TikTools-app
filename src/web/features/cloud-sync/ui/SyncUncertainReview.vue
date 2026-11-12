<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SyncRecovery } from '../model/use-sync-recovery.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const props = defineProps<{ model: SyncRecovery }>();
const { t } = useI18n();
const { uncertain, review, recoveryBusy, recoveryError, newerNeedsReview, reviewUncertain, resolveUncertain, recoveryValueText } = props.model;
</script>

<template>
  <BaseAlert
    v-if="recoveryError"
    tone="warning"
    :title="recoveryError"
  />
  <section
    v-if="uncertain.length"
    :aria-label="t('sync.uncertainTitle')"
  >
    <h3>{{ t('sync.uncertainTitle') }}</h3>
    <p>{{ t('sync.uncertainDescription') }}</p>
    <ul>
      <li
        v-for="key in uncertain"
        :key="key"
      >
        {{ key }}
        <BaseButton
          :disabled="recoveryBusy"
          @click="reviewUncertain(key)"
        >
          {{ t('sync.review') }}
        </BaseButton>
      </li>
    </ul>
    <div v-if="review">
      <h4>{{ review.key }}</h4>
      <p>{{ t('sync.base') }}: {{ recoveryValueText(review.base) }}</p>
      <p>{{ t('sync.original') }}: {{ recoveryValueText(review.original) }}</p>
      <p>{{ t('sync.local') }}: {{ recoveryValueText(review.local) }}</p>
      <p>{{ t('sync.cloud') }}: {{ recoveryValueText(review.cloud) }}</p>
      <BaseAlert
        tone="info"
        :title="t('sync.recoveryUnavailable')"
      />
      <p v-if="newerNeedsReview">
        {{ t('sync.newerWarning') }}
      </p>
      <BaseButton
        disabled
        @click="resolveUncertain('keepLocal')"
      >
        {{ t('sync.keepMine') }}
      </BaseButton>
      <BaseButton
        disabled
        @click="resolveUncertain('takeCloud')"
      >
        {{ t('sync.useCloud') }}
      </BaseButton>
      <BaseButton
        v-if="newerNeedsReview"
        disabled
        @click="resolveUncertain('takeCloudIncludingNewer')"
      >
        {{ t('sync.discardNewer') }}
      </BaseButton>
    </div>
  </section>
</template>
