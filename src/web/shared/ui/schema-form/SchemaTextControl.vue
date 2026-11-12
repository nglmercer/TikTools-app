<script setup lang="ts">
import type { Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonValue } from '../../../../automation/types.ts';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import { parseJsonField } from './lib/schema-fields.ts';
import type { SchemaFieldControlProps } from './lib/schema-control-props.ts';
defineProps<Pick<SchemaFieldControlProps, 'field' | 'controlId' | 'invalid' | 'disabled' | 'icon'> & {
  text: string;
  variableItems: MenuEntry[];
  showVariables: boolean;
  variablesOpen: boolean;
  anchor: Ref<HTMLElement | null>;
}>();
const emit = defineEmits<{ update: [value: JsonValue]; variablesOpen: [value: boolean]; variable: [action: MenuAction] }>();
const { t } = useI18n();
</script>

<template>
  <span
    class="schema-control__text"
  >
    <BaseTextArea
      v-if="field.kind === 'json' || field.kind === 'textarea'"
      :id="controlId"
      class="schema-control__grow"
      :model-value="text"
      :rows="field.monospace ? 8 : 5"
      :monospace="field.monospace"
      :max-length="field.maxLength"
      :placeholder="field.placeholder"
      :invalid="invalid"
      :disabled="disabled"
      @update:model-value="emit('update', field.kind === 'json' ? parseJsonField($event, field) : $event)"
    />
    <BaseTextField
      v-else
      :id="controlId"
      class="schema-control__grow"
      :model-value="text"
      :placeholder="field.placeholder"
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
      v-if="showVariables"
      :ref="anchor"
    >
      <BaseIconButton
        :label="t('schemaForm.insertVariable')"
        variant="default"
        :disabled="disabled"
        @click="emit('variablesOpen', !variablesOpen)"
      >
        <IconPlus />
      </BaseIconButton>
    </span>
    <BaseMenu
      v-if="showVariables"
      :open="variablesOpen"
      :items="variableItems"
      :anchor="anchor.value"
      placement="bottom-end"
      @update:open="emit('variablesOpen', $event)"
      @select="emit('variable', $event)"
    />
  </span>
</template>
