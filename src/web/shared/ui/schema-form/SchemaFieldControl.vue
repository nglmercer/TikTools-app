<script setup lang="ts">
import type { JsonValue } from '../../../../automation/types.ts';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseColorField from '@/shared/ui/color-field/BaseColorField.vue';
import BaseDateField from '@/shared/ui/date-field/BaseDateField.vue';
import BaseKeyValueEditor from '@/shared/ui/key-value/BaseKeyValueEditor.vue';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseTagsInput from '@/shared/ui/tags-input/BaseTagsInput.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { numericValue, stringList, stringRecord } from './lib/schema-fields.ts';
import type { SchemaFieldControlProps } from './lib/schema-control-props.ts';
import { useSchemaFieldInput } from './lib/use-schema-field-input.ts';
import SchemaOptionControl from './SchemaOptionControl.vue';
import SchemaMediaControl from './SchemaMediaControl.vue';
import SchemaTextControl from './SchemaTextControl.vue';
const props = defineProps<SchemaFieldControlProps>();
const emit = defineEmits<{ update: [value: JsonValue] }>();
const input = useSchemaFieldInput(props, (value) => emit('update', value));
const { text } = input;
</script>

<template>
  <component
    :is="switches ? BaseSwitch : BaseCheckbox"
    v-if="field.kind === 'boolean'"
    :id="controlId"
    :model-value="value === true || value === 'true'"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  >
    {{ field.label }}
  </component>

  <SchemaOptionControl
    v-else-if="field.kind === 'gift' || field.kind === 'search' || field.kind === 'select'"
    :field="field"
    :value="value"
    :control-id="controlId"
    :invalid="invalid"
    :disabled="disabled"
    :icon="icon"
    @update="emit('update', $event)"
  />

  <BaseKeyValueEditor
    v-else-if="field.kind === 'keyvalue'"
    :model-value="stringRecord(value)"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

  <BaseDateField
    v-else-if="field.kind === 'date' || field.kind === 'time'"
    :id="controlId"
    :type="field.kind"
    :model-value="text"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

  <BaseColorField
    v-else-if="field.kind === 'color'"
    :id="controlId"
    :model-value="text || '#000000'"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

  <BaseTextField
    v-else-if="field.kind === 'secret'"
    :id="controlId"
    :model-value="text"
    type="password"
    autocomplete="current-password"
    revealable
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
  </BaseTextField>

  <span
    v-else-if="field.kind === 'range'"
    class="schema-control__range"
  >
    <BaseSlider
      :id="controlId"
      :model-value="numericValue(value, field)"
      :min="field.min ?? 0"
      :max="field.max ?? 100"
      :step="field.step ?? 1"
      :disabled="disabled"
      @update:model-value="emit('update', $event)"
    />
    <span class="schema-control__range-value">{{ numericValue(value, field) }}</span>
  </span>

  <BaseSpinBox
    v-else-if="field.kind === 'number' || field.kind === 'rating'"
    :id="controlId"
    :model-value="numericValue(value, field)"
    :min="field.kind === 'rating' ? 0 : field.min"
    :max="field.kind === 'rating' ? (field.max ?? 5) : field.max"
    :step="field.step ?? 1"
    :orientation="compact ? 'horizontal' : undefined"
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
  </BaseSpinBox>

  <span
    v-else-if="field.kind === 'multiselect'"
    :id="controlId"
    class="schema-control__choices"
    role="group"
  >
    <component
      :is="switches ? BaseSwitch : BaseCheckbox"
      v-for="option in field.options"
      :key="option.value"
      :model-value="stringList(value).includes(option.value)"
      :disabled="disabled"
      @update:model-value="
        (checked: boolean) =>
          emit(
            'update',
            checked ? [...stringList(value), option.value] : stringList(value).filter((entry) => entry !== option.value),
          )
      "
    >
      {{ option.label }}
    </component>
  </span>

  <BaseTagsInput
    v-else-if="field.kind === 'tags'"
    :id="controlId"
    :model-value="stringList(value)"
    :placeholder="field.placeholder"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

  <SchemaMediaControl
    v-else-if="field.kind === 'media'"
    :field="field"
    :value="value"
    :control-id="controlId"
    :invalid="invalid"
    :disabled="disabled"
    :picker-error="input.pickerError.value"
    :can-pick="Boolean(openMediaPicker)"
    @update-path="input.updateMediaPath"
    @pick="input.pickMedia"
  />
  <SchemaTextControl
    v-else
    :field="field"
    :control-id="controlId"
    :invalid="invalid"
    :disabled="disabled"
    :icon="icon"
    :text="text"
    :variable-items="input.variableItems.value"
    :show-variables="input.showVariables.value"
    :variables-open="input.variablesOpen.value"
    :anchor="input.variablesAnchor"
    @update="emit('update', $event)"
    @variables-open="input.variablesOpen.value = $event"
    @variable="input.insertVariable"
  />
</template>

<style src="./schema-control.css"></style>
