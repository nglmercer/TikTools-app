<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { RankingSource } from '@/entities/ranking/types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDisclosure from '@/shared/ui/disclosure/BaseDisclosure.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import { metricLabel } from '../lib/metric-labels.ts';
import FormField from './FormField.vue';

export interface GiftRule { giftId: string; multiplier: number }

const source = defineModel<RankingSource>('source', { required: true });
const overrides = defineModel<GiftRule[]>('overrides', { default: () => [] });
const emit = defineEmits<{ remove: [] }>();
const { t } = useI18n();
</script>

<template>
  <li class="source-row">
    <div class="source-row__main">
      <BaseCheckbox v-model="source.enabled">
        {{ metricLabel(t, source.source, source.metric) }}
      </BaseCheckbox>
      <label class="source-row__weight">×<input
        v-model.number="source.weight"
        :aria-label="t('rankingsPage.form.weight')"
        type="number"
        min="0"
        max="1000000000"
      ></label>
      <BaseIconButton
        :label="t('rankingsPage.form.remove')"
        size="sm"
        @click="emit('remove')"
      >
        <IconTrash />
      </BaseIconButton>
    </div>
    <BaseDisclosure
      v-if="source.source === 'tiktok.gift'"
      :title="t('rankingsPage.form.giftMultipliers')"
      :count="overrides.length"
    >
      <div
        v-for="(rule, index) in overrides"
        :key="index"
        class="gift-rule"
      >
        <FormField :label="t('rankingsPage.form.giftId')">
          <input
            v-model="rule.giftId"
            maxlength="128"
          >
        </FormField>
        <FormField :label="t('rankingsPage.form.multiplier')">
          <input
            v-model.number="rule.multiplier"
            type="number"
            min="0"
            max="1000000000"
            step="1"
          >
        </FormField>
        <BaseIconButton
          :label="t('rankingsPage.form.removeOverride')"
          size="sm"
          @click="overrides.splice(index, 1)"
        >
          <IconTrash />
        </BaseIconButton>
      </div>
      <BaseButton
        size="sm"
        :disabled="overrides.length >= 512"
        @click="overrides = [...overrides, { giftId: '', multiplier: 1 }]"
      >
        {{ t('rankingsPage.form.addGiftOverride') }}
      </BaseButton>
    </BaseDisclosure>
  </li>
</template>

<style scoped>
.source-row { display: grid; gap: var(--space-1); padding: var(--space-2) var(--space-3); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
.source-row__main { display: flex; align-items: center; gap: var(--space-3); }
.source-row__main > :first-child { flex: 1; font-weight: 600; }
.source-row__weight { display: inline-flex; align-items: center; gap: var(--space-1); color: var(--color-text-muted); }
.source-row__weight input { width: 80px; min-height: 28px; padding: 2px 6px; color: var(--color-text); background: var(--color-bg-control); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
.gift-rule { display: flex; align-items: end; gap: var(--space-2); margin-bottom: var(--space-2); }
</style>
