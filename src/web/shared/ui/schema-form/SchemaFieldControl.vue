<script setup lang="ts">
import { computed, markRaw, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonValue } from '../../../../automation/types.ts';
import type { MediaSelection, OpenMediaPicker } from '../../../../shared/messages.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseColorField from '@/shared/ui/color-field/BaseColorField.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseDateField from '@/shared/ui/date-field/BaseDateField.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import BaseKeyValueEditor from '@/shared/ui/key-value/BaseKeyValueEditor.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseTagsInput from '@/shared/ui/tags-input/BaseTagsInput.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import {
  mediaPath,
  numericValue,
  parseJsonField,
  stringList,
  stringRecord,
  textValue,
  type SchemaFieldDescriptor,
} from './lib/schema-fields.ts';
import type { SchemaSuggestion } from './lib/schema-suggestion.ts';

const AUDIO_EXTENSIONS = ['aac', 'flac', 'm4a', 'mp3', 'mp4', 'oga', 'ogg', 'opus', 'wav', 'webm'];

const props = defineProps<{
  field: SchemaFieldDescriptor;
  value: JsonValue | undefined;
  controlId: string;
  invalid?: boolean;
  disabled?: boolean;
  switches?: boolean;
  suggestions?: readonly SchemaSuggestion[];
  openMediaPicker?: OpenMediaPicker;
}>();

const emit = defineEmits<{ update: [value: JsonValue] }>();

const { t, n } = useI18n();
const pickerError = ref('');
const variablesAnchor = useTemplateRef('variablesAnchor');
const variablesOpen = ref(false);

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
const variableItems = computed<MenuEntry[]>(() =>
  (props.suggestions ?? []).map((suggestion) => ({
    id: suggestion.value,
    label: suggestion.label,
    shortcut: suggestion.detail,
    icon: markRaw(IconFlow),
  })),
);
const showVariables = computed(() => props.field.template && variableItems.value.length > 0);

function insertVariable(action: MenuAction): void {
  emit('update', `${text.value}{{ ${action.id} }}`);
}

function pickMedia(): void {
  const media = props.field.media;
  if (!media || !props.openMediaPicker) return;
  pickerError.value = '';
  props.openMediaPicker(
    {
      mode: media.mode,
      kind: media.kind,
      extensions: media.extensions ?? (media.kind === 'audio' ? AUDIO_EXTENSIONS : undefined),
      title: props.field.label,
    },
    (selection: MediaSelection | null, error?: string) => {
      if (error) pickerError.value = error;
      else if (selection?.type === 'file') emit('update', selection.file as unknown as JsonValue);
      else if (selection?.type === 'directory') emit('update', selection.directory as unknown as JsonValue);
    },
  );
}
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

  <BaseSearchSelect
    v-else-if="field.kind === 'gift' || field.kind === 'search'"
    :id="controlId"
    :model-value="text"
    :options="searchOptions"
    :placeholder="field.placeholder"
    :allow-custom="field.kind === 'gift'"
    :invalid="invalid"
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

  <BaseComboBox
    v-else-if="field.kind === 'select'"
    :id="controlId"
    :model-value="text || (typeof field.defaultValue === 'string' ? field.defaultValue : undefined)"
    :options="comboOptions"
    :placeholder="field.placeholder"
    :disabled="disabled"
    @update:model-value="emit('update', $event ?? '')"
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
  />

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
    :disabled="disabled"
    @update:model-value="emit('update', $event)"
  />

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

  <span
    v-else-if="field.kind === 'media'"
    class="schema-control__media"
  >
    <BaseTextField
      :id="controlId"
      class="schema-control__grow"
      :model-value="mediaPath(value)"
      :placeholder="field.placeholder"
      :invalid="invalid || Boolean(pickerError)"
      :disabled="disabled"
      @update:model-value="(path) => { pickerError = ''; emit('update', path); }"
    />
    <BaseButton
      v-if="openMediaPicker"
      :disabled="disabled"
      @click="pickMedia"
    >
      {{ mediaPath(value) ? t('schemaForm.changeMedia') : t('schemaForm.browseMedia') }}
    </BaseButton>
    <span
      v-if="pickerError"
      class="schema-control__error"
      role="alert"
    >{{ pickerError }}</span>
  </span>

  <span
    v-else
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
    />
    <span
      v-if="showVariables"
      ref="variablesAnchor"
    >
      <BaseIconButton
        :label="t('schemaForm.insertVariable')"
        variant="default"
        :disabled="disabled"
        @click="variablesOpen = !variablesOpen"
      >
        <IconPlus />
      </BaseIconButton>
    </span>
    <BaseMenu
      v-if="showVariables"
      v-model:open="variablesOpen"
      :items="variableItems"
      :anchor="variablesAnchor"
      placement="bottom-end"
      @select="insertVariable"
    />
  </span>
</template>

<style scoped>
.schema-control__range,
.schema-control__media,
.schema-control__text {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: var(--space-3);
  min-width: 0;
}

.schema-control__range {
  align-items: center;
  flex-wrap: nowrap;
}

.schema-control__range-value {
  min-width: 4ch;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.schema-control__grow {
  flex: 1;
  min-width: 0;
}

.schema-control__choices {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-5);
}

.schema-control__error {
  flex-basis: 100%;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}
</style>
