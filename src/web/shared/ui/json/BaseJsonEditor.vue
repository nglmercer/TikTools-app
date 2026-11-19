<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import BaseJsonView from './BaseJsonView.vue';
import { useJsonCopy } from './use-json-copy.ts';

/**
 * JSON editing surface: a plain text area plus a live-validated preview built
 * from the existing BaseJsonView. Keeps the raw text as the source of truth so
 * comments/formatting are never rewritten behind the user's back.
 */
const props = withDefaults(defineProps<{
  label?: string;
  rows?: number;
  maxChars?: number;
  placeholder?: string;
}>(), { rows: 10, maxChars: 200_000, label: undefined, placeholder: undefined });

const text = defineModel<string>({ default: '' });
const emit = defineEmits<{
  validation: [result: { value?: Record<string, unknown>; error: string }];
}>();
const { t } = useI18n();

const mode = ref<'edit' | 'preview'>('edit');
const parsed = computed<Record<string, unknown> | undefined>(() => {
  try {
    const value: unknown = JSON.parse(text.value);
    return value && typeof value === 'object' && !Array.isArray(value)
      ? value as Record<string, unknown>
      : undefined;
  } catch {
    return undefined;
  }
});
const error = computed(() => {
  if (!text.value.trim()) return t('jsonView.editor.empty');
  if (text.value.length > props.maxChars) return t('jsonView.editor.tooLarge');
  if (!parsed.value) return t('jsonView.editor.objectRequired');
  return '';
});

watch(
  [text, error],
  () => emit('validation', { value: parsed.value, error: error.value }),
  { immediate: true },
);

const { copied, copy } = useJsonCopy(() => JSON.stringify(parsed.value ?? {}, null, 2), () => {});
const tr = (key: string): string => String(t(`jsonView.editor.${key}`));
</script>

<template>
  <div class="base-json-editor">
    <div class="base-json-editor__toolbar">
      <BaseButton
        size="sm"
        :variant="mode === 'edit' ? 'primary' : 'default'"
        @click="mode = 'edit'"
      >
        {{ tr('edit') }}
      </BaseButton>
      <BaseButton
        size="sm"
        :variant="mode === 'preview' ? 'primary' : 'default'"
        :disabled="Boolean(error)"
        @click="mode = 'preview'"
      >
        {{ tr('preview') }}
      </BaseButton>
      <BaseIconButton
        :label="copied ? t('jsonView.copied') : t('jsonView.copy')"
        size="sm"
        :disabled="Boolean(error)"
        @click="copy"
      >
        <IconCheck v-if="copied" />
        <IconCopy v-else />
      </BaseIconButton>
    </div>

    <BaseTextArea
      v-if="mode === 'edit'"
      v-model="text"
      :rows="rows"
      monospace
      :invalid="Boolean(error)"
      :aria-label="label ?? tr('label')"
      :placeholder="placeholder"
    />
    <BaseJsonView
      v-else
      :value="parsed"
      :label="label ?? tr('label')"
      max-height="40vh"
      line-numbers
      switchable
      :max-lines="500"
      :max-chars="200000"
    />

    <p
      v-if="error"
      class="base-json-editor__error"
      role="status"
    >
      {{ error }}
    </p>
  </div>
</template>

<style scoped>
.base-json-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.base-json-editor__toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.base-json-editor__error {
  margin: 0;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}
</style>
