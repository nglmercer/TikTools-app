<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonValue } from '../../../../automation/types.ts';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import { textValue } from './lib/schema-fields.ts';
import type { SchemaFieldControlProps } from './lib/schema-control-props.ts';
const props = defineProps<Pick<SchemaFieldControlProps, 'field' | 'value' | 'controlId' | 'invalid' | 'disabled' | 'icon'>>();
const emit = defineEmits<{ update: [value: JsonValue] }>();
const { t, n } = useI18n();
const text = computed(() => textValue(props.value));
const comboOptions = computed(() => props.field.options.map(({ value, label, disabled }) => ({ value, label, disabled })));
const searchOptions = computed(() =>
  props.field.options.map((option) => ({
    value: option.value,
    label: option.label,
    disabled: option.disabled,
    iconUrl: option.meta?.iconUrl,
    detail:
      typeof option.meta?.diamondCount === 'number'
        ? t('schemaForm.diamonds', { count: n(option.meta.diamondCount) })
        : undefined,
  })),
);
</script>

<template>
  <BaseSearchSelect
    v-if="field.kind === 'gift' || field.kind === 'search'"
    :id="controlId"
    :model-value="text"
    :options="searchOptions"
    :placeholder="field.placeholder"
    :allow-custom="field.kind === 'gift'"
    :invalid="invalid"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  >
    <template
      v-if="icon"
      #prefix
    >
      <component
        :is="icon"
        :size="14"
      />
    </template>
  </BaseSearchSelect>

  <BaseComboBox
    v-else
    :id="controlId"
    :model-value="text || (typeof field.defaultValue === 'string' ? field.defaultValue : undefined)"
    :options="comboOptions"
    :placeholder="field.placeholder"
    :disabled="disabled"
    @update:model-value="emit('update', $event ?? '')"
  >
    <template
      v-if="icon"
      #prefix
    >
      <component
        :is="icon"
        :size="14"
      />
    </template>
  </BaseComboBox>
</template>
