<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SyncV2Conflicts } from '../model/use-sync-v2-conflicts.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const props = defineProps<{ model: SyncV2Conflicts }>();
const { t } = useI18n();
const { v2Conflicts, v2ConflictReview, v2ConflictBusy, v2ConflictError, v2ConflictResolved, reviewV2Conflict, resolveV2Conflict, v2ConflictValueText } = props.model;
</script>

<template>
  <BaseAlert
    v-if="v2ConflictError"
    tone="warning"
    :title="v2ConflictError"
  />
  <p
    v-if="v2ConflictResolved"
    role="status"
  >
    {{ t('sync.v2.conflictResolved') }}
  </p>
  <section
    v-if="v2Conflicts.length"
    :aria-label="t('sync.v2.conflictTitle')"
  >
    <h3>{{ t('sync.v2.conflictTitle') }}</h3>
    <p>{{ t('sync.v2.conflictDescription') }}</p>
    <ul>
      <li
        v-for="target in v2Conflicts"
        :key="`${target.collection}/${target.key}`"
      >
        {{ target.collection }}/{{ target.key }}
        <BaseButton
          :disabled="v2ConflictBusy"
          @click="reviewV2Conflict(target.collection, target.key)"
        >
          {{ t('sync.review') }}
        </BaseButton>
      </li>
    </ul>
    <div v-if="v2ConflictReview">
      <h4>{{ v2ConflictReview.collection }}/{{ v2ConflictReview.key }}</h4>
      <p>{{ t('sync.base') }}: {{ v2ConflictValueText(v2ConflictReview.base) }}</p>
      <p>{{ t('sync.local') }}: {{ v2ConflictValueText(v2ConflictReview.local) }}</p>
      <p>{{ t('sync.cloud') }}: {{ v2ConflictValueText(v2ConflictReview.cloud) }}</p>
      <p v-if="v2ConflictReview.hasNewer">
        {{ t('sync.v2.conflictNewer') }}
      </p>
      <BaseButton
        :disabled="v2ConflictBusy"
        @click="resolveV2Conflict('keepLocal')"
      >
        {{ t('sync.keepMine') }}
      </BaseButton>
      <BaseButton
        :disabled="v2ConflictBusy"
        @click="resolveV2Conflict('takeCloud')"
      >
        {{ t('sync.useCloud') }}
      </BaseButton>
    </div>
  </section>
</template>
