<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
defineProps<{ selected: RuleTemplate; fieldId: string; error: string }>();
const eventName = defineModel<string>('eventName', { required: true });
const actionNames = defineModel<string[]>('actionNames', { required: true });
const params = defineModel<JsonObject>('params', { required: true });
const { t } = useI18n();
</script>

<template>
  <p class="rule-templates__lead">
    {{ localizedText(selected.description) }}
  </p>
  <BaseFormGrid>
    <BaseFormRow
      :for="`${fieldId}-event`"
      :label="t('automations.templates.eventName')"
    >
      <BaseTextField
        :id="`${fieldId}-event`"
        v-model="eventName"
      />
    </BaseFormRow>
    <BaseFormRow
      v-for="(action, index) in selected.actions"
      :key="`${action.typeId}-${index}`"
      :for="`${fieldId}-action-${index}`"
      :label="selected.actions.length > 1 ? t('automations.templates.actionNameOf', { name: action.typeId }) : t('automations.templates.actionName')"
    >
      <BaseTextField
        :id="`${fieldId}-action-${index}`"
        :model-value="actionNames[index]"
        @update:model-value="actionNames = actionNames.map((name, i) => i === index ? $event : name)"
      />
    </BaseFormRow>
  </BaseFormGrid>
  <BaseGroupBox
    v-if="selected.params"
    class="rule-templates__params"
    :title="t('automations.templates.options')"
  >
    <BaseSchemaForm
      v-model="params"
      :schema="selected.params"
    />
  </BaseGroupBox>
  <p
    v-if="error"
    class="rule-templates__error"
    role="alert"
  >
    {{ error }}
  </p>
  <p class="rule-templates__muted">
    {{ t('automations.templates.reviewHint') }}
  </p>
</template>
