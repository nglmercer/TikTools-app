<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseDisclosure from '@/shared/ui/disclosure/BaseDisclosure.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import type { OptionDraft } from '../lib/competition-draft.ts';
import FormField from './FormField.vue';

defineProps<{ actions: { id: string; name: string }[]; removable: boolean; withAction: boolean }>();
const option = defineModel<OptionDraft>({ required: true });
const emit = defineEmits<{ remove: [] }>();
const { t } = useI18n();

function changeAction(): void {
  option.value.inputs = [];
  option.value.originalAction = null;
}
</script>

<template>
  <fieldset class="option-row">
    <div class="option-row__main">
      <FormField
        class="option-row__id"
        :label="t('competitionsPanel.form.optionId')"
      >
        <input
          v-model="option.id"
          required
          maxlength="128"
          placeholder="a"
        >
      </FormField>
      <FormField :label="t('competitionsPanel.form.label')">
        <input
          v-model="option.label"
          required
          maxlength="256"
        >
      </FormField>
      <FormField :label="t('competitionsPanel.form.commands')">
        <input
          v-model="option.command"
          placeholder="!vote"
          maxlength="128"
        >
      </FormField>
      <BaseIconButton
        v-if="removable"
        :label="t('competitionsPanel.form.removeOption')"
        size="sm"
        @click="emit('remove')"
      >
        <IconTrash />
      </BaseIconButton>
    </div>
    <BaseDisclosure :title="t('competitionsPanel.form.more')">
      <div class="option-row__more">
        <FormField :label="t('competitionsPanel.form.imageUrl')">
          <input
            v-model="option.imageUrl"
            maxlength="2048"
            type="url"
          >
        </FormField>
        <FormField :label="t('competitionsPanel.form.giftIds')">
          <input
            v-model="option.giftIds"
            placeholder="123, 456"
          >
        </FormField>
        <FormField
          v-if="withAction"
          :label="t('competitionsPanel.form.winnerAction')"
        >
          <select
            v-model="option.actionId"
            @change="changeAction"
          >
            <option value="">
              {{ t('competitionsPanel.form.noAction') }}
            </option>
            <option
              v-for="available in actions"
              :key="available.id"
              :value="available.id"
            >
              {{ available.name || available.id }}
            </option>
          </select>
        </FormField>
      </div>
      <template v-if="withAction && option.actionId">
        <div
          v-for="(input, index) in option.inputs"
          :key="index"
          class="action-input"
        >
          <FormField :label="t('competitionsPanel.form.inputName')">
            <input
              v-model="input.key"
              maxlength="128"
            >
          </FormField>
          <FormField :label="t('competitionsPanel.form.valueType')">
            <select v-model="input.kind">
              <option value="string">
                {{ t('competitionsPanel.form.types.string') }}
              </option>
              <option value="number">
                {{ t('competitionsPanel.form.types.number') }}
              </option>
              <option value="boolean">
                {{ t('competitionsPanel.form.types.boolean') }}
              </option>
            </select>
          </FormField>
          <FormField :label="t('competitionsPanel.form.value')">
            <input
              v-if="input.kind !== 'boolean'"
              v-model="input.value"
            >
            <select
              v-else
              v-model="input.value"
            >
              <option value="true">
                {{ t('competitionsPanel.form.true') }}
              </option>
              <option value="false">
                {{ t('competitionsPanel.form.false') }}
              </option>
            </select>
          </FormField>
          <BaseIconButton
            :label="t('competitionsPanel.form.removeInput')"
            size="sm"
            @click="option.inputs.splice(index, 1)"
          >
            <IconTrash />
          </BaseIconButton>
        </div>
        <BaseButton
          size="sm"
          @click="option.inputs.push({ key: '', value: '', kind: 'string' })"
        >
          {{ t('competitionsPanel.form.addInput') }}
        </BaseButton>
      </template>
    </BaseDisclosure>
  </fieldset>
</template>

<style scoped>
.option-row { display: grid; gap: var(--space-1); margin: 0; padding: var(--space-2) var(--space-3); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
.option-row__main, .option-row__more, .action-input { display: flex; flex-wrap: wrap; align-items: end; gap: var(--space-2); }
.option-row__id { flex: 0 1 90px; min-width: 70px; }
.option-row__more { margin-bottom: var(--space-2); }
.action-input { margin-bottom: var(--space-2); }
</style>
