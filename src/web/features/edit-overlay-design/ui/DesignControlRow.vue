<script setup lang="ts">
import { computed, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WidgetAlign, WidgetStyle } from '../../../../widgets/sdk/template.ts';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseColorField from '@/shared/ui/color-field/BaseColorField.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';
import type { DesignDefaults, ScalarControl } from '../lib/design-controls.ts';

const props = withDefaults(
  defineProps<{
    control: ScalarControl;
    design: WidgetStyle;
    defaults: DesignDefaults;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{ update: [value: string | number | boolean] }>();

const { t } = useI18n();
const id = useId();
const value = computed(() => props.control.get(props.design, props.defaults));
const controlDisabled = computed(
  () => props.disabled || (props.control.kind === 'range' && props.control.disabled?.(props.design) === true),
);
const alignOptions = computed(() =>
  (['left', 'center', 'right'] as const).map((option) => ({ value: option, label: t(`overlays.editor.align.${option}`) })),
);
const choiceOptions = computed(() =>
  props.control.kind === 'choice'
    ? props.control.options.map((option) => ({ value: option, label: t(`overlays.editor.${props.control.kind === 'choice' ? props.control.labelKey : ''}.${option}`) }))
    : [],
);
const unitLabel = computed(() => (props.control.kind === 'range' ? props.control.unit : ''));
</script>

<template>
  <BaseFormRow
    :for="id"
    :label="control.label"
  >
    <BaseColorField
      v-if="control.kind === 'color'"
      :id="id"
      :model-value="String(value)"
      :disabled="controlDisabled"
      @update:model-value="emit('update', $event)"
    />
    <span
      v-else-if="control.kind === 'range'"
      class="design-control__range"
    >
      <BaseSlider
        :id="id"
        :model-value="Number(value)"
        :min="control.min"
        :max="control.max"
        :step="control.step"
        :disabled="controlDisabled"
        @update:model-value="emit('update', $event)"
      />
      <output
        class="design-control__value"
        :for="id"
      >{{ value }}{{ unitLabel ? ` ${unitLabel}` : '' }}</output>
    </span>
    <BaseCheckbox
      v-else-if="control.kind === 'toggle'"
      :id="id"
      :model-value="Boolean(value)"
      :disabled="controlDisabled"
      @update:model-value="emit('update', $event)"
    />
    <BaseSegmented
      v-else-if="control.kind === 'choice'"
      :id="id"
      :model-value="String(value)"
      :options="choiceOptions"
      size="sm"
      :disabled="controlDisabled"
      @update:model-value="emit('update', $event)"
    />
    <BaseSegmented
      v-else
      :id="id"
      :model-value="value as WidgetAlign"
      :options="alignOptions"
      size="sm"
      :disabled="controlDisabled"
      @update:model-value="emit('update', $event)"
    />
  </BaseFormRow>
</template>

<style scoped>
.design-control__range {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 0;
}

.design-control__range > :first-child {
  flex: 1;
}

.design-control__value {
  min-width: 48px;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  text-align: right;
}
</style>
