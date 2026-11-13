<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PointsConfig } from '../../../../shared/messages.ts';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import GuideButton from '@/features/guided-help/ui/GuideButton.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import { isConfigDirty, MIN_POINTS_PER_LEVEL, normalizePointsConfig, pointsRules } from '../model/points-rules.ts';

const { t } = useI18n();
const store = usePointsStore();
const { pointsConfig } = storeToRefs(store);
const draft = ref<PointsConfig>({ ...pointsConfig.value });
const saving = ref(false);
const saved = ref(false);

const dirty = computed(() => isConfigDirty(draft.value, pointsConfig.value));
const editable = computed(() => !saving.value);

watch(pointsConfig, (config, previous) => {
  if (!isConfigDirty(draft.value, previous)) draft.value = { ...config };
});

function discard(): void {
  draft.value = { ...pointsConfig.value };
}

async function save(): Promise<void> {
  if (!editable.value || !dirty.value) return;
  saving.value = true;
  try {
    await store.updateConfig(normalizePointsConfig(draft.value));
    draft.value = { ...pointsConfig.value };
    saved.value = true;
    setTimeout(() => (saved.value = false), 2500);
  } catch {
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <BasePanel
    class="points-rules"
    :title="t('points.rules.title')"
  >
    <template #actions>
      <GuideButton
        tour="points"
        :task-state="() => ({ 'points.reviewed': saved })"
      />
    </template>
    <form
      class="points-rules__form"
      @submit.prevent="save"
    >
      <BaseFormGrid>
        <BaseFormRow
          for="points-currency"
          :label="t('points.rules.currency')"
        >
          <BaseTextField
            id="points-currency"
            v-model="draft.currencyName"
            :disabled="!editable"
          />
        </BaseFormRow>
      </BaseFormGrid>

      <table class="points-rules__table">
        <thead>
          <tr>
            <th scope="col">
              {{ t('points.rules.source') }}
            </th>
            <th scope="col">
              {{ t('points.rules.rate', { currency: draft.currencyName }) }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="rule in pointsRules"
            :key="rule.id"
            :class="{ 'points-rules__row--off': !draft[rule.enabled] }"
          >
            <td>
              <BaseCheckbox
                v-model="draft[rule.enabled]"
                v-tooltip="t(`points.rules.sources.${rule.id}.hint`)"
                :disabled="!editable"
              >
                {{ t(`points.rules.sources.${rule.id}.label`) }}
              </BaseCheckbox>
            </td>
            <td>
              <BaseSpinBox
                v-model="draft[rule.rate]"
                class="points-rules__rate"
                :min="0"
                :step="rule.step"
                :precision="2"
                :aria-label="t(`points.rules.sources.${rule.id}.label`)"
                :disabled="!editable || !draft[rule.enabled]"
              />
            </td>
          </tr>
        </tbody>
      </table>

      <BaseFormGrid>
        <BaseFormRow
          for="points-sub-bonus"
          :label="t('points.rules.subBonus')"
          :hint="t('points.rules.subBonusHint')"
        >
          <BaseSpinBox
            id="points-sub-bonus"
            v-model="draft.subBonusMultiplier"
            :min="0"
            :max="500"
            :step="5"
            suffix="%"
            :disabled="!editable"
          />
        </BaseFormRow>
        <BaseFormRow
          for="points-per-level"
          :label="t('points.rules.perLevel')"
          :hint="t('points.rules.perLevelHint')"
        >
          <BaseSpinBox
            id="points-per-level"
            v-model="draft.pointsPerLevel"
            :min="MIN_POINTS_PER_LEVEL"
            :step="10"
            :disabled="!editable"
          />
        </BaseFormRow>
      </BaseFormGrid>
    </form>
    <template #footer>
      <span
        v-if="saved"
        class="points-rules__saved"
        role="status"
      >{{ t('points.rules.saved') }}</span>
      <span class="points-rules__spacer" />
      <BaseButton
        :disabled="!dirty || !editable"
        @click="discard"
      >
        {{ t('points.rules.discard') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="!dirty || !editable"
        @click="save"
      >
        {{ saving ? t('points.rules.saving') : t('points.rules.save') }}
      </BaseButton>
    </template>
  </BasePanel>
</template>

<style scoped>
.points-rules__form {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.points-rules__table {
  width: 100%;
  border: var(--border-width) solid var(--color-border-subtle);
  border-collapse: collapse;
}

.points-rules__table th {
  padding: var(--space-2) var(--space-4);
  border-bottom: var(--border-width) solid var(--color-border);
  background: var(--color-bg-panel-header);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  text-align: left;
}

.points-rules__table td {
  padding: var(--space-2) var(--space-4);
}

.points-rules__table tr + tr td {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.points-rules__row--off td:first-child {
  color: var(--color-text-muted);
}

.points-rules__rate {
  width: 120px;
}

.points-rules__saved {
  color: var(--color-success);
  font-size: var(--font-size-sm);
}

.points-rules__spacer {
  flex: 1;
}
</style>
