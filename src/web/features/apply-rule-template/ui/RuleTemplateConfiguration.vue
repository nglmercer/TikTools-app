<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { ActionOptionItem } from '../../../../shared/messages.ts';
import type { RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { fieldOptionsFor, optionContext, optionContextSignature } from '@/entities/plugin/lib/plugin-options.ts';
import { normalizeOptionsFrom, optionFields } from '../../../../automation/plugins/declarative.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
const props = withDefaults(
  defineProps<{
    selected: RuleTemplate;
    fieldId: string;
    error: string;
    actionOptions?: Readonly<Record<string, readonly ActionOptionItem[]>>;
    loadOptions?: (source: string, context?: JsonObject) => void;
  }>(),
  { actionOptions: () => ({}), loadOptions: undefined },
);
const eventName = defineModel<string>('eventName', { required: true });
const actionNames = defineModel<string[]>('actionNames', { required: true });
const params = defineModel<JsonObject>('params', { required: true });
const { t } = useI18n();

/** Template params may point at a plugin option source (`optionsFrom` on the
 * property schema) so they render the same live selects as the action
 * editor instead of asking the user to retype plugin-managed values. */
const paramUiHints = computed<JsonObject>(() => {
  const properties = props.selected.params?.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return {};
  const fields: JsonObject = {};
  for (const [key, property] of Object.entries(properties)) {
    if (!property || typeof property !== 'object' || Array.isArray(property)) continue;
    const source = normalizeOptionsFrom((property as JsonObject).optionsFrom);
    if (source) fields[key] = { optionsFrom: source };
  }
  return { fields };
});
const paramFieldOptions = computed(() => fieldOptionsFor(paramUiHints.value, props.actionOptions));
const paramSources = computed(() => [...new Set(optionFields(paramUiHints.value).map((field) => field.source))]);
// Params double as the option context: `optionSources.*.params` templates
// like `{{ config.provider }}` resolve against them, so sibling param edits
// refetch dependent selects (debounced; the host cache dedupes repeats).
const paramContext = computed(() => optionContext(params.value));
const paramContextSig = computed(() => optionContextSignature(paramContext.value));
let optionsTimer: ReturnType<typeof setTimeout> | undefined;
watch([paramSources, paramContextSig], () => {
  if (optionsTimer) clearTimeout(optionsTimer);
  optionsTimer = setTimeout(() => {
    for (const source of paramSources.value) props.loadOptions?.(source, paramContext.value);
  }, 300);
}, { immediate: true });
onBeforeUnmount(() => {
  if (optionsTimer) clearTimeout(optionsTimer);
});
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
      :ui-hints="paramUiHints"
      :field-options="paramFieldOptions"
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
