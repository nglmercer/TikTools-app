<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { mediaPath } from './lib/schema-fields.ts';
import type { SchemaFieldControlProps } from './lib/schema-control-props.ts';
defineProps<Pick<SchemaFieldControlProps, 'field' | 'value' | 'controlId' | 'invalid' | 'disabled'> & { pickerError: string; canPick: boolean }>();
const emit = defineEmits<{ updatePath: [path: string]; pick: [] }>();
const { t } = useI18n();
</script>

<template>
  <span
    class="schema-control__media"
  >
    <BaseTextField
      :id="controlId"
      class="schema-control__grow"
      :model-value="mediaPath(value)"
      :placeholder="field.placeholder"
      :invalid="invalid || Boolean(pickerError)"
      :disabled="disabled"
      @update:model-value="emit('updatePath', $event)"
    />
    <BaseButton
      v-if="canPick"
      :disabled="disabled"
      @click="emit('pick')"
    >
      {{ mediaPath(value) ? t('schemaForm.changeMedia') : t('schemaForm.browseMedia') }}
    </BaseButton>
    <span
      v-if="pickerError"
      class="schema-control__error"
      role="alert"
    >{{ pickerError }}</span>
  </span>
</template>
